# FasterMail

Command-line tool for [FastMail](https://www.fastmail.com). Read, search, organize, and
send email from your terminal. Also works as an
[MCP server](https://modelcontextprotocol.io/) for AI assistants like Claude.

## Install

**Homebrew** (macOS, Apple Silicon):

```bash
brew install chakrit/tap/fastermail
```

**Install script** (macOS / Linux) — downloads the latest release binary to
`~/.local/bin/fm`:

```bash
curl -fsSL https://raw.githubusercontent.com/chakrit/fastermail/main/scripts/install.sh | bash
```

**From source** (any platform with a Rust toolchain):

```bash
cargo install --git https://github.com/chakrit/fastermail
```

All three give you the `fm` binary.

## Configure

1. Create a FastMail API token at
   [Settings > Privacy & Security > API tokens](https://app.fastmail.com/settings/security/tokens).
   Required scope: **JMAP access**.

2. Run setup:

```bash
fm setup
```

This saves your token to `~/.config/fastermail/config.toml` (permissions `0600`).

Alternatively, set the environment variable:

```bash
export FASTMAIL_API_TOKEN=fmu1-...
```

## Usage

```bash
# List recent emails (inbox is the default)
fm ls

# List emails from a specific mailbox
fm ls sent
fm ls archive

# Read an email
fm read <email-id>

# Move emails to a mailbox (fuzzy matching works)
fm mv <email-id> trash
fm mv <email-id> proj        # matches "Projects"

# Search
fm emails search -q "invoice"
fm emails search --from alice@example.com --after 2025-01-01

# Flag/unflag
fm emails flag <email-id> --flag seen
fm emails flag <email-id> --flag flagged --unset

# Send
fm emails send --to user@example.com --subject "Hello" --body "Hi there"
echo "Hello" | fm emails send --to user@example.com --subject "Piped"

# Delete (moves to Trash)
fm emails delete <email-id>
fm emails delete <email-id> --permanent
```

### Mailboxes

```bash
fm mailboxes list
fm mailboxes create "Projects"
fm mailboxes rename <mailbox-id> "New Name"
fm mailboxes delete <mailbox-id>
```

### Other commands

```bash
fm identities list              # sending addresses
fm vacation get                 # auto-reply status
fm vacation set --enabled --subject "OOO" --text-body "Back Monday"
fm masked-emails list           # disposable addresses
fm masked-emails create --domain example.com
fm config                       # show current config
```

## Output formats

| Flag | When | Description |
|------|------|-------------|
| *(default)* | TTY | Colors, spinners, tables |
| `--json` | non-TTY or flag | Machine-readable JSON |
| `--raw` | flag | Full JMAP response |

Non-TTY stdout (e.g. piping to `jq`) automatically switches to JSON.

## MCP server

FasterMail can act as an MCP tool server for AI assistants:

```bash
fm mcp
```

Add to your Claude Desktop or Claude Code config:

```json
{
  "mcpServers": {
    "fastermail": {
      "command": "fm",
      "args": ["mcp"]
    }
  }
}
```

## License

MIT
