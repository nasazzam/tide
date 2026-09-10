#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ./scripts/install.sh [--prefix PATH] [--debug] [--skip-build]

Default prefix: $HOME/.local
EOF
}

prefix="${HOME}/.local"
profile=release
skip_build=0
while (( $# )); do
  case "$1" in
    --prefix) prefix=${2:?missing prefix}; shift 2 ;;
    --debug) profile=debug; shift ;;
    --skip-build) skip_build=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'install.sh: unknown option: %s\n' "$1" >&2; exit 2 ;;
  esac
done

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
command -v cargo >/dev/null 2>&1 || { echo 'install.sh: Rust/Cargo is required: https://rustup.rs' >&2; exit 1; }
command -v tmux >/dev/null 2>&1 || echo 'install.sh: warning: install tmux to use the workspace launcher' >&2

if (( ! skip_build )); then
  if [[ $profile == release ]]; then
    cargo build --manifest-path "$root/Cargo.toml" --locked --release
  else
    cargo build --manifest-path "$root/Cargo.toml" --locked
  fi
fi

install -d "$prefix/bin"
install -m 0755 "$root/target/$profile/tide-editor" "$prefix/bin/tide-editor"
install -m 0755 "$root/bin/tide" "$prefix/bin/tide"

if [[ $(uname -s) == Linux ]]; then
  install -Dm 0644 "$root/packaging/linux/tide.desktop" "$prefix/share/applications/tide.desktop"
  install -Dm 0644 "$root/assets/tide.svg" "$prefix/share/icons/hicolor/scalable/apps/tide.svg"
fi

printf 'Installed TIDE to %s/bin\n' "$prefix"
case ":$PATH:" in
  *":$prefix/bin:"*) ;;
  *) printf 'Add %s/bin to PATH, then run: tide --help\n' "$prefix" ;;
esac
