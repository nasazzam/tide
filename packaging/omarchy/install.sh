#!/usr/bin/env bash
# Install TIDE as an Omarchy terminal application.
set -euo pipefail

readonly repository="nasazzam/tide"
readonly raw="https://raw.githubusercontent.com/$repository/main"

command -v omarchy >/dev/null 2>&1 || {
  echo "TIDE Omarchy installer: Omarchy is required (https://omarchy.org)." >&2
  exit 1
}
command -v curl >/dev/null 2>&1 || {
  echo "TIDE Omarchy installer: curl is required." >&2
  exit 1
}

printf 'Installing TIDE dependencies…\n'
omarchy pkg add tmux git

printf 'Installing TIDE…\n'
curl -fsSL "$raw/install.sh" | bash

applications="$HOME/.local/share/applications"
icons="$HOME/.local/share/icons/hicolor/scalable/apps"
install -d "$applications" "$icons"
curl -fsSL "$raw/packaging/linux/tide.desktop" -o "$applications/tide.desktop"
curl -fsSL "$raw/assets/tide.svg" -o "$icons/tide.svg"
chmod 0644 "$applications/tide.desktop" "$icons/tide.svg"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$applications" >/dev/null 2>&1 || true

printf '\nTIDE is installed. Launch it from the app menu or run:\n'
printf '  omarchy launch or focus tui --app-id=org.tide.TIDE tide\n'
