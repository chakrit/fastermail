#!/usr/bin/env bash
set -euo pipefail

# Install fastermail (the `fm` binary) from GitHub releases.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/chakrit/fastermail/main/scripts/install.sh | bash
#
# Resolves the latest release via GitHub's /releases/latest/download/ redirect
# (no version marker needed) and installs to ~/.local/bin/fm.

REPO="chakrit/fastermail"
INSTALL_DIR="${HOME}/.local/bin"

# --- Detect platform ----------------------------------------------------------

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin) TRIPLE_OS="apple-darwin" ;;
  Linux)  TRIPLE_OS="unknown-linux-gnu" ;;
  *)
    echo "Error: unsupported OS: $OS"
    exit 1
    ;;
esac

case "$ARCH" in
  aarch64|arm64) TRIPLE_ARCH="aarch64" ;;
  x86_64)        TRIPLE_ARCH="x86_64" ;;
  *)
    echo "Error: unsupported architecture: $ARCH"
    exit 1
    ;;
esac

TARGET="${TRIPLE_ARCH}-${TRIPLE_OS}"

# --- Download binary ----------------------------------------------------------

ASSET_URL="https://github.com/${REPO}/releases/latest/download/fm-${TARGET}"
TMPFILE="$(mktemp)"
trap 'rm -f "$TMPFILE"' EXIT

echo "Downloading latest fm (${TARGET})..."
curl -fsSL -o "$TMPFILE" "$ASSET_URL"

if [ ! -s "$TMPFILE" ]; then
  echo "Error: download failed or produced empty file."
  exit 1
fi

# --- Install ------------------------------------------------------------------

chmod +x "$TMPFILE"
mkdir -p "$INSTALL_DIR"
mv "$TMPFILE" "${INSTALL_DIR}/fm"

echo "Installed fm to ${INSTALL_DIR}/fm"

if ! echo ":${PATH}:" | grep -q ":${INSTALL_DIR}:"; then
  echo ""
  echo "Note: ${INSTALL_DIR} is not on your PATH."
  echo "Add it with:  export PATH=\"${INSTALL_DIR}:\$PATH\""
fi
