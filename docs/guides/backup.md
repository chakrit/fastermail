# Backing up your whole account

`fm` has no `fm backup` command — by design. It exposes the four primitives a backup needs
(mailbox enumeration, `--all` pagination, raw `.eml` export, and the `changes` state
cursor); `scripts/` composes them into a resumable, integrity-checked export of every
message in the account. The rationale for primitives over a built-in command is the
[JMAP-library decision](../decisions/2026-06-21-jmap-library-and-backup-primitives.md).

Four scripts, two runners and two maintenance tools:

- `backup-parallel.sh` — recommended runner; a tempered `xargs -P` pool, the one that
  exported the 67k-message account.
- `backup-mail.sh` — simple sequential alternative, one `fm` process at a time.
- `reindex.sh` — rebuild `manifest.ndjson` as a searchable machine index.
- `sort.sh` — migrate an old flat backup into the sharded tree.

## What it produces

```
mail/
  Important__P6V/                one dir per mailbox: <sanitized-name>__<jmap-id>
    f5/                          shard = sha256(emailId)[:2] — 256 even buckets
      SuvfBs0iTulc.eml           one raw RFC822 message per file, attachments inline
      ...
  manifest.ndjson              one JSON line per message: emailId, mailboxId, path, bytes, sha256
  .backup-state/
    mailboxes.json             snapshot of the mailbox tree (names, roles, hierarchy)
    state.json                 the Email state cursor captured at first run
    <mailbox-id>.ids           cached enumeration per mailbox (drives resume)
    STOP                       circuit-breaker flag; present only while a run is halting
    worklist.<timestamp>       transient work list (parallel runner; removed on clean exit)
  logs/
    backup-parallel-<ts>.log   per-run log (backup-mail.sh: backup-<ts>.log)
    failures.log               cumulative: any message that failed to export
```

Why sharded: emailIds are time-ordered, so their leading characters barely vary (every id
in a 17k-message Archive starts `Su`/`Sv`). Bucketing on the id prefix would pile almost
everything into two folders; hashing the id (`sha256(id)[:2]`) restores a uniform 256-way
spread. The writer and every later lookup share one `shard_of` (`scripts/_shard.sh`) — a
disagreement would silently lose files.

The `.eml` files are the source of truth — byte-exact, lossless, attachments inline.
`manifest.ndjson` is convenience metadata; it can be rebuilt from the files
(`reindex.sh`).

## Prerequisites

- `fm` configured with a token (`fm setup`; verify with `fm config`).
- `jq` and `shasum`/`sha256sum` on `PATH`; `xargs` for the parallel runner.

The scripts build `target/release/fm` if it's missing. If you've changed `fm` source,
rebuild first (`cargo build --release`) so the script doesn't run a stale binary.

## Run it

`backup-parallel.sh` is the runner to use at scale — message-level concurrency, so the
few huge mailboxes parallelize internally (mailbox-level alone wouldn't finish: one big
mailbox is ~9 h single-threaded):

```bash
scripts/backup-parallel.sh                    # whole account → ./mail (gitignored)
scripts/backup-parallel.sh --concurrency 6    # default is 8
scripts/backup-parallel.sh --only P7k         # one mailbox (id or name)
scripts/backup-parallel.sh --exclude P7k      # everything but one mailbox
scripts/backup-parallel.sh --dest ~/fm        # somewhere else
```

`backup-mail.sh` is the simpler sequential alternative — same layout and manifest, one
`fm` process at a time, no pool or watchdog. Fine for a small account or a single mailbox:

```bash
scripts/backup-mail.sh --only Crypto --max-per-mailbox 5
```

Both key each mailbox by its JMAP **id** (two folders sharing a name are both captured)
and skip any `.eml` already on disk.

## Tempering (parallel runner)

A built-in watchdog polls every 30 s and trips a `STOP` flag — every worker then drains
its queue with no further Fastmail calls — on any of:

- **disk < 3 GB free** — stops before filling the volume;
- **a failure burst** (> 60 in 30 s) — treats it as a rate limit and backs off;
- **a stall** — `STALL_CYCLES` consecutive no-download cycles (default 20 ≈ 10 min; `0`
  disables), i.e. offline or wedged.

So an unattended run can neither hammer Fastmail nor fill the disk. Knobs, all env vars:

