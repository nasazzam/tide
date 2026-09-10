#!/usr/bin/env bash
# Install TIDE from a GitHub release, with a source-build fallback.
set -euo pipefail

readonly REPOSITORY="nasazzam/tide"
prefix="${TIDE_INSTALL_DIR:-$HOME/.local}"
version="${TIDE_VERSION:-latest}"
force_source=0

usage() {
  cat <<'EOF'
TIDE installer

Usage:
  curl -fsSL https://raw.githubusercontent.com/nasazzam/tide/main/install.sh | bash
  ./install.sh [--prefix PATH] [--version TAG] [--from-source]

Options:
  --prefix PATH       Installation prefix (default: ~/.local)
  --version TAG       Release tag such as v0.2.0 (default: latest)
  --from-source       Build with Cargo instead of downloading a binary
  -h, --help          Show this help
EOF
}

fail() { printf 'tide installer: %s\n' "$*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || fail "$1 is required"; }

while (( $# )); do
  case "$1" in
    --prefix) (( $# >= 2 )) || fail "--prefix requires a path"; prefix=$2; shift 2 ;;
    --version) (( $# >= 2 )) || fail "--version requires a tag"; version=$2; shift 2 ;;
    --from-source) force_source=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) fail "unknown option: $1" ;;
  esac
done

need curl
need tar
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

os=$(uname -s)
arch=$(uname -m)
case "$os:$arch" in
  Linux:x86_64|Linux:amd64) target=x86_64-unknown-linux-gnu ;;
  Darwin:x86_64|Darwin:amd64) target=x86_64-apple-darwin ;;
  Darwin:arm64|Darwin:aarch64) target=aarch64-apple-darwin ;;
  *) force_source=1; target="" ;;
esac

resolve_version() {
  if [[ $version != latest ]]; then
    [[ $version == v* ]] || version="v$version"
    return
  fi
  local effective
  effective=$(curl -fsSL -o /dev/null -w '%{url_effective}' \
    "https://github.com/$REPOSITORY/releases/latest") || return 1
  version=${effective##*/}
  [[ $version == v* ]]
}

install_release() {
  resolve_version || return 1
  local name="tide-${version}-${target}.tar.gz"
  local base="https://github.com/$REPOSITORY/releases/download/$version"
  printf 'Downloading TIDE %s for %s…\n' "$version" "$target"
  curl -fL --retry 3 "$base/$name" -o "$tmp/$name" || return 1
  if curl -fsSL "$base/$name.sha256" -o "$tmp/$name.sha256"; then
    if command -v sha256sum >/dev/null 2>&1; then
      (cd "$tmp" && sha256sum -c "$name.sha256")
    elif command -v shasum >/dev/null 2>&1; then
      expected=$(awk '{print $1}' "$tmp/$name.sha256")
      actual=$(shasum -a 256 "$tmp/$name" | awk '{print $1}')
      [[ $actual == "$expected" ]] || fail "release checksum verification failed"
    fi
  fi
  mkdir -p "$tmp/release"
  tar -xzf "$tmp/$name" -C "$tmp/release"
  install -d "$prefix/bin"
  install -m 0755 "$tmp/release/tide" "$prefix/bin/tide"
  install -m 0755 "$tmp/release/tide-editor" "$prefix/bin/tide-editor"
  if [[ $os == Linux && -f $tmp/release/tide.desktop && -f $tmp/release/tide.svg ]]; then
    install -Dm 0644 "$tmp/release/tide.desktop" "$prefix/share/applications/tide.desktop"
    install -Dm 0644 "$tmp/release/tide.svg" "$prefix/share/icons/hicolor/scalable/apps/tide.svg"
  fi
}

install_source() {
  need cargo
  printf 'Building TIDE %s from source…\n' "$version"
  local archive_url
  if [[ $version == latest ]]; then
    archive_url="https://github.com/$REPOSITORY/archive/refs/heads/main.tar.gz"
  else
    archive_url="https://github.com/$REPOSITORY/archive/refs/tags/$version.tar.gz"
  fi
  curl -fL --retry 3 "$archive_url" -o "$tmp/source.tar.gz"
  mkdir -p "$tmp/source"
  tar -xzf "$tmp/source.tar.gz" -C "$tmp/source" --strip-components=1
  "$tmp/source/scripts/install.sh" --prefix "$prefix"
}

if (( force_source )) || ! install_release; then
  printf 'A prebuilt release is unavailable; falling back to a source build.\n' >&2
  install_source
fi

printf '\nTIDE installed successfully.\n'
printf '  Binary: %s/bin/tide\n' "$prefix"
command -v tmux >/dev/null 2>&1 || printf '  Note: install tmux to use the full workspace.\n'
case ":$PATH:" in
  *":$prefix/bin:"*) printf '  Start:  tide --help\n' ;;
  *) printf '  Add %s/bin to PATH, then run tide --help.\n' "$prefix" ;;
esac
