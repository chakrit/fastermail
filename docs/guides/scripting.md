# Scripting fm

`fm` is built to compose. Every command emits machine-readable JSON, exit codes are
stable, and stdin feeds message bodies — enough to drive it from shell, cron, or any
language that can spawn a process. For the per-command flag list see
[`../reference/cli.md`](../reference/cli.md).

## JSON output

Output mode auto-detects: a TTY gets human tables, anything else (a pipe, a file, a
subprocess) gets JSON. Force it with `--json` when you want JSON on a terminal too.

```bash
fm ls inbox --all --json | jq -r '.[].id'           # every inbox id
fm mailboxes list --json | jq -r '.[] | "\(.id)\t\(.name)"'
fm emails search --from alice@example.com --json | jq length
```

`--raw` returns the unparsed JMAP response instead — reach for it when you need a field
`fm`'s simplified JSON drops.

Note JMAP ids can contain characters your shell treats specially; always quote them.

## Exit codes

Check `$?`, not stderr text. The codes:

| Code | Meaning                                          |
|------|--------------------------------------------------|
| 0    | Success                                          |
| 1    | Startup error (missing token, connection failure)|
| 2    | Invalid arguments                                |
| 3    | API error (the JMAP call failed)                 |

```bash
if ! fm emails export "$id" --to "$id.eml"; then
  echo "export failed for $id (exit $?)" >&2
fi
```

## Stable mailbox handles

Role aliases (`inbox`, `archive`, …) and names are convenient interactively, but names
are ambiguous when two mailboxes share one. In scripts, resolve once and key off the raw
**JMAP id** — it is stable and unique:

```bash
crypto_id=$(fm mailboxes list --json | jq -r '.[] | select(.name=="Crypto") | .id' | head -1)
fm emails list -m "$crypto_id" --all --json
```

`-m` accepts a raw id directly, so no second lookup is needed on later calls.

## Paging the full set

`-n` caps results; `--all` pages the entire match via JMAP `position`/`anchor`, oldest
first, so nothing is silently truncated. Use it for any "process every message" job:

```bash
fm emails list -m archive --all --json | jq -r '.[].id' | while read -r id; do
  fm emails export "$id" --to "archive/$id.eml"
done
```

## Incremental sync

`fm emails changes` exposes JMAP state tokens. Capture a cursor now, ask for the delta
later:

```bash
state=$(fm emails changes --json | jq -r '.state')      # bootstrap cursor
# ...time passes...
fm emails changes --since "$state" --json                # created/updated/destroyed + newState
```

Feed the returned `newState` into the next call. A too-old token returns
`cannotCalculateChanges` — fall back to a full `--all` enumeration. This is the engine
behind incremental [backup](backup.md).

## Piping bodies

`fm emails send` reads the body from stdin when `--body` is omitted:

```bash
render-report | fm emails send --to team@corp.com --subject "Nightly" --html
```

## A worked example

The whole-account [backup script](backup.md) (`scripts/backup-mail.sh`) is the canonical
example: enumerate mailboxes → page each with `--all` → `export` every message → check
exit codes → reconcile counts. Read it as a reference for non-trivial automation.
