# FasterMail — Specification

This file is a pointer to the full specification, now split into individual files for
agent-friendly access.

## Spec Directory

See [`specs/`](specs/) for the full specification:

- [`specs/README.md`](specs/README.md) — Overview, design decisions, startup flow, error strategy
- [`specs/protocol.md`](specs/protocol.md) — MCP protocol layer (JSON-RPC, handshake, error codes)
- [`specs/jmap.md`](specs/jmap.md) — JMAP client layer (session, auth, request format)
- [`specs/architecture.md`](specs/architecture.md) — Project structure, key types, deps, distribution
- [`specs/tools/README.md`](specs/tools/README.md) — Tool index with one-line descriptions
- [`specs/tools/*.md`](specs/tools/) — Individual tool specifications (one file per tool)
- [`specs/testing.md`](specs/testing.md) — Test strategy and mock server design
