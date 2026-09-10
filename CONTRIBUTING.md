# Contributing to TIDE

Thank you for helping improve TIDE.

## Development setup

1. Install Rust 1.88+, tmux, and Git.
2. Fork and clone the repository.
3. Create a focused branch from `main`.
4. Run `make check` before opening a pull request.

```bash
cargo build
cargo run --bin tide-editor -- .
PATH="$PWD/target/debug:$PATH" ./bin/tide --help
make check
```

## Pull requests

- Keep changes focused and explain the user-facing reason.
- Add or update tests when practical.
- Update README and CHANGELOG for behavior changes.
- Ensure formatting, Clippy, tests, and shell checks pass.
- Avoid unrelated generated files or dependency upgrades.

## Commit messages

Use short imperative subjects, for example:

```text
Add configurable agent pane width
Fix reload conflict after atomic save
```

## Reporting bugs

Use the bug-report template and include the TIDE version, OS, terminal, tmux
version, reproduction steps, expected result, and relevant logs. Never include
API keys, access tokens, or private prompts.
