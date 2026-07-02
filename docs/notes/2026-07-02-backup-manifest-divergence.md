# Backup manifest: minimal (incremental) vs rich (reindexed) — divergence

**2026-07-02** · status: OPEN — chakrit's design call · surfaced by `chakrit.notes.claude`
over the ace-connect bridge.

## What

`scripts/backup-worker.sh` (the parallel / incremental path) writes **minimal** manifest
records: `{emailId, mailboxId, path, bytes, sha256}`. The rich `from` / `subject` /
`receivedAt` fields exist only where `scripts/reindex.sh` has run as a post-pass (one
`Email/query` per mailbox). The original full backup was reindexed → rich; **incremental
pulls stay minimal until `reindex.sh` is re-run.**

A consumer (`notes/index.py`) reading those three from the manifest got empty
subject/from + `received_at=0` for newly-pulled mail. It fixed its side by reading
`Subject`/`From`/`Date` from the `.eml` — correct: `docs/guides/backup.md` already
declares the **`.eml` the source of truth**, the manifest mere convenience metadata.

## Options (chakrit's call — deliberately not auto-fixed)

1. **Document the divergence** in `backup.md` (incremental records stay minimal until
   reindex'd; `.eml` is authoritative). Honest minimum; `.eml` consumers already
   unaffected. **Recommended** — the `.eml`-as-source stance is already the contract.
2. **Auto-run `reindex.sh`** after an incremental pull (manifest stays uniformly rich).
3. **Make `backup-worker.sh` write rich inline** — adds a per-message metadata fetch,
   defeating reindex's bulk (one query per mailbox) efficiency. Least preferred.
