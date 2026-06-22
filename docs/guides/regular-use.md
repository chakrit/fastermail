# Everyday use

Day-to-day email from the terminal: triage the inbox, search, organize, and dip into
mailboxes, contacts, and the other resources. This walks the common tasks; for the
exhaustive flag list see [`../reference/cli.md`](../reference/cli.md).

Assumes `fm` is installed and a token is configured (`fm setup`). Check with `fm config`.

## The triage loop

The core workflow is read → search → move → flag → archive. Three shortcuts skip the
`emails` prefix:

```bash
fm ls                       # list the inbox (default mailbox)
fm ls sent                  # list another mailbox by role alias
fm ls "Projects"            # ...or by name (fuzzy: "proj" works too)
fm read <email-id>          # read one message (text body)
fm read <email-id> --format html
fm mv <email-id> archive    # move (role alias, name, or raw id)
fm mv <id1> <id2> trash     # move several at once
```

`fm ls` / `fm read` / `fm mv` are aliases for `fm emails list` / `get` / `move`.

### Flag and delete

```bash
fm emails flag <email-id> --flag seen              # mark read
fm emails flag <email-id> --flag flagged           # star it
fm emails flag <email-id> --flag flagged --unset   # un-star
fm emails delete <email-id>                        # → Trash
fm emails delete <email-id> --permanent            # skip Trash
```

Flags are JMAP keywords: `seen`, `flagged`, `answered`, `draft`.

## Search

At least one filter is required; combine freely.

```bash
fm emails search -q "invoice"
fm emails search --from alice@example.com --after 2025-01-01
fm emails search --subject "report" --has-attachment
fm emails search -q meeting -m archive          # restrict to one mailbox
```

`-n <N>` caps results (default 20). `--all` pages the entire match set, oldest first,
ignoring `-n` — handy for piping, but see [scripting](scripting.md) and
[backup](backup.md) for bulk work.

## Sending

Sending is deliberately minimal — compose context-aware replies through an MCP assistant
instead. For a quick one-off:

```bash
fm emails send --to user@example.com --subject "Hello" --body "Hi there"
echo "Body from a pipe" | fm emails send --to user@example.com --subject "Piped"
```

`--cc`, `--bcc`, and `--html` are available; `--body` reads stdin when omitted.

## Mailboxes

```bash
fm mailboxes list
fm mailboxes create "Projects"
fm mailboxes rename <mailbox-id> "New Name"
fm mailboxes delete <mailbox-id>
```

Anywhere a mailbox is accepted (`-m`, `--to`, shortcut positionals) you can pass a **role
alias** (`inbox`, `sent`, `drafts`, `trash`, `junk`, `archive`), a **name** (exact →
prefix → substring, case-insensitive), or a **raw JMAP id**. The id is the only
unambiguous handle when two mailboxes share a name — `fm mailboxes list` shows ids.

## Contacts

```bash
fm contacts list
fm contacts search "alice"
fm contacts create "Alice Smith" --email work:alice@corp.com --phone +15551234
fm contacts update <contact-id> --company "Acme"
fm contacts delete <contact-id>
fm contacts address-books            # list address books
```

`--email` / `--phone` are repeatable and accept an optional `type:` prefix
(`work:a@b.com`, or just `a@b.com`).

## Vacation and masked email

```bash
fm vacation get
fm vacation set --enabled --subject "OOO" --text-body "Back Monday"
fm vacation set --disabled

fm masked-emails list
fm masked-emails create --domain example.com --description "newsletter signup"
fm masked-emails update <id> --state disabled
```

## Output

Human tables on a TTY; JSON when piped or with `--json`; full JMAP with `--raw`. See
[scripting](scripting.md) for the machine-readable side.
