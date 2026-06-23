#!/usr/bin/env bash
# reindex.sh — rebuild manifest.ndjson as a machine index.
#
# Enriches every record with the JMAP metadata the backup fetches then discards
# (receivedAt / from / subject) and drops the denormalized mailbox *name* (mailboxId is
# the stable key; the name lives in mailboxes.json). Integrity fields (bytes / sha256) are
# reused from the existing manifest — the backup already hashed each file, so no body is
# re-read or re-downloaded. The only network cost is one Email/query per mailbox.
#
# Usage:
#   scripts/reindex.sh                 # rebuild ./mail/manifest.ndjson in place
#   scripts/reindex.sh --dest ~/fm     # a backup elsewhere
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

DEST="${BACKUP_DIR:-$ROOT_DIR/mail}"
FM="${FM:-$ROOT_DIR/target/release/fm}"
while [ $# -gt 0 ]; do
  case "$1" in
    --dest)    DEST="$2"; shift 2 ;;
    --fm)      FM="$2"; shift 2 ;;
    -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
    *)         echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

command -v jq >/dev/null 2>&1 || { echo "error: jq is required" >&2; exit 1; }
MANIFEST="$DEST/manifest.ndjson"
[ -s "$MANIFEST" ] || { echo "error: no manifest at $MANIFEST" >&2; exit 1; }
[ -x "$FM" ]       || { echo "error: fm not executable: $FM" >&2; exit 1; }

log() { printf '%s  %s\n' "$(date +%H:%M:%S)" "$*"; }

# --- gather metadata: one Email/query per mailbox present in the manifest --
meta_lines="$(mktemp)"
meta_array="$(mktemp)"
trap 'rm -f "$meta_lines" "$meta_array"' EXIT

while IFS= read -r mb_id; do
  [ -n "$mb_id" ] || continue
  log "metadata: mailbox $mb_id"
  "$FM" emails list -m "$mb_id" --all --json 2>/dev/null \
    | jq -c '.[] | {id, receivedAt, from: (.from[0].email // null), subject}' >> "$meta_lines"
done < <(jq -r '.mailboxId' "$MANIFEST" | sort -u)

jq -cs '.' "$meta_lines" > "$meta_array"

# --- enrich each record; integrity + locator fields carried over unchanged --
# Build the id->metadata index ONCE under `-n`, then stream the manifest. Putting the index
# build inside the per-record pipeline (the obvious way) rebuilds it for every record —
# O(n^2), which crawls on a 64k manifest.
tmp="$MANIFEST.tmp.$$"
jq -c -n --slurpfile m "$meta_array" --slurpfile recs "$MANIFEST" '
  ($m[0] | reduce .[] as $e ({}; .[$e.id] = $e)) as $idx
  | $recs[]
  | {
      emailId,
      mailboxId,
      receivedAt: $idx[.emailId].receivedAt,
      from:       $idx[.emailId].from,
      subject:    $idx[.emailId].subject,
      bytes,
      sha256,
      path
    }
' > "$tmp"
mv "$tmp" "$MANIFEST"

log "reindexed $(wc -l < "$MANIFEST" | tr -d ' ') record(s) in $MANIFEST"
