# CLI Mode

FasterMail ships as a single `fm` binary that operates in two modes:

- **MCP mode** (`fm mcp`): stdio JSON-RPC server for AI assistants.
- **CLI mode** (`fm <resource> <verb>`): interactive command-line tool for humans and scripts.

Both modes share the same action implementations and JMAP client.

## Design Philosophy

The CLI is primarily an **email organization and triage** tool. The core workflow is:
read, search, move, flag, archive — the inbox-zero loop.

Sending and replying are supported but minimal. Composing email is better done through
MCP-enabled AI assistants, which can draft context-aware replies. The CLI just needs to
get messages out of the way.

**Optimize for:**

- Moving emails between mailboxes (the core action)
- Searching and filtering
- Flagging/unflagging
- Reading and listing

**Minimal support:**

- `fm emails send` — basic, no interactive compose
- Reply workflows — leave to MCP

## Binary Name

The binary is named `fm`. The Cargo package name remains `fastermail`; the binary target is `fm`.

## Command Structure

```
fm                          Show help
fm mcp                      Run MCP server (stdio JSON-RPC)
fm --version                Print version
fm --help                   Print help

# Email triage (primary workflow)
fm emails list              List emails from a mailbox
fm emails search            Search emails with filters
fm emails get <id>          Get full body of a single email
fm emails move              Move emails between mailboxes
fm emails flag              Set/unset flags on emails
fm emails delete            Delete emails

# Email sending (minimal)
fm emails send              Compose and send an email

# Mailbox management
fm mailboxes list           List all mailboxes
fm mailboxes create         Create a mailbox
fm mailboxes rename         Rename a mailbox
fm mailboxes delete         Delete a mailbox

# Other resources
fm identities list          List sending identities
fm vacation get             Get vacation/auto-reply settings
fm vacation set             Enable/disable/update vacation auto-reply
fm masked-emails list       List masked email addresses
fm masked-emails create     Create a new masked email address
fm masked-emails update     Enable/disable/delete a masked email

# Contacts
fm contacts list            List contacts
fm contacts search          Search contacts
fm contacts create          Create a new contact
fm contacts update          Update a contact
fm contacts delete          Delete a contact
fm contacts address-books   List address books

# Configuration
fm setup                    Interactive first-time setup (writes config file)
fm config                   Print active token and its source
```

### Top-Level Shortcuts

For the triage workflow, these shortcuts skip the `emails` prefix:

```
fm mv <id>... <mailbox>     → fm emails move <id>... --to <mailbox>
fm ls [mailbox]             → fm emails list [--mailbox <mailbox>]
fm read <id>                → fm emails get <id>
```

These are clap aliases, not separate implementations.

## Mailbox Resolution

Mailboxes can be specified by ID, role alias, or fuzzy name. This applies everywhere a
mailbox is accepted: `--mailbox`, `--to`, positional args in shortcuts.

### Built-in Role Aliases

These names resolve to mailbox IDs via the JMAP `role` field:

| Alias | JMAP Role |
|-------|-----------|
| `inbox` | `inbox` |
| `sent` | `sent` |
| `drafts` | `drafts` |
| `trash` | `trash` |
| `junk` | `junk` |
| `archive` | `archive` |

### Fuzzy Name Matching

When the input doesn't match a role alias or a full mailbox ID, resolve by:

1. **Exact name match** (case-insensitive): `Projects` matches "Projects"
2. **Prefix match** (case-insensitive): `proj` matches "Projects"
3. **Substring match** (case-insensitive): `ject` matches "Projects"

If multiple mailboxes match, prompt the user to select (interactive mode) or fail with
the list of candidates (non-interactive / JSON mode).

### Examples

```bash
fm mv e-abc123 trash              # role alias → Trash mailbox
fm mv e-abc123 proj               # fuzzy → "Projects" mailbox
fm ls inbox                       # role alias → Inbox
fm emails list --mailbox archive  # role alias → Archive
```

## Output Modes

`--json` and `--raw` are **global flags** defined on the root command (`global = true`), so
they apply to every subcommand whether or not it is re-listed below. Every command supports
three output formats:

