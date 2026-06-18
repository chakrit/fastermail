# Session: release tooling + first release (2026-06-18)

## Done

- **Dead-code cleanup** — wired in `TerminalGuard` (RAII terminal cleanup, now
  instantiated in `main`) and routed fatal errors through `Io::error` (made an
  associated fn). Both were built-but-unwired; warnings gone, 152 tests green.
- **Release tooling** ported from `ace-rs/ace`: `scripts/{build-all,release,install}.sh`
  + runbook at `docs/guides/releasing.md`. Adaptations vs ace: shared tap →
  external clone via `TAP_DIR` (not a subtree, since `chakrit/homebrew-tap` also
  hosts kue); install via GitHub `releases/latest/download/` redirect (no domain /
  `latest` marker); build-after-bump (no `build.rs` to pin a git hash).
- **MIT license** — real `LICENSE` + `Cargo.toml` `license` field + formula `license`.
- **README** — added Homebrew + install-script options.
- **Remote** renamed `origin` → `gh`.
- **Cut v0.1.0** — all 7 targets published:
  <https://github.com/chakrit/fastermail/releases/tag/v0.1.0>. Tap formula patched
  with real url/sha and pushed (`chakrit/homebrew-tap`). `brew install
  chakrit/tap/fastermail` is live.

## Toolchain gotcha (resolved)

zig 0.15.2's linker rejects `--fix-cortex-a53-843419` (rustc emits it for aarch64),
breaking `aarch64-unknown-linux-{gnu,musl}` links. Fixed by `cargo-zigbuild ≥ 0.23.0`,
whose zigcc wrapper filters the arg. Pinned in `releasing.md` prereqs.

## Open follow-ups

- Verify `brew install chakrit/tap/fastermail` end-to-end (formula live but the
  install path is untested; macOS arm64 only).
- `fm` on Windows is shipped but its runtime is untested.
- `releasing.md` §6 gap: no `SHA256SUMS` / signing — only the Homebrew sha is computed.

## State

Clean tree, `main` + tag `v0.1.0` pushed to `gh`. Implementation feature-complete
vs `docs/spec/`. Nothing in flight.
