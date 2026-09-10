# Changelog

All notable changes to TIDE will be documented here. This project follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.3.0] - 2026-09-10

### Added

- Live Hunk diff review from the editor header or `Ctrl+D`.
- Continuously refreshed changed-file, hunk, addition, and deletion counts.
- Dedicated tmux diff window with watch mode for agent-authored changes.
- Foreground Hunk review support when running the native editor without tmux.
- Documentation for Hunk's live agent review and annotation workflow.

## [0.2.0] - 2026-09-09

### Added

- Native Rust project Explorer and editor with tabs, search, syntax highlighting,
  Git decorations, external-change detection, and rich media previews.
- Agent-ready tmux workspace with direct argument forwarding and multi-agent
  panes separated by `::`.
- Project-level `.tide.toml` configuration for workspace, editor, and
  language-server settings.
- Configurable classic, agents-left, and agents-bottom layouts and pane sizing.
- Native Language Server Protocol diagnostics and completion.
- Safe Explorer actions for creating, renaming, moving, and deleting entries.
- Named tmux workspace restoration and session discovery.
- Linux, macOS, Windows, WSL, Arch, Omarchy, Homebrew, and Winget packaging.
- Checksum-verifying release installer and cross-platform CI.

[Unreleased]: https://github.com/nasazzam/tide/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/nasazzam/tide/releases/tag/v0.3.0
[0.2.0]: https://github.com/nasazzam/tide/releases/tag/v0.2.0
