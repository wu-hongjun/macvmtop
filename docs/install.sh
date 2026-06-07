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

path_contains() {
  case ":$PATH:" in
    *":$1:"*) return 0 ;;
    *) return 1 ;;
  esac
}

shell_name() {
  basename "${SHELL:-sh}"
}

path_profile() {
  case "$(shell_name)" in
    zsh) printf '%s\n' "$HOME/.zshrc" ;;
    bash) printf '%s\n' "$HOME/.bash_profile" ;;
    fish) printf '%s\n' "$HOME/.config/fish/config.fish" ;;
    *) printf '%s\n' "$HOME/.profile" ;;
  esac
}

path_export_line() {
  case "$(shell_name)" in
    fish) printf 'fish_add_path -U %s\n' "$1" ;;
    *) printf 'export PATH="%s:$PATH"\n' "$1" ;;
  esac
}

append_path_to_profile() {
  PROFILE="$1"
  DIR="$2"

  mkdir -p "$(dirname "$PROFILE")"
  touch "$PROFILE"

  if grep -F "$DIR" "$PROFILE" >/dev/null 2>&1; then
    return 0
  fi

  {
    printf '\n'
    printf '# Added by macvmtop installer\n'
    path_export_line "$DIR"
  } >>"$PROFILE"
}

offer_path_update() {
  DIR="$1"

  if path_contains "$DIR"; then
    say "$DIR is already on PATH."
    return 0
  fi

  PROFILE="$(path_profile)"
  say "$DIR is not on PATH."
  say "To use macvmtop from a new shell, add this to $PROFILE:"
  say "  $(path_export_line "$DIR")"

  if [ "${MACVMTOP_NO_PATH_PROMPT:-}" = "1" ]; then
    return 0
  fi

  if [ -r /dev/tty ] && [ -w /dev/tty ]; then
    printf 'Add macvmtop to PATH in %s now? [y/N] ' "$PROFILE" >/dev/tty
    IFS= read -r ANSWER </dev/tty || ANSWER=""
    case "$ANSWER" in
      y | Y | yes | YES)
        append_path_to_profile "$PROFILE" "$DIR"
        say "Updated $PROFILE."
        say "Restart your shell, or run this now:"
        say "  $(path_export_line "$DIR")"
        ;;
      *)
        say "Skipped PATH update."
        ;;
    esac
  else
    say "No interactive terminal is available, so the installer did not edit your shell profile."
  fi
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
      offer_path_update "$INSTALL_DIR"
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
offer_path_update "$HOME/.cargo/bin"
