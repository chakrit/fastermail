# FasterMail — Specification

FasterMail is an MCP (Model Context Protocol) server written in Rust that exposes FastMail's
APIs to AI assistants. It communicates over stdio using JSON-RPC 2.0.

**Phase 1 (JMAP):** Email, sending, vacation response, masked email — all available via JMAP today.
**Phase 2 (CardDAV/CalDAV):** Contacts and calendars — FastMail does not yet expose these via
JMAP (only CardDAV/CalDAV). When FastMail enables JMAP for contacts/calendars, Phase 2 tools
can migrate to JMAP.

## Design Decisions

- **Auth**: `FASTMAIL_API_TOKEN` environment variable, read at startup. Fail fast if unset.
- **Transport**: stdio only (newline-delimited JSON-RPC 2.0).
- **Dependencies**: Minimize for fast compile times. No MCP SDK crate.
- **Architecture**: Unit-of-work pattern — each MCP tool maps to an action struct with a
  `run(&self, ctx: &Context) -> Result<T>` method.
- **Distribution**: Linux + macOS binaries (x86_64 + aarch64) and Docker images.

## Spec Files

| File | Contents |
|------|----------|
| [protocol.md](protocol.md) | MCP protocol layer — JSON-RPC, handshake, error codes |
| [jmap.md](jmap.md) | JMAP client layer — session, auth, request format |
| [architecture.md](architecture.md) | Project structure, key types, deps, distribution |
| [tools/README.md](tools/README.md) | Tool index with one-line descriptions |
| [tools/*.md](tools/) | Individual tool specifications |
| [testing.md](testing.md) | Test strategy and mock server design |

## Startup Flow

1. Read `FASTMAIL_API_TOKEN` from env. If unset, print error to stderr and exit 1.
2. Fetch JMAP session from `https://api.fastmail.com/jmap/session`.
3. Extract `apiUrl` and primary `accountId`.
4. Enter stdio read loop — wait for `initialize` request.
5. Respond with capabilities, wait for `initialized` notification.
6. Enter main loop: read request → dispatch → write response.
7. On stdin EOF, clean up and exit 0.

## Error Strategy

- **Startup errors** (missing token, session fetch failure): print to stderr, exit 1.
- **Protocol errors** (malformed JSON, unknown method): JSON-RPC error response.
- **Tool errors** (JMAP call failed, invalid params): successful JSON-RPC response with
  `isError: true` and descriptive text content — lets the LLM retry with adjusted params.
- **One error enum** for the entire crate. No nested wrapper enums.
