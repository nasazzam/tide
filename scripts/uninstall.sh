#!/usr/bin/env bash
set -euo pipefail
prefix="${HOME}/.local"
if [[ ${1:-} == --prefix ]]; then
  prefix=${2:?missing prefix}
elif (( $# )); then
  echo "Usage: ./scripts/uninstall.sh [--prefix PATH]" >&2
  exit 2
fi
rm -f \
  "$prefix/bin/tide" \
  "$prefix/bin/tide-editor" \
  "$prefix/share/applications/tide.desktop" \
  "$prefix/share/icons/hicolor/scalable/apps/tide.svg"
printf 'Removed TIDE from %s\n' "$prefix"
