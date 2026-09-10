# TIDE feature guide

A detailed guide to TIDE's editor, workspace, agents, language intelligence, and
project tools. For installation and command-line usage, return to the
[main README](../README.md).

## Table of contents

- [Project Explorer](#project-explorer)
- [Native editor](#native-editor)
- [Agent panes](#agent-panes)
- [Quick Open and project search](#quick-open-and-project-search)
- [Live file synchronization](#live-file-synchronization)
- [Git integration](#git-integration)
- [Live diff review with Hunk](#live-diff-review-with-hunk)
- [Media and binary previews](#media-and-binary-previews)
- [LSP diagnostics and completion](#lsp-diagnostics-and-completion)
- [File management](#file-management)

## Project Explorer

Browse the complete project tree without leaving the keyboard. Expand and
collapse directories, toggle hidden files, scroll deep paths horizontally, or
open files with a click. The tree refreshes without restarting TIDE.

## Native editor

Open multiple tabs with language-aware syntax highlighting, mouse selection,
undo and redo, familiar navigation keys, safe saving, and a read-only hex view
for unknown binary formats. Files up to 50 MiB can be opened.

## Agent panes

TIDE launches any CLI command in a dedicated tmux pane. Use one agent, split the
available area between multiple agents with `::`, or leave the pane empty. Agent
arguments and session flags are preserved exactly.

Workspace layouts and pane sizes can be selected through command-line options
or [project configuration](../README.md#project-configuration).

### Fast workspace toggles

TIDE uses adjacent home-row chords for distraction-free navigation:

- <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd> shows or hides Explorer.
- <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd> focuses and expands Editor.
- <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>D</kbd> opens the live Diff review.
- <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>F</kbd> focuses and expands Agent.
- <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd> focuses and expands Terminal.

Press the Editor, Agent, or Terminal chord again to restore the complete
workspace. Switching directly from one focused pane to another also works. The
bindings are conditional: outside a TIDE workspace window, tmux passes these
chords through normally. A terminal with extended-key support is required to
distinguish Ctrl+Shift from Ctrl; TIDE enables tmux's extended-key handling.

## Quick Open and project search

Press <kbd>Ctrl</kbd>+<kbd>P</kbd> and type part of a path for fuzzy filename
lookup. Prefix the query with `%` to search file contents across the project;
selecting a result jumps directly to its matching line.

## Live file synchronization

TIDE watches every open file. Clean buffers reload automatically when an agent
or another tool edits them. If the disk and an unsaved buffer both change,
TIDE blocks the save and asks you to resolve the conflict, preventing accidental
data loss.

## Git integration

The Explorer refreshes repository status and decorates modified, added,
deleted, and untracked paths. Parent directories inherit status when they
contain changed files. The editor header continuously summarizes changed files,
diff hunks, additions, and deletions.

## Live diff review with Hunk

The **Δ DIFF** control at the top-right of the editor opens the current
changeset in [Hunk](https://hunk.dev), a review-first terminal diff viewer built
for agent-authored changes. Click it or press <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>D</kbd> to open
a live `hunk diff --watch` review. Hunk reloads while an agent edits files and
supports multi-file review, split or unified layouts, hunk navigation, and
inline agent annotations.

Inside a TIDE workspace, the review opens in a dedicated tmux window named
`diff`; leave it with Hunk's quit command and switch tmux windows normally.
Editor-only mode temporarily hands the terminal to Hunk and restores TIDE when
the review closes.

Hunk is optional. Install it using one of its supported methods:

```bash
brew install hunk
# or: mise use -g hunk
# or: npm install -g hunkdiff
```

Agents can participate in a live review through Hunk's own skill and session
controls. Run `hunk skill path` and ask the agent to load that skill for review
annotations and change explanations.

## Media and binary previews

TIDE supports Sixel, Kitty graphics, iTerm2 images, and true-color half-blocks.
It automatically selects a safe protocol, or you can choose one explicitly:

```bash
tide --image-protocol sixel pi
tide --image-protocol kitty pi
tide --image-protocol halfblocks pi
```

Optional helpers extend format support:

- `pdftoppm` — PDFs
- `magick` — SVG, AVIF, HEIC, JXL, and font files
- `ffmpegthumbnailer` — video thumbnails

## LSP diagnostics and completion

TIDE speaks the Language Server Protocol directly. It auto-detects
`rust-analyzer`, `gopls`, `pylsp`, and `typescript-language-server` from common
project markers, or accepts any server command through `.tide.toml`. Diagnostics
appear in the gutter and on the active line. Press <kbd>Ctrl</kbd>+<kbd>Space</kbd>
to request completion, use the arrow keys, and accept with Enter or Tab.

Install at least one server for automatic detection:

| Project | Language server |
|---|---|
| Rust | `rustup component add rust-analyzer` |
| Go | `go install golang.org/x/tools/gopls@latest` |
| Python | `pipx install python-lsp-server` |
| JavaScript / TypeScript | `npm install -g typescript typescript-language-server` |

Other servers work by setting `lsp.command` explicitly.

## File management

The Explorer can create files and directories, rename or move entries, and
recursively delete them. All destinations are constrained to the project root,
existing paths are never overwritten, and destructive deletion requires typing
`delete` explicitly.

See the [keyboard reference](../README.md#keyboard-reference) for all shortcuts.