- `CONCURRENCY` (8) — pool size; `--concurrency` does the same.
- `EXPORT_TIMEOUT` (300) — per-message export timeout in seconds; needs
  `timeout`/`gtimeout`.
- `STALL_CYCLES` (20) — no-progress 30 s cycles before STOP; `0` disables the stall guard.
- `RETRIES` (3) — per-message attempts before a failure is logged.

A `STOP`-tripped run exits non-zero; just re-run to resume.

## Giant mailboxes

A mailbox of uniformly huge attachments (the 13k-message TORA print folder, `P7k`) is
pathological two ways. Reorder around it so the fast mailboxes finish first:

```bash
scripts/backup-parallel.sh --exclude P7k      # everything else first
```

then capture it with a long timeout and the stall guard off — its messages legitimately
take many minutes each, which would otherwise read as a stall:

```bash
EXPORT_TIMEOUT=1800 STALL_CYCLES=0 scripts/backup-parallel.sh --only P7k
```

A per-message timeout (`rc=124`) is logged and **not** retried — re-pulling a doomed giant
just hammers the server; the longer-timeout pass above is the fix.

## Resuming

Interrupt it (or let the watchdog stop it) and run it again — any `.eml` already on disk
is skipped (sharded path or a pre-`sort` flat path), so it picks up where it left off. The
per-mailbox `.ids` cache means a resume doesn't re-page the account; to force fresh
enumeration pass `--refresh` (`backup-mail.sh`) or `rm mail/.backup-state/*.ids` (either
runner).

## Integrity

Two checks:

1. **Per message** — each export must produce a non-empty file whose first line looks like
   an RFC822 header; a failure leaves no partial file behind and is logged to
   `failures.log`.
2. **Per mailbox** — `backup-mail.sh` reconciles enumerated-id count against `.eml` files
   on disk and **exits non-zero** if any mailbox is short, naming the gap. So
   `scripts/backup-mail.sh && echo OK` is a meaningful "complete" signal. The parallel
   runner logs failures and exits non-zero only on a `STOP`; reconcile a parallel run with
   a follow-up `backup-mail.sh`, which retries the gaps and skips the rest.

## Search index

`manifest.ndjson` as written carries only locator and integrity fields (`emailId`,
`mailboxId`, `path`, `bytes`, `sha256`). `reindex.sh` rebuilds it into a richer machine
index:

```bash
scripts/reindex.sh                 # rewrite ./mail/manifest.ndjson in place
```

It enriches each record with `receivedAt` / `from` / `subject` — the metadata enumeration
already fetches and discards (one `Email/query` per mailbox; no body re-read, no
re-download) — reuses the existing `bytes` / `sha256`, and drops the denormalized mailbox
*name* (`mailboxId` is the stable key; names live in `mailboxes.json`).

## Migrating an old flat backup

Early `backup-mail.sh` wrote every message of a mailbox into one flat directory — 17k
files in a single folder. `sort.sh` relocates them into the sharded tree and rewrites
the manifest paths to match. Idempotent: already-sharded files are left alone, so a
re-run is a no-op.

```bash
scripts/sort.sh --dry-run          # report what would move, touch nothing
scripts/sort.sh                    # shard ./mail in place
```

## Keeping it current

The first run records the Email state cursor in `.backup-state/state.json`. To pull only
what changed since, use it with `fm emails changes` (see [scripting](scripting.md)):

```bash
state=$(jq -r '.state' mail/.backup-state/state.json)
fm emails changes --since "$state" --json     # created / updated / destroyed + newState
```

Export the `created`/`updated` ids the same way the runners do, and advance the cursor to
the returned `newState`.

**Caveat:** a plain re-run does **not** pick up *new* mail — the cached `.ids` are reused,
so enumeration never re-runs. Force fresh enumeration first
(`rm mail/.backup-state/*.ids`, or `--refresh` with `backup-mail.sh`), or drive an
incremental pull from the `changes` delta above. The delta isn't wired into a script yet.

## Smoke-testing

Cap the work to prove the pipeline before committing to a full run:

```bash
scripts/backup-mail.sh --only <mailbox-id-or-name> --max-per-mailbox 5
```

`--only` restricts to one mailbox; `--max-per-mailbox N` stops after N messages. Neither
flag belongs in a real backup.
</content>
</invoke>
