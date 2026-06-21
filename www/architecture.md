# Architecture

fastermail is a single Rust codebase with two front-ends over one core:

- **`fm`** — a human-facing terminal CLI.
- **MCP server** (`fm mcp`) — a stdio JSON-RPC server exposing the same operations
  as tools for an AI agent.

Both talk to **FastMail over JMAP** (RFC 8620/8621). It covers email, mailboxes,
contacts, identities, vacation response, and masked email. Calendars are out of
scope (FastMail exposes only CalDAV, no JMAP).

## Guiding idea — a transparent translation layer

fastermail deliberately adds **no email-processing value**; it's a thin pass-through
between you and the FastMail servers. Two consequences shape the whole API:

1. **Vocabulary mirrors JMAP/FastMail** — the same method and field names, so terms
   stay searchable across the JMAP specs (`keyword`, `anchor`, `state`,
   `created`/`updated`/`destroyed`, `blobId`/`downloadUrl`, …).
2. **Ergonomic Rust sugar is an optional layer on top** of the direct JMAP
   accessors — never a replacement. You can drop to the faithful JMAP calls at any
   time.

## Layout

- `src/jmap/` — the JMAP client (session discovery, method calls) and wire types.
- `src/actions/` — one unit-of-work struct per operation (the operations layer).
- `src/cli/` — clap subcommands (the terminal front-end).
- `src/mcp/` — the JSON-RPC stdio server and tool dispatch (the AI front-end).

One crate-wide error type; MCP tool errors return `isError: true` rather than
failing the JSON-RPC call; dependencies are kept minimal for fast compiles.

## Direction

fastermail is moving toward a **`lib` + `bin`** split: the library will expose the
JMAP primitives (direct accessors plus optional iterator-style sugar like anchored
enumeration and change feeds), and the `fm` binary and MCP server become thin
callers. That lets other Rust programs build on the same primitives — e.g. a backup
tool composed from raw-`.eml` download + paginated enumeration + incremental
`changes` — without fastermail itself owning that policy. See
`docs/decisions/` for the full design.
