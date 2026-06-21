# Usage

## Install

From source (until published to crates.io):

```sh
cargo build --release      # binary at target/release/fm
# or, to put it on PATH:
cargo install --path .     # installs ~/.cargo/bin/fm
```

## Authenticate

fastermail reads a FastMail API token (create one at FastMail → Settings →
Privacy & Security → API tokens, with JMAP access). Resolution order:

1. `FASTMAIL_API_TOKEN` environment variable — also populated from a `.env` /
   `.env.local` file loaded **relative to the current working directory**.
2. `~/.config/fastermail/config.toml` (`[auth] token = "..."`), cwd-independent.
   Write it interactively with `fm setup` (the file must be mode `0600`).

Quickest start: `fm setup`.

## CLI

Global flags (any subcommand): `--json` (machine output; automatic when stdout is
not a TTY), `--raw` (raw JMAP response).

- **Email:** `fm emails list -m <mailbox> [-n N] [--include-body]`,
  `fm emails search [-q kw] [--from] [--to] [--subject] [-m mb] [--has-attachment]
  [--after YYYY-MM-DD] [--before …] [-n N] [--include-body]`,
  `fm emails get <id> [--format text|html|both]`,
  `fm emails move <ids…> --to <mb>`, `fm emails flag <ids…> --flag <f> [--unset]`,
  `fm emails send …`, `fm emails delete <ids…> [--permanent]`.
  Triage shortcuts: `fm ls [mailbox]`, `fm read <id>`, `fm mv <ids…> <mailbox>`.
- **Mailboxes:** `fm mailboxes list|create|rename|delete`.
- **Contacts:** `fm contacts list|get|search|create|update|delete` (+ address books).
- **Identities:** `fm identities list`.
- **Vacation:** `fm vacation get|set`.
- **Masked email:** `fm masked-emails list|create|update`.
- **Config:** `fm config`, `fm setup`.

## MCP server

The same binary serves MCP over stdio: `fm mcp`. Register it with an MCP host — for
Claude Code, a project `.mcp.json`:

```json
{
  "mcpServers": {
    "fastermail": {
      "command": "/absolute/path/to/fm",
      "args": ["mcp"],
      "env": { "FASTMAIL_API_TOKEN": "${FASTMAIL_API_TOKEN}" }
    }
  }
}
```

The server reads the token at startup; set it via the `env` block above, or run
`fm setup` first so it's read from `~/.config/fastermail/config.toml` (the MCP
server's working directory is not guaranteed, so don't rely on `.env.local` for it).
