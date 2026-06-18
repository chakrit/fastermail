# No Integration Tests
- **Date:** 2026-06-18
- **PR:** manual (commit `e114f8f`)
- **Status:** accepted

## Decision
FasterMail does not ship integration tests (no `tests/integration.rs`, no
binary-spawning end-to-end harness). The previously-specced `testing.md §6`
section was removed. Coverage stays at the unit level against `MockJmap`.

## Rationale
An MCP server that spawns the binary and drives a full `initialize` →
`tools/list` → `tools/call` handshake is the conventional thing to add, and a
future agent auditing the repo will flag its absence as a gap (one did — the
session-18 spec-impl analysis surfaced it as the single "real" gap). This entry
exists to stop that re-litigation.

Why not add them anyway:
- The unit suite already exercises every branch worth testing — dispatch,
  param validation, response projection, error mapping — through `MockJmap`,
  which simulates FastMail's HTTP layer in-process. The handshake/dispatch logic
  is covered by `mcp/handler.rs` and `mcp/types.rs` tests.
- The end-to-end path the integration harness would add (spawn process, pipe
  JSON-RPC over stdio, assert on stdout) mostly tests stdio plumbing and the
  process lifecycle — low defect-density surface for the wiring cost.
- The owner explicitly deprioritized it and asked that it not be recommended.

If this reverses (e.g. a regression slips through that only an end-to-end test
would catch), write a new dated decision that links back and supersedes this one.
