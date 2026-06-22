#!/usr/bin/env bash
# backup-mail.sh — one-off, resumable export of the entire FastMail account to .eml files.
#
# Enumerates every mailbox, keys each by its stable JMAP id (so duplicate-named folders
# are all reachable), and downloads each message's raw RFC822 via `fm emails export`.
# Re-run any time: messages already on disk are skipped, so an interrupted 7GB run resumes
# where it left off. The .eml files are the source of truth; manifest.ndjson is convenience
# metadata for a future search index (which can dedupe by emailId and parse headers itself).
#
# Usage:
#   scripts/backup-mail.sh                       # full account into ./mail
#   scripts/backup-mail.sh --dest ~/fm-backup    # somewhere else
#   scripts/backup-mail.sh --only P8k            # one mailbox (id or name)
#   scripts/backup-mail.sh --only Crypto --max-per-mailbox 5   # capped smoke test
#   scripts/backup-mail.sh --refresh             # re-enumerate id lists (don't trust cache)
#
# See docs/guides/backup.md.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

DEST="${BACKUP_DIR:-$ROOT_DIR/mail}"
FM="${FM:-$ROOT_DIR/target/release/fm}"
ONLY=""
MAX_PER_MAILBOX=0
REFRESH=0

while [ $# -gt 0 ]; do
  case "$1" in
    --dest)            DEST="$2"; shift 2 ;;
    --fm)              FM="$2"; shift 2 ;;
    --only)            ONLY="$2"; shift 2 ;;
    --max-per-mailbox) MAX_PER_MAILBOX="$2"; shift 2 ;;
    --refresh)         REFRESH=1; shift ;;
    -h|--help)         sed -n '2,20p' "$0"; exit 0 ;;
    *)                 echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

# --- preflight -------------------------------------------------------------
die() { echo "error: $*" >&2; exit 1; }

command -v jq >/dev/null 2>&1 || die "jq is required (brew install jq)"

if command -v shasum >/dev/null 2>&1; then
  sha256_of() { shasum -a 256 "$1" | awk '{print $1}'; }
elif command -v sha256sum >/dev/null 2>&1; then
  sha256_of() { sha256sum "$1" | awk '{print $1}'; }
else
  die "need shasum or sha256sum"
fi

if [ ! -x "$FM" ]; then
  if [ "$FM" = "$ROOT_DIR/target/release/fm" ]; then
    echo "building fm (release)…"
    ( cd "$ROOT_DIR" && cargo build --release ) || die "cargo build failed"
  else
    die "fm binary not found / not executable: $FM"
  fi
fi

# Confirm a token actually resolves before doing anything.
"$FM" config --json | jq -e '.token != "(not set)"' >/dev/null 2>&1 \
  || die "no FastMail token resolves — run 'fm setup' or set FASTMAIL_API_TOKEN"

# --- dirs + logging --------------------------------------------------------
STATE_DIR="$DEST/.backup-state"
LOG_DIR="$DEST/logs"
mkdir -p "$STATE_DIR" "$LOG_DIR"

TS="$(date +%Y%m%dT%H%M%S)"
LOG="$LOG_DIR/backup-$TS.log"
FAILS="$LOG_DIR/failures.log"
MANIFEST="$DEST/manifest.ndjson"

# Tee everything to the run log so a broken run is traceable afterward.
exec > >(tee -a "$LOG") 2>&1

log() { printf '%s  %s\n' "$(date +%Y-%m-%dT%H:%M:%S)" "$*"; }

# Does the file look like an RFC822 message (a header line first), not empty/garbage?
looks_rfc822() {
  [ -s "$1" ] || return 1
  head -1 "$1" | LC_ALL=C grep -qE '^(From |[!-9;-~]+:)'
}

sanitize() {
  local s
  s="$(printf '%s' "$1" | tr '/ ' '__' | LC_ALL=C tr -cd '[:alnum:]_.-')"
  [ -n "$s" ] || s="mailbox"
  printf '%s' "$s"
}

log "backup starting → $DEST   (fm: $FM)"
[ -n "$ONLY" ] && log "restricted to mailbox: $ONLY"
[ "$MAX_PER_MAILBOX" -gt 0 ] && log "cap: $MAX_PER_MAILBOX message(s) per mailbox (smoke mode)"

# --- snapshot mailbox tree + a state cursor for later incremental top-ups --
"$FM" mailboxes list --json > "$STATE_DIR/mailboxes.json" \
  || die "could not list mailboxes"
mb_total="$(jq 'length' "$STATE_DIR/mailboxes.json")"
log "account has $mb_total mailbox(es)"

# Capture the Email state ONCE at the first run so a future `fm emails changes --since`
# can resume from the moment the backup began (don't overwrite on resume).
if [ ! -f "$STATE_DIR/state.json" ]; then
  "$FM" emails changes --json > "$STATE_DIR/state.json" 2>/dev/null \
    && log "saved start cursor: $(jq -r '.state' "$STATE_DIR/state.json")"
