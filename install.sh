#!/bin/sh
# Hymn Finder installer for macOS and Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/AbelHristodor/sda_manager/main/install.sh | sh
#
# Downloads the latest released `hymnal-gui` binary for your OS/arch and
# installs it to $BIN_DIR (default ~/.local/bin). Override the destination:
#
#   curl -fsSL .../install.sh | BIN_DIR=/usr/local/bin sh
set -eu

REPO="AbelHristodor/sda_manager"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Darwin)
    case "$arch" in
      arm64|aarch64) target="aarch64-apple-darwin" ;;
      *)
        echo "Error: unsupported macOS architecture '$arch'." >&2
        echo "Only Apple Silicon (arm64) binaries are published; build from source for Intel." >&2
        exit 1
        ;;
    esac
    ;;
  Linux)
    case "$arch" in
      x86_64|amd64) target="x86_64-unknown-linux-gnu" ;;
      *)
        echo "Error: unsupported Linux architecture '$arch'." >&2
        echo "Only x86_64 binaries are published; build from source for others." >&2
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Error: unsupported OS '$os'. Use install.ps1 on Windows." >&2
    exit 1
    ;;
esac

asset="hymnal-gui-${target}.tar.gz"
url="https://github.com/${REPO}/releases/latest/download/${asset}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading $asset ..."
if ! curl -fsSL "$url" -o "$tmp/$asset"; then
  echo "Error: download failed from $url" >&2
  echo "Make sure a release has been published for $REPO." >&2
  exit 1
fi

tar -xzf "$tmp/$asset" -C "$tmp"

mkdir -p "$BIN_DIR"
install -m 0755 "$tmp/hymnal-gui-${target}/hymnal-gui" "$BIN_DIR/hymnal-gui"

echo "Installed hymnal-gui to $BIN_DIR/hymnal-gui"

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *)
    echo
    echo "Note: $BIN_DIR is not on your PATH. Add this to your shell profile:"
    echo "  export PATH=\"$BIN_DIR:\$PATH\""
    ;;
esac

if [ "$os" = "Darwin" ]; then
  echo
  echo "macOS: the binary is unsigned. The first time, right-click hymnal-gui"
  echo "in Finder and choose Open (or run: xattr -d com.apple.quarantine \"$BIN_DIR/hymnal-gui\")."
fi
