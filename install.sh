#!/bin/sh
# Olenro CLI/TUI one-line installer.
#
#   curl -fsSL https://raw.githubusercontent.com/lianhr12/olenro/master/install.sh | sh
#
# Detects your OS/arch, downloads the matching prebuilt `olenro` binary from the
# latest `cli-v*` GitHub Release, and installs it to ~/.local/bin (override with
# OLENRO_INSTALL_DIR). Pin a version with OLENRO_VERSION=cli-v2.0.0.
#
# Windows users: use `cargo install --git https://github.com/horace68/olenro olenro-cli`
# or download the .zip from the Releases page.

set -eu

REPO="lianhr12/olenro"
BIN="olenro"
INSTALL_DIR="${OLENRO_INSTALL_DIR:-$HOME/.local/bin}"

err() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || err "missing required command: $1"
}

need uname
need tar

# --- Detect target triple --------------------------------------------------
os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Darwin) os_part="apple-darwin" ;;
  Linux)  os_part="unknown-linux-gnu" ;;
  *) err "unsupported OS: $os (Windows: use 'cargo install --git https://github.com/$REPO olenro-cli')" ;;
esac

case "$arch" in
  x86_64 | amd64)  arch_part="x86_64" ;;
  arm64 | aarch64) arch_part="aarch64" ;;
  *) err "unsupported architecture: $arch" ;;
esac

target="${arch_part}-${os_part}"

# --- Resolve the release tag ----------------------------------------------
if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -qO- "$1"; }
else
  err "need curl or wget"
fi

tag="${OLENRO_VERSION:-}"
if [ -z "$tag" ]; then
  # Pick the newest release whose tag starts with cli-v (skips desktop v* tags).
  tag="$(fetch "https://api.github.com/repos/$REPO/releases" \
    | grep '"tag_name"' \
    | grep 'cli-v' \
    | head -n 1 \
    | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"
fi
[ -n "$tag" ] || err "could not find a cli-v* release (set OLENRO_VERSION to pin one)"

asset="${BIN}-${target}.tar.gz"
url="https://github.com/$REPO/releases/download/$tag/$asset"

printf 'Installing %s %s (%s)\n' "$BIN" "$tag" "$target"

# --- Download + extract + install -----------------------------------------
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fetch "$url" > "$tmp/$asset" || err "download failed: $url"
tar -xzf "$tmp/$asset" -C "$tmp" || err "extract failed (asset for $target may not exist in $tag)"

bin_path="$(find "$tmp" -type f -name "$BIN" | head -n 1)"
[ -n "$bin_path" ] || err "binary '$BIN' not found inside archive"

mkdir -p "$INSTALL_DIR"
install -m 0755 "$bin_path" "$INSTALL_DIR/$BIN" 2>/dev/null \
  || { cp "$bin_path" "$INSTALL_DIR/$BIN" && chmod 0755 "$INSTALL_DIR/$BIN"; }

printf '\n✓ Installed to %s/%s\n' "$INSTALL_DIR" "$BIN"

case ":$PATH:" in
  *":$INSTALL_DIR:"*) printf 'Run: %s\n' "$BIN" ;;
  *)
    printf '\n%s is not on your PATH. Add it:\n' "$INSTALL_DIR"
    printf '  export PATH="%s:$PATH"\n' "$INSTALL_DIR"
    printf 'Then run: %s\n' "$BIN"
    ;;
esac
