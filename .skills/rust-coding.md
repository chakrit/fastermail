---
name: rust-coding
description: >
  Rust-specific coding conventions — covers error handling, Option/Result idioms, serde
  patterns, testing, and dependency management. TRIGGER when: working on Rust files (.rs),
  Cargo.toml, or any Rust project. DO NOT TRIGGER for: non-Rust languages.
---

# Rust Coding

## Toolchain

- Prefer stable Rust. Respect any existing `rust-toolchain.toml` or project settings.
- Do not install or switch to nightly unless the user explicitly asks. If a crate or feature
  appears to require nightly, ask before proceeding.
- **macOS Xcode prerequisite**: If any `cargo` invocation fails with a linker error mentioning
  `xcodebuild` or Xcode license, **stop immediately**. Almost every cargo command that matters
  will fail in this state. Tell the user to open **Xcode.app** (which handles license acceptance
  and component installation for whatever version they have). Do not attempt workarounds —
  `xcode-select --install` no longer works standalone and defers to Xcode.app anyway.

## Dependencies

- Check crate versions/metadata/docs via `cargo search` or `cargo info`, not web searches.
- Prioritize fast compile times when choosing crates.

## Metrics

- After `cargo build` or `cargo test`, report the compilation time shown in the output. Flag
  regressions — if a change noticeably increases compile time, question whether a lighter
  approach exists.

## Coding Style

- Use monadic combinators (`map`, `and_then`, `unwrap_or`, etc.) on `Option`/`Result` where
  they simplify over match/if chains.
- **Imports vs qualified paths** — if a type is used more than once, import it. For single-use,
  inline qualified paths are fine when short (2–3 segments like `std::io::Error`), but import
  when the path hurts readability (4+ segments). Always check existing `use` statements
  first — if the module is already imported, extend it rather than writing a new inline path.

Error handling:
- **One error enum per module/folder** — don't create wrapper enums that just re-wrap the same
  `io::Error` / `serde` errors. Consolidate into the folder-level enum.
- **NEVER use `.unwrap()`** — always propagate errors with `?` or handle explicitly.
  No exceptions.
- In tests, use `.expect("reason")` instead of `.unwrap()` so failures always have context.

## Serde Patterns

- **Loading vs validation**: Serde handles parsing only. Validation is a separate pass in code
  after loading — not by relying on serde's required-field behavior.
- All config/DTO structs use `#[derive(Default)]` + `#[serde(default)]` at the struct level.
  No per-field `#[serde(default)]`.
- **Prefer default**: Prefer `String` (defaults to `""`) over `Option<String>` when there is no
  meaningful distinction between absent and empty. Same for `Vec<T>` (empty vec) vs
  `Option<Vec<T>>`. Reserve `Option<T>` for cases where absence carries distinct semantics
  from the zero value.

## Testing

- Unit tests are inline `#[cfg(test)] mod tests` in the same file.
- Don't test that serde serializes/deserializes correctly — that's testing the crate, not your
  code.

## Versioning & Release

- Version lives in `Cargo.toml`'s `version` field. Clap and other crates read it at compile
  time via `env!("CARGO_PKG_VERSION")` — no runtime config, no duplication.
- **Bumping**: use `cargo set-version` (from `cargo-edit`): `cargo set-version 0.2.0` or
  `cargo set-version --bump patch`. Parses TOML properly — never sed/awk Cargo.toml.
  Install: `cargo install cargo-edit`.
- **Flow**: bump version → commit → tag `v{version}` → build → release. The binary's
  `--version` matches because the version was baked in at compile time from the committed
  Cargo.toml.
- No additional versioning tools needed. `cargo-release` and `cargo-dist` are overkill for
  projects without CI-driven release pipelines.