| Mode | Flag | When | Description |
|------|------|------|-------------|
| Human | (default) | TTY | Colors, spinners, aligned tables, status indicators |
| JSON | `--json` | non-TTY or flag | Simplified JSON matching MCP tool responses |
| Raw | `--raw` | flag only | Full JMAP response for debugging |

**Auto-detection:** When stdout is not a terminal, default to JSON mode (like piping to
`jq` or another program). The `--json` flag forces JSON even on a TTY. Human mode is
only used when stdout is a TTY and no format flag is given.

### Human Output

Uses `console` for colors and `indicatif` for progress spinners. Status indicators:

- `✓` green — success
- `⚠` yellow — warning
- `✗` red — error
- `→` dim — hint

```
$ fm ls inbox -n 3
✓ 3 emails in Inbox

ID          DATE                 FROM                    SUBJECT
e-abc123    2024-03-15 09:30    alice@example.com       Meeting tomorrow
e-def456    2024-03-14 17:22    bob@corp.com            Q1 Report
e-ghi789    2024-03-14 11:05    notifications@gh.com    [PR] Review requested
```

```
$ fm mv e-abc123 archive
✓ Moved 1 email to Archive
```

### JSON Output

```
$ fm ls inbox -n 3 --json
[
  {"id": "e-abc123", "date": "2024-03-15T09:30:00Z", "from": [...], "subject": "Meeting tomorrow", "preview": "..."},
  ...
]
```

## UX Patterns

### Io Struct

