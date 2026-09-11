# Changelog

All notable changes to TIDE will be documented here. This project follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.4.4] - 2026-09-10

### Fixed

- Clicking a workspace pane or restoring a hidden pane now focuses that area.
- Modified shortcuts can no longer fall through to Explorer actions such as
  create, rename, move, or delete.
- The project root is explicitly protected from recursive deletion.

## [0.4.3] - 2026-09-10

### Changed

- Editor, Agent, and Terminal controls now truly hide and restore their panes
  instead of zooming them; remaining panes automatically consume freed space.
- Hidden panes preserve their running processes and return in the configured
  workspace layout.

## [0.4.2] - 2026-09-10

### Fixed

- Use tmux's portable `-l <percentage>%` pane sizing syntax so workspace creation works on tmux 3.4 instead of failing with `size missing`.

## [0.4.1] - 2026-09-10

### Changed

- Replaced the F6–F9 view controls with adjacent Ctrl+Shift+A/S/D/F/G home-row
  chords for Explorer, Editor, Diff, Agent, and Terminal.

## [0.4.0] - 2026-09-10

### Added

- F6–F9 workspace toggles for Explorer, Editor, Agent, and Terminal views.

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

[Unreleased]: https://github.com/nasazzam/tide/compare/v0.4.4...HEAD
[0.4.4]: https://github.com/nasazzam/tide/releases/tag/v0.4.4
[0.4.3]: https://github.com/nasazzam/tide/releases/tag/v0.4.3
[0.4.2]: https://github.com/nasazzam/tide/releases/tag/v0.4.2
[0.4.1]: https://github.com/nasazzam/tide/releases/tag/v0.4.1
[0.4.0]: https://github.com/nasazzam/tide/releases/tag/v0.4.0
[0.3.0]: https://github.com/nasazzam/tide/releases/tag/v0.3.0
[0.2.0]: https://github.com/nasazzam/tide/releases/tag/v0.2.0
