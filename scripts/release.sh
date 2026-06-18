#!/usr/bin/env bash
set -euo pipefail

# Cut and publish a fastermail release end-to-end.
#
# Usage: ./scripts/release.sh <version>
#
# Bumps Cargo.toml/Cargo.lock, cross-builds every target, patches the Homebrew
# formula in the shared tap, commits + tags, pushes, publishes the GitHub
# release, verifies the published sha, then commits + pushes the formula — all
# in one linear flow with no in-between rebuild that could drift the formula
# sha away from the uploaded artifact.
#
# The Homebrew tap (chakrit/homebrew-tap) is shared across tools, so unlike a
# per-project subtree it lives as a sibling clone. Point TAP_DIR at it; the
# default assumes it sits next to this repo.

if [ $# -ne 1 ]; then
  echo "Usage: ./scripts/release.sh <version>" >&2
  exit 1
fi

VERSION="${1#v}"
TAG="v$VERSION"
ARTIFACT="fm-aarch64-apple-darwin"
BINARY="target/dist/$ARTIFACT"
URL="https://github.com/chakrit/fastermail/releases/download/$TAG/$ARTIFACT"

TAP_DIR="${TAP_DIR:-../homebrew-tap}"
FORMULA="$TAP_DIR/Formula/fastermail.rb"

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Error: working tree is dirty. Commit or stash changes first." >&2
  exit 1
fi

if ! cargo set-version --help >/dev/null 2>&1; then
  echo "Error: cargo set-version not found. Run: cargo install cargo-edit" >&2
  exit 1
fi

if [ ! -f "$FORMULA" ]; then
  echo "Error: $FORMULA not found. Set TAP_DIR to your chakrit/homebrew-tap clone." >&2
  exit 1
fi

echo "==> Bumping to $TAG"
cargo set-version "$VERSION"

# Build after the version bump so the binary reports $VERSION (clap reads
# CARGO_PKG_VERSION at compile time). There is no build.rs, so HEAD movement
# does not affect the artifact — the formula sha and the upload stay in sync.
echo "==> Building all targets"
./scripts/build-all.sh

if [ ! -f "$BINARY" ]; then
  echo "Error: $BINARY missing after build." >&2
  exit 1
fi

EXPECTED_SHA=$(shasum -a 256 "$BINARY" | cut -d' ' -f1)
echo "==> Patching formula (sha256: $EXPECTED_SHA)"

sed -i '' "s|^  version .*|  version \"$VERSION\"|" "$FORMULA"
sed -i '' "s|^  url .*|  url \"$URL\"|" "$FORMULA"
sed -i '' "s|^  sha256 .*|  sha256 \"$EXPECTED_SHA\"|" "$FORMULA"

git add Cargo.toml Cargo.lock
git commit -m "$TAG"
git tag "$TAG"

echo "==> Pushing to gh"
git push gh main
git push gh "$TAG"

echo "==> Publishing GitHub release"
gh release create "$TAG" \
  --title "fastermail $TAG" \
  --generate-notes \
  target/dist/fm-*

echo "==> Verifying published artifact matches formula sha"
PUBLISHED_SHA=$(curl -fsSL "$URL" | shasum -a 256 | cut -d' ' -f1)
if [ "$PUBLISHED_SHA" != "$EXPECTED_SHA" ]; then
  echo "Error: published sha ($PUBLISHED_SHA) != expected ($EXPECTED_SHA)" >&2
  echo "  Release is broken — investigate before pushing the formula." >&2
  exit 1
fi
echo "==> sha verified: $EXPECTED_SHA"

echo "==> Pushing formula to tap ($TAP_DIR)"
git -C "$TAP_DIR" add Formula/fastermail.rb
git -C "$TAP_DIR" commit -m "fastermail $TAG"
git -C "$TAP_DIR" push gh main

echo
echo "==> Released: https://github.com/chakrit/fastermail/releases/tag/$TAG"
echo "==> Install:  brew install chakrit/tap/fastermail"
