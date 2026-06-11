#!/usr/bin/env sh
# Install the `spill` CLI from GitHub Releases.
# Override: SPILL_VERSION=vX.Y.Z, SPILL_BIN_DIR=/path/to/bin
set -eu

REPO="lgrossi/spill"
BIN_DIR="${SPILL_BIN_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Linux) plat="unknown-linux-gnu" ;;
  Darwin) plat="apple-darwin" ;;
  *) echo "unsupported OS: $os" >&2; exit 1 ;;
esac
case "$arch" in
  x86_64 | amd64) cpu="x86_64" ;;
  arm64 | aarch64) cpu="aarch64" ;;
  *) echo "unsupported arch: $arch" >&2; exit 1 ;;
esac
target="${cpu}-${plat}"

tag="${SPILL_VERSION:-}"
if [ -z "$tag" ]; then
  tag="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
fi
[ -n "$tag" ] || { echo "could not resolve latest spill release" >&2; exit 1; }

url="https://github.com/${REPO}/releases/download/${tag}/spill-${target}.tar.gz"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading spill ${tag} (${target})…"
curl -fsSL "$url" -o "$tmp/spill.tar.gz"
tar -C "$tmp" -xzf "$tmp/spill.tar.gz"
mkdir -p "$BIN_DIR"
install -m 0755 "$tmp/spill" "$BIN_DIR/spill"

echo "Installed spill to $BIN_DIR/spill"
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo "Note: add $BIN_DIR to your PATH" ;;
esac
