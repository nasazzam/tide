#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

cat >"$tmp/tmux" <<'MOCK'
#!/usr/bin/env bash
printf '%q ' "$@" >>"$TIDE_TEST_LOG"
printf '\n' >>"$TIDE_TEST_LOG"
case "${1:-}" in
  has-session) exit 1 ;;
  new-session) printf '%%root-pane\n' ;;
  split-window) printf '%%test-pane\n' ;;
  list-sessions) printf 'tide-restored\t/tmp/project\t0\n' ;;
esac
MOCK
cat >"$tmp/tide-editor" <<'MOCK'
#!/usr/bin/env bash
exit 0
MOCK
chmod +x "$tmp/tmux" "$tmp/tide-editor"

export PATH="$tmp:$PATH"
export TIDE_TEST_LOG="$tmp/tmux.log"
"$root/bin/tide" --no-attach --image-protocol halfblocks pi -c :: claude --continue >/dev/null

grep -F 'pi\ -c' "$TIDE_TEST_LOG" >/dev/null
grep -F 'claude\ --continue' "$TIDE_TEST_LOG" >/dev/null

project="$tmp/project"
mkdir -p "$project"
cat >"$project/.tide.toml" <<'CONFIG'
[workspace]
layout = "agents-left"
shell_size = 20
agent_size = 40
session = "project"
restore = true
CONFIG
: >"$TIDE_TEST_LOG"
"$root/bin/tide" --project "$project" --no-attach pi >/dev/null
grep -F 'split-window -v -p 20' "$TIDE_TEST_LOG" >/dev/null
grep -F 'split-window -h -b -p 40' "$TIDE_TEST_LOG" >/dev/null
"$root/bin/tide" --list-sessions | grep -F 'tide-restored' >/dev/null

"$root/bin/tide" --version | grep -F 'tide 0.3.0' >/dev/null
"$root/bin/tide" --help | grep -F 'agent1 [args...] :: agent2' >/dev/null
printf 'launcher tests passed\n'
