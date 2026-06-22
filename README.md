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
fm ls                                    # list the inbox
fm read <email-id>                       # read a message
fm mv <email-id> archive                 # move (role alias, name, or id)
fm emails search -q "invoice"            # search
fm emails send --to a@b.com --subject Hi --body "Hello"
```

Full walkthroughs live in [`docs/guides/`](docs/guides/):

- **[Everyday use](docs/guides/regular-use.md)** — triage, search, mailboxes, contacts,
  vacation, masked email.
- **[Scripting](docs/guides/scripting.md)** — JSON output, `jq`, exit codes, incremental
  sync, automation.
- **[Backup](docs/guides/backup.md)** — export your entire account to `.eml`, resumably.

Every command, flag, and default: [`docs/reference/cli.md`](docs/reference/cli.md).

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
