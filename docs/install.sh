#!/bin/sh
set -eu

REPO="wu-hongjun/macvmtop"
BIN="macvmtop"
INSTALL_DIR="${MACVMTOP_INSTALL_DIR:-$HOME/.local/bin}"
GITHUB_BASE="https://github.com/$REPO"

say() {
  printf '%s\n' "$*"
}

fail() {
  printf 'macvmtop install: %s\n' "$*" >&2
  exit 1
}

case "$(uname -s)" in
  Darwin) ;;
  *) fail "macvmtop currently supports macOS only" ;;
esac

case "$(uname -m)" in
  arm64 | aarch64) TARGET="aarch64-apple-darwin" ;;
  x86_64) TARGET="x86_64-apple-darwin" ;;
  *) fail "unsupported macOS architecture: $(uname -m)" ;;
esac

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT INT TERM

mkdir -p "$INSTALL_DIR"

ASSET="$BIN-$TARGET.tar.gz"
URL="$GITHUB_BASE/releases/latest/download/$ASSET"

if command -v curl >/dev/null 2>&1; then
  if curl -fsSL "$URL" -o "$TMP_DIR/$ASSET"; then
    tar -xzf "$TMP_DIR/$ASSET" -C "$TMP_DIR"
    CANDIDATE="$TMP_DIR/$BIN"

    if [ ! -f "$CANDIDATE" ]; then
      CANDIDATE="$(find "$TMP_DIR" -type f -name "$BIN" 2>/dev/null | head -n 1 || true)"
    fi

    if [ -n "$CANDIDATE" ] && [ -f "$CANDIDATE" ]; then
      install -m 0755 "$CANDIDATE" "$INSTALL_DIR/$BIN"
      say "macvmtop installed to $INSTALL_DIR/$BIN"
      say "Add $INSTALL_DIR to PATH if it is not already there."
      exit 0
    fi

    fail "release asset did not contain a $BIN executable"
  fi
fi

say "No prebuilt release asset found for $TARGET; falling back to cargo install from git." >&2

if ! command -v cargo >/dev/null 2>&1; then
  fail "cargo is required for source install. Install Rust from https://rustup.rs/ and rerun this script."
fi

cargo install --git "$GITHUB_BASE.git" --locked --force "$BIN"

say "macvmtop installed with cargo."
say "Add $HOME/.cargo/bin to PATH if it is not already there."
