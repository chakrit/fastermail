# FasterMail — How It Works

FasterMail is a bridge between AI assistants (like Claude) and your FastMail account. It
lets an AI read, search, send, and manage your email through a structured tool interface.

It can also be used directly from the command line as `fm`.

## The Big Picture

```
┌─────────────┐     stdio      ┌─────────────┐     HTTPS      ┌─────────────┐
│   Claude /  │ ◄──JSON-RPC──► │  FasterMail  │ ◄───JMAP────► │  FastMail   │
│   AI Agent  │                │   (fm mcp)   │               │   Servers   │
└─────────────┘                └─────────────┘                └─────────────┘

┌─────────────┐                ┌─────────────┐     HTTPS      ┌─────────────┐
│   Terminal  │ ───commands──► │  FasterMail  │ ◄───JMAP────► │  FastMail   │
│   (human)   │ ◄──output───── │   (fm cli)   │               │   Servers   │
└─────────────┘                └─────────────┘                └─────────────┘
```

**Two modes, one binary:**
- `fm mcp` — speaks MCP (Model Context Protocol) over stdio for AI assistants
- `fm emails list`, `fm emails search`, etc. — human-friendly CLI

Both use the same FastMail JMAP connection underneath.

## What Can It Do?

### Email (read, search, send, organize)

| Action | CLI | What it does |
|--------|-----|-------------|
| List emails | `fm emails list -m Inbox` | Show recent emails from a mailbox |
| Search | `fm emails search -q "invoice"` | Full-text search across all mail |
| Read body | `fm emails get <id>` | Get the full text/HTML of an email |
| Send | `fm emails send --to ... --subject ...` | Compose and send |
| Move | `fm emails move <id> --to <mailbox>` | Move between mailboxes |
| Delete | `fm emails delete <id>` | Move to Trash (or `--permanent`) |
| Flag | `fm emails flag <id> --flag seen` | Mark as read, flagged, etc. |

### Mailboxes (folders)

| Action | CLI |
|--------|-----|
| List all | `fm mailboxes list` |
| Create | `fm mailboxes create "Projects"` |
| Rename | `fm mailboxes rename <id> "New Name"` |
| Delete | `fm mailboxes delete <id>` |

### Vacation Auto-Reply

| Action | CLI |
|--------|-----|
| Check status | `fm vacation get` |
| Turn on | `fm vacation set --enabled --subject "OOO" --text-body "..."` |
| Turn off | `fm vacation set --disabled` |

### Sending Identities

| Action | CLI |
|--------|-----|
| List | `fm identities list` |

Shows your configured "From" addresses (name + email).

### Masked Email (FastMail-specific)

Masked emails are disposable addresses that forward to your real inbox. Great for signups.

| Action | CLI |
|--------|-----|
| List all | `fm masked-emails list` |
| Create | `fm masked-emails create --domain example.com` |
| Disable | `fm masked-emails update <id> --state disabled` |
| Delete | `fm masked-emails update <id> --state deleted` |

## How Authentication Works

FasterMail uses a **FastMail API token** to authenticate. You create one in FastMail's
settings under **Settings → Privacy & Security → API tokens**.

The token is provided via:
1. `FASTMAIL_API_TOKEN` environment variable (preferred), or
2. `~/.config/fastermail/config.toml` config file

On startup, FasterMail connects to FastMail's JMAP session endpoint, discovers your
account, and is ready to go.

## Protocols

### JMAP (JSON Mail Access Protocol)

FastMail's native API. It's like a modern, JSON-based replacement for IMAP. FasterMail
speaks JMAP to FastMail's servers for all operations.

Key JMAP capabilities used:
- **Core** — session discovery, method calls
- **Mail** — emails, mailboxes
- **Submission** — sending email, identities
- **Vacation Response** — auto-reply settings
- **Masked Email** — FastMail's disposable address feature

### MCP (Model Context Protocol)

Anthropic's protocol for connecting AI assistants to external tools. FasterMail implements
an MCP server that advertises 15 tools. The AI discovers available tools, then calls them
by name with JSON arguments.

The MCP server communicates over stdin/stdout using newline-delimited JSON-RPC 2.0. All
logging goes to stderr so it never interferes with the protocol.

## Output Formats

CLI commands default to human-readable tables. Add `--json` for machine-readable output
(matches the MCP tool response format), or `--raw` for the full JMAP response.

## Architecture

FasterMail is a single Rust binary with minimal dependencies:

- **serde/serde_json** — JSON handling
- **ureq** — HTTP client (no async runtime needed)
- **thiserror** — error types
- **clap** — CLI argument parsing

No async, no tokio, no MCP SDK crate — just straightforward synchronous Rust. This keeps
compile times fast and the binary small.

Internally, each operation is an "action" — a struct that takes parameters and produces a
JSON result. The MCP server and CLI are both thin frontends that create action structs and
call `.run()`.
