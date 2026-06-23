#!/usr/bin/env bash
# backup-parallel.sh — tempered parallel resume of the whole-account mail backup.
#
# Enumerates every mailbox, lists the not-yet-downloaded messages, and fetches them through
# an xargs -P pool of backup-worker.sh — message-level concurrency, so the few huge
# mailboxes parallelize internally (mailbox-level alone wouldn't finish: one big mailbox is
# ~9 h single-threaded). Concurrency is modest, and a built-in watchdog trips a STOP flag
# that halts every worker if failures spike (rate limiter) or disk runs low — so an
# unattended run cannot hammer Fastmail or fill the disk. Resumes: on-disk messages skip.
#
# Usage:
#   scripts/backup-parallel.sh                      # resume full account into ./mail
#   scripts/backup-parallel.sh --concurrency 6
#   scripts/backup-parallel.sh --only P7k           # one mailbox (id or name)
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source-path=SCRIPTDIR
# shellcheck source=_shard.sh
source "$SCRIPT_DIR/_shard.sh"

DEST="${BACKUP_DIR:-$ROOT_DIR/mail}"
FM="${FM:-$ROOT_DIR/target/release/fm}"
CONCURRENCY="${CONCURRENCY:-8}"
ONLY=""
EXCLUDE=""
while [ $# -gt 0 ]; do
  case "$1" in
    --dest)        DEST="$2"; shift 2 ;;
    --fm)          FM="$2"; shift 2 ;;
    --concurrency) CONCURRENCY="$2"; shift 2 ;;
    --only)        ONLY="$2"; shift 2 ;;
    --exclude)     EXCLUDE="$2"; shift 2 ;;
    -h|--help)     sed -n '2,16p' "$0"; exit 0 ;;
    *)             echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

# --- preflight -------------------------------------------------------------
die() { echo "error: $*" >&2; exit 1; }
command -v jq    >/dev/null 2>&1 || die "jq is required"
command -v xargs >/dev/null 2>&1 || die "xargs is required"
[ -x "$FM" ] || die "fm not executable: $FM"
"$FM" config --json | jq -e '.token != "(not set)"' >/dev/null 2>&1 \
  || die "no FastMail token resolves — run 'fm setup' or set FASTMAIL_API_TOKEN"

STATE_DIR="$DEST/.backup-state"
LOG_DIR="$DEST/logs"
mkdir -p "$STATE_DIR" "$LOG_DIR"

TS="$(date +%Y%m%dT%H%M%S)"
LOG="$LOG_DIR/backup-parallel-$TS.log"
FAILS="$LOG_DIR/failures.log"
WORK="$STATE_DIR/worklist.$TS"
STOP="$STATE_DIR/STOP"
rm -f "$STOP"

exec > >(tee -a "$LOG") 2>&1
log() { printf '%s  %s\n' "$(date +%Y-%m-%dT%H:%M:%S)" "$*"; }

sanitize() {
  local s
  s="$(printf '%s' "$1" | tr '/ ' '__' | LC_ALL=C tr -cd '[:alnum:]_.-')"
  [ -n "$s" ] || s="mailbox"
  printf '%s' "$s"
}

# A timeout command for the workers, if one exists (exported below).
EXPORT_TIMEOUT="${EXPORT_TIMEOUT:-300}"
TIMEOUT_BIN=""
if [ "$EXPORT_TIMEOUT" -gt 0 ]; then
  if command -v timeout >/dev/null 2>&1; then
    TIMEOUT_BIN="timeout"
  elif command -v gtimeout >/dev/null 2>&1; then
    TIMEOUT_BIN="gtimeout"
  fi
fi

log "parallel backup → $DEST   (concurrency=$CONCURRENCY, fm=$FM)"
[ -n "$ONLY" ] && log "restricted to mailbox: $ONLY"

# --- snapshot mailboxes ----------------------------------------------------
"$FM" mailboxes list --json > "$STATE_DIR/mailboxes.json" || die "could not list mailboxes"

