# Architecture

## Project Structure

```
fastermail/
├── Cargo.toml
├── Dockerfile
├── docs/                    # Specs, references, guides (see docs/README.md)
├── .claude/skills/          # Coding convention files (ACE-managed symlinks)
├── src/
│   ├── lib.rs               # Library crate root (`fastermail`): exports the [lib] core
│   ├── main.rs              # `fm` binary: load .env, init logging, parse CLI, route MCP/CLI
│   │                        #   — a thin L3 caller on top of the library
│   ├── error.rs             # [lib] Single error enum for the crate
│   ├── json.rs              # [lib] Typed accessors over serde_json::Value (JSON Pointer paths)
│   ├── logging.rs           # [lib] Leveled stderr logging (FASTERMAIL_LOG) + log_* macros
│   ├── recorder.rs          # [lib] Request/response recording for test data capture
│   ├── config.rs            # [bin] Token resolution (env var → ~/.config/fastermail/config.toml)
│   ├── present.rs           # [bin] L3 Email presenters: view property lists + projection (CLI+MCP share)
│   ├── jmap/                # [lib] L0 transport + L1 typed JMAP accessors
│   │   ├── mod.rs           # JMAP module root
│   │   ├── client.rs        # HTTP client, session, call/call_one, blob download
│   │   ├── email.rs         # L1 Email accessors (get/query/changes/state/blob) + EmailEnumerator
│   │   └── types.rs         # JMAP request/response/session types, BlobId, back_reference
│   ├── testutil/            # [lib] MockJmap harness — gated behind the `testutil` feature
│   │   └── mock_jmap.rs     #   (httpmock-based; enabled for tests via a self dev-dependency)
│   ├── mcp/                 # [bin] MCP stdio server (L3 presenter for AI clients)
│   │   ├── mod.rs           # MCP module root
│   │   ├── types.rs         # JSON-RPC & MCP types (Request, Response, Tool, etc.)
│   │   ├── server.rs        # stdio read/write loop, dispatch to handlers
│   │   └── handler.rs       # Route tools/list and tools/call to actions
│   ├── cli/                 # [bin] Terminal front-end (L3 presenter for humans)
│   │   ├── mod.rs           # clap command tree + MCP/CLI routing
│   │   ├── io.rs            # Output modes (human/JSON/raw), TTY detection
│   │   ├── resolve.rs       # Mailbox resolution (role aliases, fuzzy match)
│   │   ├── emails.rs        # Email subcommands
│   │   ├── mailboxes.rs     # Mailbox subcommands
│   │   ├── identities.rs    # Identity subcommands
│   │   ├── vacation.rs      # Vacation subcommands
│   │   ├── masked_emails.rs # Masked email subcommands
│   │   └── contacts.rs      # Contact subcommands
│   └── actions/             # [bin] Unit-of-work structs (Action trait): JMAP calls (+ projection, pre-Email-migration)
│       ├── mod.rs           # Action trait + registry + Context
│       ├── email.rs         # Email action structs
│       ├── mailbox.rs       # Mailbox action structs
│       ├── vacation.rs      # Vacation response action structs
│       ├── masked_email.rs  # Masked email action structs
│       ├── identity.rs      # Identity action structs
│       └── contact.rs       # Contact action structs (JSContact flattening)
```

`[lib]` modules form the `fastermail` library (L0 transport + L1 JMAP accessors); `[bin]`
modules are the `fm` binary and MCP server — thin L3 callers that depend on the library.
This split is step 1 of the layering rearchitect. Email's read projection has migrated to
the L3 `present.rs` presenter (shared by CLI + MCP); the other five resources still project
in `actions/` and migrate in later steps (see
`docs/notes/2026-06-21-layering-rearchitect-plan.md`).

Calendars are out of scope — FastMail exposes no `jmap:calendars` capability (CalDAV only).

## Key Types

```rust
// Context passed to all actions
struct Context {
    jmap: JmapClient,
    account_id: String,
    recorder: Option<Recorder>,
}

// Action trait — unit-of-work pattern
trait Action {
    fn run(&self, ctx: &Context) -> Result<serde_json::Value>;
}

// Single crate-level error enum
enum Error {
    Io(std::io::Error),
    Http(ureq::Error),
    Json(serde_json::Error),
    Jmap { method: String, message: String },
    InvalidParams(String),
    MissingToken,
}
```

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `FASTMAIL_API_TOKEN` | yes | FastMail API token (`fmu1-*`) |
| `JMAP_SESSION_URL` | no | Override JMAP session URL (for testing) |
| `FASTERMAIL_RECORD_DIR` | no | Directory to write request/response recordings |
| `FASTERMAIL_LOG` | no | Log level: `error`, `warn`, `info`, `debug`, `trace` (default: `info`) |

At startup `main.rs` loads `.env` then `.env.local` (local overrides base) before reading
these. The API token also resolves from `~/.config/fastermail/config.toml` when
`FASTMAIL_API_TOKEN` is unset — the env var wins. See `config.rs`.

## Record Mode

Set `FASTERMAIL_RECORD_DIR` to a directory path to capture all traffic as JSON files.
Each interaction is saved as a timestamped file for later use as test data.

Files are named `{epoch}_{micros}_{type}_{method}.json`. Types:
- `mcp_req` — incoming MCP JSON-RPC message from client
- `mcp_resp` — outgoing MCP JSON-RPC response to client
- `jmap` — tool call arguments and JMAP result

Each file contains: `timestamp`, `type`, `method`, and full message/data.

## Dependencies

Guiding principle: minimize compile time.

| Crate           | Purpose                        | Why this one                    |
|-----------------|--------------------------------|---------------------------------|
| `serde`         | Serialization                  | Required, no alternative        |
| `serde_json`    | JSON parsing                   | Required, no alternative        |
| `ureq`          | HTTP client                    | Blocking, minimal, fast compile. No async runtime needed — stdio is inherently sequential |
| `thiserror`     | Error derive macros            | Tiny, zero runtime cost         |
| `clap`          | CLI arg parsing (derive)       | Derive-based command tree for the `fm` CLI |
| `inquire`       | Interactive prompts            | `fm setup` token wizard         |
| `indicatif`     | Progress spinners              | CLI human-mode feedback         |
| `console`       | Terminal styling               | Colors / status glyphs in human output |
| `toml`          | Config-file parsing            | Reads `~/.config/fastermail/config.toml` |

**No async runtime.** The MCP stdio server reads one message, processes it, writes a response.
There is no concurrency — `ureq` (blocking HTTP) is sufficient and avoids pulling in `tokio`
(~30s compile time penalty).

## Distribution

### Binary Targets

| Target                        | OS    | Arch    |
|-------------------------------|-------|---------|
| `x86_64-unknown-linux-gnu`    | Linux | x86_64  |
| `aarch64-unknown-linux-gnu`   | Linux | aarch64 |
| `x86_64-apple-darwin`         | macOS | x86_64  |
| `aarch64-apple-darwin`        | macOS | aarch64 |

Cross-compilation via `cross` or CI matrix.

### Docker

```dockerfile
FROM rust:1-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/fastermail /usr/local/bin/
ENTRYPOINT ["fastermail"]
```

Multi-arch image (`linux/amd64` + `linux/arm64`).

### Versioning

Version lives in `Cargo.toml`. Binary reads it via `env!("CARGO_PKG_VERSION")`.
Bump with `cargo set-version`. Tag releases as `v{version}`.
