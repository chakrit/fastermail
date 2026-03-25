# Architecture

## Project Structure

```
fastermail/
├── Cargo.toml
├── Dockerfile
├── specs/                   # Specification files
├── .skills/                 # Coding convention files
├── src/
│   ├── main.rs              # Entry point: read env, init session, run server loop
│   ├── error.rs             # Single error enum for the crate
│   ├── recorder.rs          # Request/response recording for test data capture
│   ├── mcp/
│   │   ├── mod.rs           # MCP module root
│   │   ├── types.rs         # JSON-RPC & MCP types (Request, Response, Tool, etc.)
│   │   ├── server.rs        # stdio read/write loop, dispatch to handlers
│   │   └── handler.rs       # Route tools/list and tools/call to actions
│   ├── jmap/
│   │   ├── mod.rs           # JMAP module root
│   │   ├── client.rs        # HTTP client, session management, JMAP request builder
│   │   └── types.rs         # JMAP request/response types, filter builders
│   └── actions/
│       ├── mod.rs           # Action trait + registry
│       ├── email.rs         # Email action structs
│       ├── mailbox.rs       # Mailbox action structs
│       ├── vacation.rs      # Vacation response action structs
│       ├── masked_email.rs  # Masked email action structs
│       └── identity.rs      # Identity action structs
```

Phase 2 action file (`contact.rs`) will be added when contacts
implementation begins. Calendars dropped (no JMAP capability).

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
