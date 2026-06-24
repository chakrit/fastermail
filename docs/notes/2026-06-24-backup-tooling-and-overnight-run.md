# Backup tooling + the overnight 67k-message run

**2026-06-24** · status: backup tooling shipped; full account backed up 100%.

This was a tangent off the main thread (the layering rearchitect — see
[next `/ace`](#open-items--next-ace)), triggered by a failed `backup-mail.sh` run.

## What shipped

A tempered, resumable parallel mail-backup pipeline under `scripts/` (six commits,
`bdb16f1`..`bf15a84`), grown out from the single `backup-mail.sh`:

- `_shard.sh` — shared `shard_of`: `sha256(emailId)[:2]`, a 256-way bucket.
- `sort.sh` — migrate flat `<mbox>/<id>.eml` → sharded `<mbox>/<shard>/<id>.eml`.
- `backup-parallel.sh` — `xargs -P` pool over pending messages + in-process watchdog.
- `backup-worker.sh` — one-message export unit (retry, fail-fast on `rc=124` timeout).
- `reindex.sh` — rebuild `manifest.ndjson` as a rich machine index.

Design points worth keeping:

- **Sharded storage** — `mail/<sanitizedName>__<mailboxId>/<sha256(id)[:2]>/<id>.eml`.
  emailIds are time-ordered (leading chars cluster — every id in a 17k Archive starts
  `Su`/`Sv`), so the shard key hashes the id; sharding by the raw prefix would pile
  everything into ~2 buckets.
- **Watchdog** (in `backup-parallel.sh`) halts the whole pool on a failure burst (rate
  limiter), disk < 3 GB, or `STALL_CYCLES` no-progress cycles (`0` = off). Makes an
  unattended run unable to spam Fastmail or fill the disk.
- **Manifest = machine index** — one ndjson record per stored `.eml`:
  `{emailId, mailboxId, receivedAt, from, subject, bytes, sha256, path}`. `reindex.sh`
  enriches with the from/subject/receivedAt that enumeration already fetches and throws
  away. Build the id→meta index **once** (`jq -n` + `--slurpfile`), not inside the
  per-record pipeline — the obvious way is O(n²) and crawls on a 64k manifest.
- **Resumability** — workers skip on-disk (sharded or legacy flat path); a re-run fetches
  only what's missing. Caveat: cached `.ids` means a plain re-run does **not** pick up
  *new* mail. Force re-enumeration with `rm <dest>/.backup-state/*.ids`, or build the
  `fm emails changes --since <state.json cursor>` delta (not wired yet — natural next
  feature, esp. for the notes-repo dashboard).

## The run (full account)

- 67,186 / 67,186 on disk (100%), all 33 mailboxes 0-gap, `manifest == disk`, ~31 GB.
- Pathological mailbox: **TORA Printing (`P7k`, 13,204)** — uniformly large print
  attachments. Two lessons: (1) reorder around it with `--exclude P7k` so the fast
  mailboxes finish first; (2) its oversized attachments exceed the 300 s export timeout —
  capture with `EXPORT_TIMEOUT=1800 STALL_CYCLES=0 ./scripts/backup-parallel.sh --only
  P7k`. All captured; nothing was truly un-exportable.
- Survived **3 machine crashes** — cause was a **macOS NFS-client kernel panic** from the
  parallel notes-repo NAS migration, *not* this backup — with zero loss. Integrity
  re-verified each time (`disk == manifest`, 0 zero-byte, `.ids` intact).
- `chakrit.notes.claude` migrated `mail/` → SMB NAS and verified cryptographically
  (sha256 vs manifest: 0 missing, 0 mismatch; 67,141 unique after multi-mailbox dedup).

## Open items / next `/ace`

1. **DECISION (chakrit's):** delete local `mail/` to free ~31 GB? NAS copy is verified
   identical; held for explicit OK. `notes.claude` is positioned to run the delete.
2. **`docs/guides/backup.md` is stale** — documents only the old `backup-mail.sh`. Update
   it to cover `sort` / `backup-parallel` / `reindex`, the giant-pass, and the
   incremental playbook.
3. **Main thread (unchanged by this session):** the layering rearchitect — **step 2**
   (Email read-projection relocation + typed mutation API shape) per
   `docs/notes/2026-06-21-layering-rearchitect-plan.md`. The backup work was a detour;
   that is still the pending feature.

## Gotchas (cost real time)

These are generic shell/ops lessons; candidates for the `general-coding` Shell section if
they recur:

- `pgrep -f 'backup-parallel.sh'` **self-matches** its own command line → false "running".
  Use a more specific pattern, or check for `backup-worker.sh` processes.
- A single `df` / `uptime` reading during heavy I/O can spike transiently (saw 39 GB → 3 GB
  → 39 GB within a minute, nearly triggering a false disk-emergency halt). Take 2–3 reads
  before acting on a low-disk / high-load number.
- `"$VAR…"` — a bare `$var` glued to a multibyte char (`…`) — trips `set -u`; brace it
  `${VAR}…`. Caught by a smoke test before the full launch.
