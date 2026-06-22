# Backing up your whole account

`fm` has no `fm backup` command — by design. Instead it exposes the four primitives a
backup needs (mailbox enumeration, `--all` pagination, raw `.eml` export, and the
`changes` state cursor), and `scripts/backup-mail.sh` composes them into a resumable,
integrity-checked export of every message in the account. The rationale for primitives
over a built-in command is the
[JMAP-library decision](../decisions/2026-06-21-jmap-library-and-backup-primitives.md).

## What it produces

```
mail/
  Crypto__P8k/                 one dir per mailbox: <sanitized-name>__<jmap-id>
    Su0ldpOYpR_g.eml           one raw RFC822 message per file, attachments inline
    ...
  manifest.ndjson              one JSON line per message: id, mailbox, path, bytes, sha256
  .backup-state/
    mailboxes.json             snapshot of the mailbox tree (names, roles, hierarchy)
    state.json                 the Email state cursor captured at first run
    <mailbox-id>.ids           cached enumeration per mailbox (drives resume)
  logs/
    backup-<timestamp>.log     full run log, one per invocation
    failures.log               cumulative: any message that failed to export
```

The `.eml` files are the source of truth — byte-exact, lossless, attachments inline.
`manifest.ndjson` is convenience metadata for a future search index; it can be rebuilt
from the files.

## Prerequisites

- `fm` configured with a token (`fm setup`; verify with `fm config`).
- `jq` and `shasum`/`sha256sum` on `PATH`.

The script builds `target/release/fm` itself if it's missing. If you've changed `fm`
source, rebuild first (`cargo build --release`) so the script doesn't run a stale binary.

## Run it

```bash
scripts/backup-mail.sh                 # whole account → ./mail (gitignored)
scripts/backup-mail.sh --dest ~/fm     # somewhere else
```

It enumerates every mailbox, keys each by its JMAP **id** (so two folders sharing a name
are both captured), and downloads message by message. Expect it to be slow at scale — one
`fm` process per message, sequential by design to stay friendly to FastMail. At ~67k
messages this is a long, unattended run.

### Resuming

Interrupt it (or let it die) and just run it again — any `.eml` already on disk is
skipped, so it picks up where it left off. The per-mailbox `.ids` cache means a resume
doesn't re-page the account; pass `--refresh` to force re-enumeration if you suspect the
cache is stale.

## Integrity

Two checks, both automatic:

1. **Per message** — each export must produce a non-empty file whose first line looks
   like an RFC822 header. A failure leaves no partial file behind; the id is logged to
   `failures.log` and the run continues.
2. **Per mailbox** — after a full run the script reconciles enumerated-id count against
   `.eml` files on disk and **exits non-zero** if any mailbox is short, naming the gap in
   the log.

So `scripts/backup-mail.sh && echo OK` is a meaningful "the backup is complete" signal.
Re-run after a failure: the gaps are retried, the rest skipped.

## Keeping it current

The first run records the Email state cursor in `.backup-state/state.json`. To pull only
what changed since, use it with `fm emails changes` (see [scripting](scripting.md)):

```bash
state=$(jq -r '.state' mail/.backup-state/state.json)
fm emails changes --since "$state" --json     # created / updated / destroyed + newState
```

Export the `created`/`updated` ids the same way the script does, and advance the cursor
to the returned `newState`. (A from-scratch re-run also works — it only downloads what's
missing.)

## Smoke-testing

Cap the work to prove the pipeline before committing to a full run:

```bash
scripts/backup-mail.sh --only <mailbox-id-or-name> --max-per-mailbox 5
```

`--only` restricts to one mailbox; `--max-per-mailbox N` stops after N messages. Neither
flag belongs in a real backup.