# --- build the work list: <mbid> <safe> <eid>, one per enumerated message ---
# Workers skip anything already on disk, so listing every id (not just the missing ones)
# keeps this pass cheap and the resume logic in one place.
log "enumerating mailboxes + building work list…"
: > "$WORK"
while IFS="$(printf '\t')" read -r mb_id mb_name; do
  if [ -n "$ONLY" ] && [ "$ONLY" != "$mb_id" ] && [ "$ONLY" != "$mb_name" ]; then
    continue
  fi
  if [ -n "$EXCLUDE" ] && { [ "$EXCLUDE" = "$mb_id" ] || [ "$EXCLUDE" = "$mb_name" ]; }; then
    continue
  fi

  safe="$(sanitize "$mb_name")"
  ids_file="$STATE_DIR/${mb_id}.ids"
  if [ ! -s "$ids_file" ]; then
    "$FM" emails list -m "$mb_id" --all --json 2>/dev/null | jq -r '.[].id' > "$ids_file" \
      || { log "FAIL enumerate $mb_name ($mb_id)"; continue; }
  fi

  while IFS= read -r eid; do
    [ -n "$eid" ] && printf '%s %s %s\n' "$mb_id" "$safe" "$eid" >> "$WORK"
  done < "$ids_file"
done < <(jq -r '.[] | [.id, .name] | @tsv' "$STATE_DIR/mailboxes.json")

work_total="$(wc -l < "$WORK" | tr -d ' ')"
log "work items: $work_total (already-present messages are skipped by workers)"
[ "$work_total" -eq 0 ] && { log "nothing to do"; rm -f "$WORK"; exit 0; }

# --- run the pool, watched -------------------------------------------------
export BACKUP_DIR="$DEST"
export FM RETRIES="${RETRIES:-3}" EXPORT_TIMEOUT TIMEOUT_BIN

log "starting pool of ${CONCURRENCY} workers…"
xargs -P "$CONCURRENCY" -L1 "$SCRIPT_DIR/backup-worker.sh" < "$WORK" &
pool_pid=$!

# Watchdog: every 30s, halt the whole pool (STOP flag + kill) on any of:
#   disk < 3 GB free · a failure burst (rate limiter) · ~10 min with no new download.
MANIFEST="$DEST/manifest.ndjson"
fail_prev=0; [ -f "$FAILS" ]    && fail_prev="$(wc -l < "$FAILS" | tr -d ' ')"
done_prev=0; [ -f "$MANIFEST" ] && done_prev="$(wc -l < "$MANIFEST" | tr -d ' ')"
stall=0
while kill -0 "$pool_pid" 2>/dev/null; do
  sleep 30

  disk_gb="$(df -g "$DEST" | awk 'NR==2 {print $4}')"
  if [ "${disk_gb:-99}" -lt 3 ]; then
    log "WATCHDOG: disk ${disk_gb}GB free < 3 GB — tripping STOP"
    touch "$STOP"; kill "$pool_pid" 2>/dev/null; break
  fi

  fail_now=0; [ -f "$FAILS" ] && fail_now="$(wc -l < "$FAILS" | tr -d ' ')"
  if [ "$((fail_now - fail_prev))" -gt 60 ]; then
    log "WATCHDOG: $((fail_now - fail_prev)) failures in 30s — likely rate limit; tripping STOP"
    touch "$STOP"; kill "$pool_pid" 2>/dev/null; break
  fi

  done_now=0; [ -f "$MANIFEST" ] && done_now="$(wc -l < "$MANIFEST" | tr -d ' ')"
  if [ "$done_now" -le "$done_prev" ]; then
    stall=$((stall + 1))
    if [ "$stall" -ge 20 ]; then
      log "WATCHDOG: no new downloads for ~10 min — tripping STOP (stalled or offline)"
      touch "$STOP"; kill "$pool_pid" 2>/dev/null; break
    fi
  else
    stall=0
  fi

  log "progress: $done_now records on disk, $fail_now failure(s), stall=${stall}/20"
  fail_prev="$fail_now"; done_prev="$done_now"
done
wait "$pool_pid" 2>/dev/null

# --- summary ---------------------------------------------------------------
on_disk="$(find "$DEST" -name '*.eml' | wc -l | tr -d ' ')"
fail_count=0
[ -f "$FAILS" ] && fail_count="$(wc -l < "$FAILS" | tr -d ' ')"
rm -f "$WORK"

log "----------------------------------------------------------------"
log "pool finished: $on_disk .eml on disk, $fail_count failure(s) logged"
if [ -e "$STOP" ]; then
  log "STOP was tripped — run halted early; re-run to resume safely"
  exit 1
fi
log "run log: $LOG"
