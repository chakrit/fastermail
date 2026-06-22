# shellcheck shell=bash
# _shard.sh — shared shard-key logic, sourced by backup-mail.sh and sort.sh.
#
# Buckets a message by the first 2 hex of sha256(emailId) → 256 even buckets.
# emailIds are time-ordered, so their leading chars are near-constant (every id in a
# 17k-message Archive starts "Su"/"Sv") — sharding on the id prefix would pile almost
# everything into two folders. Hashing restores a uniform spread.
#
# Both scripts MUST share this function: a write and a later lookup that disagree on the
# bucket would silently lose the file.

if command -v shasum >/dev/null 2>&1; then
  sha256_hex() { shasum -a 256 | awk '{print $1}'; }
elif command -v sha256sum >/dev/null 2>&1; then
  sha256_hex() { sha256sum | awk '{print $1}'; }
else
  echo "error: _shard.sh needs shasum or sha256sum" >&2
  exit 1
fi

# shard_of <emailId> → 2-hex-char bucket name
shard_of() { printf '%s' "$1" | sha256_hex | cut -c1-2; }