Centralized output through an `Io` struct (adapted from ACE's pattern):

```rust
pub struct Io { /* output_mode, stderr handle, spinner state */ }

impl Io {
    pub fn progress(&self, msg: &str) -> /* spinner handle */;
    pub fn done(&self, msg: &str);      // ✓ green
    pub fn warn(&self, msg: &str);      // ⚠ yellow
    pub fn error(&self, msg: &str);     // ✗ red
    pub fn hint(&self, msg: &str);      // → dim
    pub fn data(&self, msg: &str);      // stdout, raw data (tables, JSON)
    pub fn separator(&self);            // visual break
}
```

All user-facing output goes through `Io`. Commands never write directly to
stdout/stderr. In JSON/Raw mode, `progress`/`done`/`warn` are suppressed; only `data`
and `error` produce output.

### Interactive Prompts

When disambiguation is needed (e.g., fuzzy mailbox match returns multiple results),
use `inquire` to prompt:

```
$ fm mv e-abc123 pro
? Multiple mailboxes match "pro":
> Projects
  Promotions
  Process Notes
```

Prompts are skipped in non-interactive mode (non-TTY stdin); the command fails with
candidates listed in the error message instead.

### Terminal Cleanup

A `TerminalGuard` (RAII) ensures terminal state is restored on SIGINT or panic —
clears active spinners, re-shows cursor, resets colors.

## Authentication

### Environment Variable (default)

```bash
export FASTMAIL_API_TOKEN=fmu1-...
fm emails list
```

### Config File

`~/.config/fastermail/config.toml`:

```toml
[auth]
token = "fmu1-..."
```

Precedence: `FASTMAIL_API_TOKEN` env var > config file.

The config file is created with `0600` permissions. `fm` refuses to run if the file is
world-readable.

## CLI Argument Design

Conventions:

- **Required positional args** for primary identifiers: `fm emails get <id>`.
- **Named flags** for optional parameters: `--mailbox`, `--limit`, `--format`.
- **Flag names match MCP parameter names** where possible, converted to kebab-case:
  `mailboxId` → `--mailbox-id`, `includeBody` → `--include-body`.
- **Short aliases** for common flags: `-n` for `--limit`, `-m` for `--mailbox`.
- **Mailbox flags accept any resolution form**: ID, role alias, or fuzzy name.

### Per-Command Arguments

#### `fm emails list`

```
fm emails list [OPTIONS]

Options:
  -m, --mailbox <MAILBOX>    Mailbox (ID, role alias, or name)
  -n, --limit <N>            Max results (default 20)
      --all                  Fetch every match via pagination (ignores --limit; oldest first)
      --include-body         Include body content
      --json                 JSON output
      --raw                  Raw JMAP output
```

#### `fm emails search`

```
fm emails search [OPTIONS]

At least one filter is required.

Options:
  -q, --keyword <TEXT>       Full-text search
      --from <ADDR>          Sender address filter
      --to <ADDR>            Recipient address filter
      --subject <TEXT>        Subject filter
  -m, --mailbox <MAILBOX>    Restrict to mailbox (ID, role alias, or name)
      --has-attachment        Filter for emails with attachments
      --after <YYYY-MM-DD>   Date lower bound
      --before <YYYY-MM-DD>  Date upper bound
  -n, --limit <N>            Max results (default 20)
      --all                  Fetch every match via pagination (ignores --limit; oldest first)
      --include-body         Include body content
      --json                 JSON output
      --raw                  Raw JMAP output
```

#### `fm emails get <EMAIL_ID>`

```
fm emails get <EMAIL_ID> [OPTIONS]

Options:
      --format <FORMAT>      text, html, or both (default text)
      --json                 JSON output
      --raw                  Raw JMAP output
```

#### `fm emails changes`

```
fm emails changes [OPTIONS]

Incremental sync cursor: the created/updated/destroyed email ids since a prior
JMAP state token. Pass the returned newState on the next call. A too-old state
yields a cannotCalculateChanges error — fall back to a full `--all` enumeration.

Omit --since to print the current state token instead — the bootstrap cursor a
first sync captures before any changes exist to ask for.

Options:
      --since <STATE>        State to fetch changes since; omit to print the
                             current state (bootstrap cursor)
  -n, --limit <N>            Max changes per call
      --json                 JSON output
      --raw                  Raw JMAP output
```

#### `fm emails export <EMAIL_ID>`

```
fm emails export <EMAIL_ID> [OPTIONS]

Download the message's raw RFC822 (.eml) bytes — lossless, byte-exact, with
attachments inline. The building block for backup.

Options:
      --to <PATH>            Write to this file (default: raw bytes to stdout)
```

#### `fm emails move`

```
fm emails move <EMAIL_ID>... --to <MAILBOX>

Options:
      --to <MAILBOX>         Target mailbox (ID, role alias, or name)
```

#### `fm emails flag`

```
fm emails flag <EMAIL_ID>... --flag <FLAG> [--unset]

Options:
      --flag <FLAG>          seen, flagged, answered, or draft
      --unset                Unset the flag (default: set)
```

#### `fm emails delete`

```
fm emails delete <EMAIL_ID>... [OPTIONS]

Options:
      --permanent            Permanently delete (skip trash)
```

#### `fm emails send`

```
fm emails send [OPTIONS]

Options:
      --to <ADDR>            Recipient (repeatable)
      --subject <TEXT>        Subject line
      --body <TEXT>           Body text (or read from stdin if omitted)
      --cc <ADDR>             CC recipient (repeatable)
      --bcc <ADDR>            BCC recipient (repeatable)
      --html                  Body is HTML
      --reply-to <EMAIL_ID>  Email ID being replied to
```

#### `fm mailboxes list`

```
fm mailboxes list [OPTIONS]

Options:
      --role <ROLE>          Filter by role (inbox, sent, drafts, trash, junk, archive)
      --json                 JSON output
```

#### `fm mailboxes create`

```
fm mailboxes create <NAME> [OPTIONS]

Options:
      --parent-id <ID>       Parent mailbox ID
```

#### `fm mailboxes rename`

```
fm mailboxes rename <MAILBOX_ID> <NEW_NAME>
```

#### `fm mailboxes delete`

```
fm mailboxes delete <MAILBOX_ID>
```

#### `fm identities list`

```
fm identities list [OPTIONS]

Options:
      --json                 JSON output
```

#### `fm vacation get`

```
fm vacation get [OPTIONS]

Options:
      --json                 JSON output
```

#### `fm vacation set`

```
fm vacation set [OPTIONS]

Options:
      --enabled              Enable auto-reply
      --disabled             Disable auto-reply
      --from <DATE>          Start date (ISO 8601)
      --to <DATE>            End date (ISO 8601)
      --subject <TEXT>        Auto-reply subject
      --text-body <TEXT>      Plain text body
      --html-body <TEXT>      HTML body
```

#### `fm masked-emails list`

```
fm masked-emails list [OPTIONS]

Options:
      --state <STATE>        Filter: pending, enabled, disabled, deleted
      --json                 JSON output
```

#### `fm masked-emails create`

```
fm masked-emails create [OPTIONS]

Options:
      --domain <DOMAIN>      Domain this address is for
      --description <TEXT>    Human-readable label
      --prefix <TEXT>         Preferred prefix for the address
```

#### `fm masked-emails update`

```
fm masked-emails update <ID> --state <STATE>

Options:
      --state <STATE>        enabled, disabled, or deleted
```

#### `fm contacts list`

```
fm contacts list [OPTIONS]

Options:
      --address-book <ID>    Filter by address book ID
  -n, --limit <N>            Max results (default 50)
      --json                 JSON output
```

#### `fm contacts search`

```
fm contacts search <QUERY> [OPTIONS]

Options:
  -n, --limit <N>            Max results (default 20)
      --json                 JSON output
```

#### `fm contacts create`

```
fm contacts create <NAME> [OPTIONS]

Options:
      --email <ADDR>         Email (repeatable: "work:a@b.com" or "a@b.com")
      --phone <NUMBER>       Phone (repeatable: "work:+1234" or "+1234")
      --company <TEXT>       Organization name
      --notes <TEXT>         Free-text notes
      --address-book <ID>   Target address book ID
```

#### `fm contacts update`

```
fm contacts update <CONTACT_ID> [OPTIONS]

At least one field is required.

Options:
      --name <TEXT>          Updated full name
      --email <ADDR>         Updated emails (replaces all; repeatable)
      --phone <NUMBER>       Updated phones (replaces all; repeatable)
      --company <TEXT>       Updated organization name
      --notes <TEXT>         Updated notes
```

#### `fm contacts delete`

```
fm contacts delete <CONTACT_ID>
```

#### `fm contacts address-books`

```
fm contacts address-books [OPTIONS]

Options:
      --json                 JSON output
```

## Implementation

### Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
console = "0.16"
indicatif = { version = "0.18", default-features = false, features = ["unicode-width"] }
inquire = { version = "0.9", default-features = false, features = ["console", "one-liners"] }
```

### Architecture

```
src/
  main.rs              CLI entry point, clap App definition
  cli/
    mod.rs             Cli struct, Command enum, OutputMode, top-level dispatch
    io.rs              Io struct (progress/done/warn/error/hint/data), TerminalGuard
    emails.rs          Email subcommand handlers
    mailboxes.rs       Mailbox subcommand handlers + resolution logic
    identities.rs      Identity subcommand handlers
    vacation.rs        Vacation subcommand handlers
    masked_emails.rs   Masked email subcommand handlers
    contacts.rs        Contact subcommand handlers + typed-value parsing
  actions/             (existing — shared with MCP)
  jmap/                (existing — shared with MCP)
  mcp/                 (existing — MCP server)
```

Each CLI subcommand handler:
1. Parses clap args into the corresponding action struct.
2. Resolves mailbox references (alias/fuzzy → ID) if needed.
3. Shows a spinner via `Io::progress()` during the JMAP call.
4. Calls `action.run(&ctx)`.
5. Formats the result through `Io` according to the output mode.

The `Context` creation (token → connect → session → account_id) is shared between
`fm mcp` and CLI commands.

### Error Handling

- `thiserror` enums for structured errors.
- `exit_on_err()` pattern: top-level catches `Result`, prints via `Io::error()`, exits
  with appropriate code.
- No panics in normal operation. `TerminalGuard` handles cleanup if one occurs.

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Startup error (missing token, connection failure) |
| 2 | Invalid arguments |
| 3 | API error (JMAP call failed) |

### Stdin Body Input

`fm emails send` reads body from stdin when `--body` is omitted, enabling piping:

```bash
echo "Hello, world!" | fm emails send --to user@example.com --subject "Test"
cat draft.html | fm emails send --to user@example.com --subject "Newsletter" --html
```