fi

# --- iterate mailboxes -----------------------------------------------------
mb_index=0

# id<TAB>name per mailbox, optionally filtered by --only (id or exact-ish name).
jq -r '.[] | [.id, .name] | @tsv' "$STATE_DIR/mailboxes.json" \
| while IFS="$(printf '\t')" read -r mb_id mb_name; do
    mb_index=$((mb_index + 1))

    if [ -n "$ONLY" ] && [ "$ONLY" != "$mb_id" ] && [ "$ONLY" != "$mb_name" ]; then
      continue
    fi

    safe="$(sanitize "$mb_name")"
    dir="$DEST/${safe}__${mb_id}"
    mkdir -p "$dir"

    ids_file="$STATE_DIR/${mb_id}.ids"
    if [ "$REFRESH" -eq 1 ] || [ ! -s "$ids_file" ]; then
      "$FM" emails list -m "$mb_id" --all --json 2>/dev/null \
        | jq -r '.[].id' > "$ids_file" \
        || { log "FAIL enumerate mailbox $mb_name ($mb_id)"; continue; }
    fi
    want="$(wc -l < "$ids_file" | tr -d ' ')"
    log "[$mb_index/$mb_total] $mb_name ($mb_id): $want message(s) → $dir"

    got=0
    n=0
    while IFS= read -r eid; do
      [ -n "$eid" ] || continue
      n=$((n + 1))
      if [ "$MAX_PER_MAILBOX" -gt 0 ] && [ "$n" -gt "$MAX_PER_MAILBOX" ]; then
        log "  cap reached ($MAX_PER_MAILBOX); skipping rest of $mb_name"
        break
      fi

      file="$dir/$eid.eml"
      if [ -s "$file" ]; then
        got=$((got + 1)); continue          # already have it → resume skip
      fi

      if "$FM" emails export "$eid" --to "$file" >/dev/null 2>&1 && looks_rfc822 "$file"; then
        bytes="$(wc -c < "$file" | tr -d ' ')"
        sha="$(sha256_of "$file")"
        jq -nc \
          --arg id "$eid" --arg mid "$mb_id" --arg mb "$mb_name" \
          --arg path "${safe}__${mb_id}/$eid.eml" \
          --argjson bytes "$bytes" --arg sha "$sha" \
          '{emailId:$id, mailboxId:$mid, mailbox:$mb, path:$path, bytes:$bytes, sha256:$sha}' \
          >> "$MANIFEST"
        got=$((got + 1))
      else
        rm -f "$file"                        # no partial file left behind
        log "  FAIL export $eid"
        echo "$(date +%Y-%m-%dT%H:%M:%S) $mb_id $eid" >> "$FAILS"
      fi
    done < "$ids_file"

    if [ "$MAX_PER_MAILBOX" -eq 0 ] && [ "$got" -lt "$want" ]; then
      log "  INCOMPLETE $mb_name: $got/$want on disk"
    else
      log "  ok $mb_name: $got/$want"
    fi
  done

# NOTE: the while-loop runs in a subshell (piped from jq), so per-mailbox state can't
# escape it. The final tally and exit code come from the filesystem — the authoritative
# state, and the right gate across resume runs regardless of what this run did.
on_disk="$(find "$DEST" -type f -name '*.eml' 2>/dev/null | wc -l | tr -d ' ')"
fail_count=0
[ -f "$FAILS" ] && fail_count="$(wc -l < "$FAILS" | tr -d ' ')"

log "----------------------------------------------------------------"
log "done: $on_disk .eml on disk under $DEST"
log "manifest: $MANIFEST"
[ "$fail_count" -gt 0 ] && log "failures logged: $fail_count (see $FAILS)"
log "run log: $LOG"

# Reconcile enumerated-vs-downloaded per mailbox; exit non-zero if any gap remains.
rc=0
for ids_file in "$STATE_DIR"/*.ids; do
  [ -e "$ids_file" ] || continue
  mb_id="$(basename "$ids_file" .ids)"
  want="$(wc -l < "$ids_file" | tr -d ' ')"
  mb_dir="$(find "$DEST" -type d -name "*__${mb_id}" 2>/dev/null | head -1)"
  have=0
  [ -n "$mb_dir" ] && have="$(find "$mb_dir" -name '*.eml' | wc -l | tr -d ' ')"
  if [ "$MAX_PER_MAILBOX" -eq 0 ] && [ "${have:-0}" -lt "${want:-0}" ]; then
    log "GAP mailbox $mb_id: $have/$want"
    rc=1
  fi
done

[ "$rc" -eq 0 ] && log "reconciliation: all enumerated mailboxes complete"
exit "$rc"
