#!/usr/bin/env bash
set -euo pipefail

required_environment() {
  local name="$1"
  local value="${!name:-}"
  if [ -z "$value" ]; then
    printf 'orchestrator live integration: missing %s\n' "$name" >&2
    exit 1
  fi
}

required_environment ORCHESTRATOR_BIN
required_environment ORCHESTRATOR_TEST_CITY_TOML

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
isolated_script="${ORCHESTRATOR_ISOLATED_TEST_SCRIPT:-$script_dir/orchestrator-isolated-gc-test.sh}"
model_sequence="${ORCHESTRATOR_LIVE_CODEX_MODELS:-gpt-5.4-nano gpt-5.4-mini}"
tmux_socket_prefix="${ORCHESTRATOR_LIVE_TMUX_SOCKET_PREFIX:-orchestrator-live}"
agent_run_timeout_seconds="${ORCHESTRATOR_AGENT_RUN_TIMEOUT_SECONDS:-240}"
bead_close_timeout_seconds="${ORCHESTRATOR_BEAD_CLOSE_TIMEOUT_SECONDS:-120}"
mail_timeout_seconds="${ORCHESTRATOR_MAIL_TIMEOUT_SECONDS:-120}"
live_root="$(mktemp -d "${TMPDIR:-/tmp}/orchestrator-live.XXXXXX")"

cleanup() {
  set +e
  if [ "${ORCHESTRATOR_KEEP_TEST_ROOT:-}" = "1" ]; then
    printf 'orchestrator live integration: kept test root %s\n' "$live_root" >&2
  else
    rm -rf "$live_root"
  fi
}
trap cleanup EXIT

safe_socket_segment() {
  printf '%s' "$1" | tr -c 'A-Za-z0-9_-' '-'
}

render_fixture_for_model() {
  local source_fixture="$1"
  local rendered_fixture="$2"
  local codex_model="$3"
  local tmux_socket="$4"

  python3 - "$source_fixture" "$rendered_fixture" "$codex_model" "$tmux_socket" <<'PY'
import pathlib
import sys

source_path = pathlib.Path(sys.argv[1])
rendered_path = pathlib.Path(sys.argv[2])
codex_model = sys.argv[3]
tmux_socket = sys.argv[4]

rendered_lines = []
current_table = ""
session_table_seen = False
session_socket_seen = False

def table_name(line: str) -> str:
    stripped = line.strip()
    if stripped.startswith("[") and stripped.endswith("]"):
        return stripped
    return ""

for line in source_path.read_text().splitlines():
    next_table = table_name(line)
    if next_table and current_table == "[session]" and not session_socket_seen:
        rendered_lines.append(f'socket = "{tmux_socket}"')
        session_socket_seen = True

    if next_table:
        current_table = next_table
        if current_table == "[session]":
            session_table_seen = True

    if current_table == "[session]" and line.strip().startswith("socket = "):
        rendered_lines.append(f'socket = "{tmux_socket}"')
        session_socket_seen = True
        continue

    if line.strip().startswith("model = "):
        rendered_lines.append(f'model = "{codex_model}"')
    else:
        rendered_lines.append(line)

if current_table == "[session]" and not session_socket_seen:
    rendered_lines.append(f'socket = "{tmux_socket}"')
    session_socket_seen = True

if not session_table_seen:
    insert_index = 0
    current_table = ""
    for index, line in enumerate(rendered_lines):
        next_table = table_name(line)
        if next_table:
            if current_table == "[workspace]":
                insert_index = index
                break
            current_table = next_table
        elif current_table == "[workspace]":
            insert_index = index + 1
    session_block = [
        "[session]",
        f'socket = "{tmux_socket}"',
        "",
    ]
    if insert_index == 0 or rendered_lines[insert_index - 1].strip():
        session_block.insert(0, "")
    rendered_lines[insert_index:insert_index] = session_block

rendered_text = "\n".join(rendered_lines) + "\n"
for known_model in ("gpt-5.4-nano", "gpt-5.4-mini"):
    rendered_text = rendered_text.replace(known_model, codex_model)

rendered_path.write_text(rendered_text)
PY
}

model_related_failure() {
  local log_path="$1"
  grep -Eiq \
    'model_not_found|invalid[ _-]*model|unknown model|unsupported model|model[ _-]*(is )?not (available|found|supported)|model .*does not exist' \
    "$log_path"
}

attempt_model() {
  local codex_model="$1"
  local attempt_root="$live_root/$codex_model"
  local rendered_fixture="$attempt_root/deterministic-city.toml"
  local log_path="$attempt_root/run.log"
  local socket_model_segment
  local tmux_socket

  mkdir -p "$attempt_root"
  socket_model_segment="$(safe_socket_segment "$codex_model")"
  tmux_socket="$(safe_socket_segment "$tmux_socket_prefix")-$socket_model_segment-$$"
  render_fixture_for_model "$ORCHESTRATOR_TEST_CITY_TOML" "$rendered_fixture" "$codex_model" "$tmux_socket"

  printf 'orchestrator live integration: trying Codex model %s with tmux socket %s\n' \
    "$codex_model" "$tmux_socket"

  set +e
  ORCHESTRATOR_CODEX_PROVIDER_MODE=real \
    ORCHESTRATOR_EXPECTED_CODEX_MODEL="$codex_model" \
    ORCHESTRATOR_TEST_CITY_TOML="$rendered_fixture" \
    ORCHESTRATOR_TEST_ROOT="$attempt_root/gc" \
    ORCHESTRATOR_PRESERVE_TEST_ROOT=1 \
    ORCHESTRATOR_AGENT_RUN_TIMEOUT_SECONDS="$agent_run_timeout_seconds" \
    ORCHESTRATOR_BEAD_CLOSE_TIMEOUT_SECONDS="$bead_close_timeout_seconds" \
    ORCHESTRATOR_MAIL_TIMEOUT_SECONDS="$mail_timeout_seconds" \
    bash "$isolated_script" >"$log_path" 2>&1
  local status="$?"
  set -e

  cat "$log_path"
  return "$status"
}

read -r -a codex_models <<<"$model_sequence"
if [ "${#codex_models[@]}" -eq 0 ]; then
  printf 'orchestrator live integration: no Codex models configured\n' >&2
  exit 1
fi

for model_index in "${!codex_models[@]}"; do
  codex_model="${codex_models[$model_index]}"
  log_path="$live_root/$codex_model/run.log"

  set +e
  attempt_model "$codex_model"
  status="$?"
  set -e

  if [ "$status" -eq 0 ]; then
    printf 'orchestrator live integration: model %s passed\n' "$codex_model"
    exit 0
  fi

  if [ "$model_index" -eq 0 ] && model_related_failure "$log_path"; then
    printf 'orchestrator live integration: model %s failed with a model-related error; trying fallback\n' \
      "$codex_model" >&2
    continue
  fi

  if [ "$model_index" -eq 0 ]; then
    printf 'orchestrator live integration: model %s failed for a non-model reason; not retrying\n' \
      "$codex_model" >&2
  else
    printf 'orchestrator live integration: model %s failed; no further fallback configured\n' \
      "$codex_model" >&2
  fi
  exit "$status"
done

printf 'orchestrator live integration: all configured models failed\n' >&2
exit 1
