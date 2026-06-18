# Releasing fastermail

End-to-end runbook for cutting a fastermail release. One script does the work
(`scripts/release.sh`, with `scripts/build-all.sh` as its cross-build primitive); this
doc covers the prereqs, the moving pieces, and what each step does.

## 1. Prerequisites

One-time host setup:

- `cargo install cargo-edit` — provides `cargo set-version` (used by `release.sh`).
- `cargo install cargo-zigbuild` (**≥ 0.23.0**) — cross-compiles the Linux/Windows
  targets. 0.23.0 is the first release whose zigcc wrapper filters the
  `--fix-cortex-a53-843419` link arg that rustc emits for aarch64; with zig 0.15.x, older
  versions fail to link `aarch64-unknown-linux-{gnu,musl}`.
- **Zig 0.14.x or 0.15.2** — Zig 0.16 has a known `ar` regression that breaks `ring`
  (rust-cross/cargo-zigbuild#433). `brew install zig` currently pulls 0.16; install a
  known-good version manually from <https://ziglang.org/download/> if your package
  manager is too new.
- `gh` CLI, authenticated against `chakrit/fastermail`.
- **macOS host** for the full matrix. Linux hosts can build the Linux/Windows targets
  only; the `*-apple-darwin` targets need Apple's toolchain.
- A clone of `chakrit/homebrew-tap`. `release.sh` patches and pushes the formula there.
  It defaults to `../homebrew-tap` (a sibling of this repo); override with `TAP_DIR`.
- The fastermail remote named `gh` (`release.sh` pushes to it):

  ```sh
  git remote rename origin gh   # if it is still the clone default
  ```

Optional: `cargo install sccache` to speed up repeat cross-builds.

## 2. Runbook

From a clean working tree on `main`:

```sh
./scripts/release.sh 0.1.0   # bump, build, patch formula, commit, tag, push, publish
```

Override the tap location if it is not a sibling clone:

```sh
TAP_DIR=~/src/homebrew-tap ./scripts/release.sh 0.1.0
```

## 3. What each script does

**`release.sh <version>`** — refuses to run with a dirty tree. In one linear flow:
calls `cargo set-version` to update `Cargo.toml` + `Cargo.lock`, runs `build-all.sh`,
computes the sha256 of `target/dist/fm-aarch64-apple-darwin`, sed-patches
`$TAP_DIR/Formula/fastermail.rb` (version, download URL, sha), commits and tags as
`v<version>`, pushes `main` and the tag to `gh`, runs `gh release create
v<ver> --generate-notes <binaries>`, re-downloads the published macOS arm64 artifact
and verifies its sha matches the formula (aborts if not), then commits and pushes the
formula from the tap clone.

The build happens **after** the version bump so the binary reports the right version
(clap reads `CARGO_PKG_VERSION` at compile time). fastermail has no `build.rs`, so HEAD
movement does not affect the artifact — the formula sha and the upload stay in sync.

**`build-all.sh`** — invoked by `release.sh`. Cross-builds all seven targets into
`target/dist/fm-<triple>` (`fm-<triple>.exe` for Windows). Builds `*-apple-darwin` with
plain `cargo build` + `SDKROOT` (Zig 0.14 can't resolve Apple frameworks); builds the
rest with `cargo zigbuild`. Builds each target group in a single multi-target
invocation; on group failure, retries per-target to isolate which one broke. Also
usable standalone for local cross-build smoke tests.

**`install.sh`** — end-user installer for macOS/Linux. Resolves the latest release via
GitHub's `/releases/latest/download/` redirect (no version marker needed), downloads the
matching binary, and installs to `~/.local/bin/fm`. Run via:

```sh
curl -fsSL https://raw.githubusercontent.com/chakrit/fastermail/main/scripts/install.sh | bash
```

## 4. Targets

All seven are built and uploaded to every GitHub release.

| Triple                       | Installer    |
| ---------------------------- | ------------ |
| `aarch64-apple-darwin`       | `install.sh` |
| `x86_64-apple-darwin`        | `install.sh` |
| `aarch64-unknown-linux-gnu`  | `install.sh` |
| `x86_64-unknown-linux-gnu`   | `install.sh` |
| `aarch64-unknown-linux-musl` | `install.sh` |
| `x86_64-unknown-linux-musl`  | `install.sh` |
| `x86_64-pc-windows-gnu`      | manual       |

## 5. Homebrew

The formula lives at `Formula/fastermail.rb` in the shared `chakrit/homebrew-tap` repo
(it also hosts other tools, so it is a sibling clone, not a subtree of this repo).
`release.sh` sed-patches three lines after the macOS arm64 binary is built:

- `version "<x.y.z>"`
- `url "https://github.com/chakrit/fastermail/releases/download/v<x.y.z>/fm-aarch64-apple-darwin"`
- `sha256 "<sha of the macOS arm64 binary>"`

After publishing the GitHub release, `release.sh` re-downloads the macOS arm64 artifact
and re-hashes it as a safety net — if the published sha doesn't match the formula, the
script aborts before pushing the formula. End users install with:

```sh
brew install chakrit/tap/fastermail
```

The formula carries only the macOS arm64 binary + sha. Other platforms are served by
`install.sh` and the raw GitHub release assets.

## 6. Open gaps

- **Checksums / signing** — only the Homebrew sha256 is computed. Publishing a
  `SHA256SUMS` file alongside the release assets and verifying it from `install.sh` would
  be a nice add.
