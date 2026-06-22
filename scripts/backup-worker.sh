#!/usr/bin/env bash
# backup-worker.sh — export ONE message; the unit of backup-parallel.sh's xargs pool.
#
# Args: <mailboxId> <safeName> <emailId>. Self-contained so xargs can spawn it per message.
# Skips messages already on disk (resume) and bails the moment the circuit-breaker STOP
# flag appears, so a rate-limit trip drains the queue with no further Fastmail calls.
# Always exits 0 — a failure is logged, never propagated (xargs must keep going).
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source-path=SCRIPTDIR
# shellcheck source=_shard.sh
source "$SCRIPT_DIR/_shard.sh"

mb_id="$1"
safe="$2"
eid="$3"

DEST="${BACKUP_DIR:-$ROOT_DIR/mail}"
FM="${FM:-$ROOT_DIR/target/release/fm}"
RETRIES="${RETRIES:-3}"
EXPORT_TIMEOUT="${EXPORT_TIMEOUT:-300}"
TIMEOUT_BIN="${TIMEOUT_BIN:-}"

FAILS="$DEST/logs/failures.log"
MANIFEST="$DEST/manifest.ndjson"
STOP="$DEST/.backup-state/STOP"

shard="$(shard_of "$eid")"
dir="$DEST/${safe}__${mb_id}"
file="$dir/$shard/$eid.eml"

# --- fast exits: breaker tripped, or already have it (sharded or pre-sort flat) ---
[ -e "$STOP" ] && exit 0
{ [ -s "$file" ] || [ -s "$dir/$eid.eml" ]; } && exit 0

looks_rfc822() {
  [ -s "$1" ] || return 1
  head -1 "$1" | LC_ALL=C grep -qE '^(From |[!-9;-~]+:)'
}

fm_export() {  # fm_export <out-path>
  if [ -n "$TIMEOUT_BIN" ]; then
    "$TIMEOUT_BIN" "$EXPORT_TIMEOUT" "$FM" emails export "$eid" --to "$1"
  else
    "$FM" emails export "$eid" --to "$1"
  fi
}

mkdir -p "$dir/$shard"

attempt=1
while :; do
  [ -e "$STOP" ] && exit 0
  reason="$(fm_export "$file" 2>&1 >/dev/null)"
  rc=$?
  if [ "$rc" -eq 0 ] && looks_rfc822 "$file"; then
    jq -nc \
      --arg id "$eid" --arg mid "$mb_id" --arg path "${safe}__${mb_id}/$shard/$eid.eml" \
      --argjson bytes "$(wc -c < "$file" | tr -d ' ')" --arg sha "$(sha256_hex < "$file")" \
      '{emailId:$id, mailboxId:$mid, path:$path, bytes:$bytes, sha256:$sha}' >> "$MANIFEST"
    exit 0
  fi

  rm -f "$file"
  if [ "$attempt" -ge "$RETRIES" ]; then
    reason="$(printf '%s' "$reason" | tr '\n' ' ')"
    printf '%s %s %s rc=%s %s\n' "$(date +%Y-%m-%dT%H:%M:%S)" "$mb_id" "$eid" "$rc" "$reason" >> "$FAILS"
    exit 0
  fi
  sleep "$attempt"
  attempt=$((attempt + 1))
done
