#![deny(warnings)]

//! fastermail library core — the L0 transport, L1 JMAP accessors, and supporting
//! primitives (errors, JSON helpers, logging, recorder). The `fm` binary and the MCP
//! handler are thin L3 callers on top of this; external consumers depend on it directly.

#[macro_use]
pub mod logging;
pub mod error;
pub mod jmap;
pub mod json;
pub mod recorder;

/// HTTP-mocking test harness. Gated behind the `testutil` feature so neither the harness
/// nor its `httpmock` dependency reach a release build; the package enables it for its own
/// tests via a self dev-dependency (see `Cargo.toml`).
#[cfg(feature = "testutil")]
pub mod testutil;
