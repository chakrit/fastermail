#!/usr/bin/env bash
# sort.sh — migrate an existing flat mail backup into a sha256-sharded tree.
#
# backup-mail.sh historically wrote every message of a mailbox into one flat directory
# (mail/Archive__P3V/<id>.eml) — 17k files in a single folder. This relocates each to
# mail/Archive__P3V/<sha256(id)[:2]>/<id>.eml (256 even buckets, see _shard.sh) and
# rewrites manifest.ndjson paths to match. Idempotent: already-sharded files are left
# alone, so re-running is a no-op.
#
# Usage:
#   scripts/sort.sh                 # shard ./mail in place
#   scripts/sort.sh --dest ~/fm     # a backup elsewhere
#   scripts/sort.sh --dry-run       # report what would move, touch nothing
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source-path=SCRIPTDIR
# shellcheck source=_shard.sh
source "$SCRIPT_DIR/_shard.sh"

DEST="${BACKUP_DIR:-$ROOT_DIR/mail}"
DRY=0
while [ $# -gt 0 ]; do
  case "$1" in
    --dest)    DEST="$2"; shift 2 ;;
    --dry-run) DRY=1; shift ;;
    -h|--help) sed -n '2,14p' "$0"; exit 0 ;;
    *)         echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

command -v jq >/dev/null 2>&1 || { echo "error: jq is required" >&2; exit 1; }
[ -d "$DEST" ] || { echo "error: no such dest: $DEST" >&2; exit 1; }

log() { printf '%s  %s\n' "$(date +%H:%M:%S)" "$*"; }

# "would shard" under --dry-run, "sharded" for a real run — chosen once, reused in logs.
if [ "$DRY" -eq 1 ]; then
  verb="would shard"
  log "DRY RUN — nothing will be moved"
else
  verb="sharded"
fi

# --- relocate flat .eml files into per-mailbox shard subdirs ----------------
# Mailbox dirs are named <sanitizedName>__<mailboxId>; -maxdepth 1 matches only the flat
# files, so anything already a level deep in a shard is skipped and re-runs are no-ops.
moved=0
for mb_dir in "$DEST"/*__*; do
  [ -d "$mb_dir" ] || continue

  n=0
  while IFS= read -r flat; do
    [ -n "$flat" ] || continue
    eid="$(basename "$flat" .eml)"
    dst="$mb_dir/$(shard_of "$eid")/$eid.eml"

    if [ "$DRY" -eq 1 ]; then
      [ "$n" -lt 2 ] && log "  e.g. ${flat#"$DEST"/} -> ${dst#"$DEST"/}"
    else
      mkdir -p "$(dirname "$dst")"
      mv "$flat" "$dst"
    fi
    n=$((n + 1))
  done < <(find "$mb_dir" -maxdepth 1 -type f -name '*.eml')

  [ "$n" -gt 0 ] || continue
  log "$(basename "$mb_dir"): $verb $n file(s)"
  moved=$((moved + n))
done

# --- rewrite manifest paths to the sharded layout --------------------------
# Rebuild path as <mailboxDir>/<shard>/<emailId>.eml from its first segment, so the rewrite
# is idempotent even on an already-sharded path. jq can't hash, so shard is computed here
# and passed in; $shard below is a jq --arg binding, not a shell var, hence single quotes.
# shellcheck disable=SC2016
rewrite_path='.path = ((.path | split("/")[0]) + "/" + $shard + "/" + .emailId + ".eml")'

MANIFEST="$DEST/manifest.ndjson"
if [ -f "$MANIFEST" ] && [ "$DRY" -eq 0 ] && [ "$moved" -gt 0 ]; then
  tmp="$MANIFEST.tmp.$$"
  : > "$tmp"
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    eid="$(printf '%s' "$line" | jq -r '.emailId')"
    shard="$(shard_of "$eid")"
    printf '%s' "$line" | jq -c --arg shard "$shard" "$rewrite_path" >> "$tmp"
  done < "$MANIFEST"
  mv "$tmp" "$MANIFEST"
  log "manifest paths rewritten ($MANIFEST)"
fi

log "----------------------------------------------------------------"
log "$verb $moved file(s) under $DEST"
