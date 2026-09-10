# Omarchy plugin submission

- **Name:** TIDE Launcher
- **Plugin ID:** `org.tide.launcher`
- **Version:** `0.2.0`
- **Kind:** `bar-widget`
- **Author:** nasazzam
- **License:** MIT
- **Repository:** <https://github.com/nasazzam/tide>
- **Manifest:** <https://github.com/nasazzam/tide/blob/main/manifest.json>
- **Entry point:** <https://github.com/nasazzam/tide/blob/main/TideWidget.qml>

## Description

TIDE Launcher adds a small terminal button to the Omarchy bar. Pressing it
launches or focuses TIDE using Omarchy's supported TUI launcher:

```text
omarchy launch or focus tui --app-id=org.tide.TIDE tide
```

The widget complements TIDE's Omarchy application package; it does not install
packages or execute setup hooks.

## Installation

Install the application first:

```bash
curl -fsSL https://raw.githubusercontent.com/nasazzam/tide/main/packaging/omarchy/install.sh | bash
```

Then install and enable the optional bar widget:

```bash
omarchy plugin add https://github.com/nasazzam/tide.git --enable
```

## Review and security notes

- The manifest uses schema version 1 and declares only `bar-widget`.
- `omarchy plugin validate .` passes.
- The QML entry point contains no network access, storage access, background
  service, authentication access, or install hook.
- Its only action calls the documented Omarchy TUI launcher.
- The plugin is disabled by default unless the user passes `--enable` or enables
  it afterward.
- TIDE itself is MIT licensed and its release artifacts are checksum-protected.

Attach: `assets/tide-icon.png`
