#!/usr/bin/env bash
set -euo pipefail

required_environment() {
  local name="$1"
  local value="${!name:-}"
  if [ -z "$value" ]; then
    printf 'orchestrator integration: missing %s\n' "$name" >&2
    exit 1
  fi
}

required_environment ORCHESTRATOR_BIN
required_environment ORCHESTRATOR_TEST_CITY_TOML

codex_provider_mode="${ORCHESTRATOR_CODEX_PROVIDER_MODE:-real}"
expected_codex_model="${ORCHESTRATOR_EXPECTED_CODEX_MODEL:-gpt-5.3-codex-spark}"

platform_home="$(
  python3 - <<'PY'
import os
import pwd

print(pwd.getpwuid(os.getuid()).pw_dir)
PY
)"
real_home="${HOME:-$platform_home}"
if [ -n "$platform_home" ] && [ "$real_home" != "$platform_home" ]; then
  real_home="$platform_home"
fi

root="$(mktemp -d "${TMPDIR:-/tmp}/orchestrator-gc.XXXXXX")"
supervisor_pid=""
orchestrator_pid=""

cleanup() {
  set +e
  if [ -n "$orchestrator_pid" ] && kill -0 "$orchestrator_pid" 2>/dev/null; then
    kill "$orchestrator_pid" >/dev/null 2>&1
    wait "$orchestrator_pid" >/dev/null 2>&1
  fi
  if [ -n "$supervisor_pid" ]; then
    run_isolated gc supervisor stop --wait >/dev/null 2>&1
    kill "$supervisor_pid" >/dev/null 2>&1
    wait "$supervisor_pid" >/dev/null 2>&1
  fi
  if [ "${ORCHESTRATOR_KEEP_TEST_ROOT:-}" = "1" ]; then
    printf 'orchestrator integration: kept test root %s\n' "$root" >&2
  else
    rm -rf "$root"
  fi
}
trap cleanup EXIT

gc_home="$root/gc-home"
runtime_dir="$root/runtime"
temporary_dir="$root/tmp"
city_dir="$root/city"
state_path="$root/orchestrator.redb"
bin_dir="$root/bin"
git_config_global="$gc_home/gitconfig"

mkdir -p "$gc_home" "$runtime_dir" "$temporary_dir" "$city_dir" "$bin_dir"
touch "$git_config_global"

seed_supervisor_config() {
  local port
  port="$(
    python3 - <<'PY'
import socket

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
    listener.bind(("127.0.0.1", 0))
    print(listener.getsockname()[1])
PY
  )"
  cat >"$gc_home/supervisor.toml" <<EOF
[supervisor]
bind = "127.0.0.1"
port = $port
EOF
}

install_host_command_shims() {
  for command_name in systemctl launchctl; do
    cat >"$bin_dir/$command_name" <<'EOF'
#!/usr/bin/env sh
exit 0
EOF
    chmod +x "$bin_dir/$command_name"
  done
}

install_codex_shim() {
  cat >"$bin_dir/codex" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

expected_model="${ORCHESTRATOR_EXPECTED_CODEX_MODEL:-gpt-5.3-codex-spark}"
city_path="${GC_CITY_PATH:-${GC_CITY:-}}"
target_agent="${GC_AGENT:-${GC_ALIAS:-${GC_SESSION_NAME:-}}}"
target_agent="${target_agent##*/}"

if [ -z "$city_path" ] || [ -z "$target_agent" ]; then
  printf 'codex shim: missing city path or agent identity\n' >&2
  env | sort >&2
  exit 1
fi

argument_log="$city_path/.gc/cascade-test/codex-arguments.tsv"
mkdir -p "$(dirname "$argument_log")"
printf '%s\t%s\n' "$target_agent" "$*" >>"$argument_log"

case " $* " in
  *" --model $expected_model "* | *" -m $expected_model "*) ;;
  *)
    printf 'codex shim: expected model %s in args: %s\n' "$expected_model" "$*" >&2
    exit 1
    ;;
esac

case " $* " in
  *" model_reasoning_effort=low "*) ;;
  *)
    printf 'codex shim: expected low reasoning effort in args: %s\n' "$*" >&2
    exit 1
    ;;
esac

bash "$city_path/agents/cascade-test-agent/run.sh" "$target_agent"
EOF
  chmod +x "$bin_dir/codex"
}

run_isolated() {
  env -i \
    PATH="$bin_dir:$PATH" \
    HOME="$real_home" \
    USER="${USER:-nixbld}" \
    LOGNAME="${LOGNAME:-nixbld}" \
    SHELL="${SHELL:-/bin/sh}" \
    LANG="${LANG:-C.UTF-8}" \
    TMPDIR="$temporary_dir" \
    GC_HOME="$gc_home" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    DOLT_ROOT_PATH="$gc_home" \
    GIT_CONFIG_GLOBAL="$git_config_global" \
    ORCHESTRATOR_EXPECTED_CODEX_MODEL="$expected_codex_model" \
    BEADS_DOLT_AUTO_START=0 \
    "$@"
}

exec_isolated() {
  exec env -i \
    PATH="$bin_dir:$PATH" \
    HOME="$real_home" \
    USER="${USER:-nixbld}" \
    LOGNAME="${LOGNAME:-nixbld}" \
    SHELL="${SHELL:-/bin/sh}" \
    LANG="${LANG:-C.UTF-8}" \
    TMPDIR="$temporary_dir" \
    GC_HOME="$gc_home" \
    XDG_RUNTIME_DIR="$runtime_dir" \
    DOLT_ROOT_PATH="$gc_home" \
    GIT_CONFIG_GLOBAL="$git_config_global" \
    ORCHESTRATOR_EXPECTED_CODEX_MODEL="$expected_codex_model" \
    BEADS_DOLT_AUTO_START=0 \
    "$@"
}

seed_dolt_identity() {
  mkdir -p "$gc_home/.dolt"
  printf '{"user.name":"gc-test","user.email":"gc-test@test.local"}' \
    >"$gc_home/.dolt/config_global.json"
}

start_isolated_supervisor() {
  (exec_isolated gc supervisor run) >"$root/supervisor.log" 2>&1 &
  supervisor_pid="$!"

  local deadline=$((SECONDS + 30))
  until run_isolated gc supervisor status 2>/dev/null | grep -q "Supervisor is running"; do
    if ! kill -0 "$supervisor_pid" 2>/dev/null; then
      cat "$root/supervisor.log" >&2
      exit 1
    fi
    if [ "$SECONDS" -ge "$deadline" ]; then
      cat "$root/supervisor.log" >&2
      exit 1
    fi
    sleep 0.2
  done
}

install_test_agents() {
  mkdir -p \
    "$city_dir/agents/cascade-test-agent" \
    "$city_dir/agents/cascade-mail-recorder" \
    "$city_dir/.gc/cascade-test/gates"

  cat >"$city_dir/agents/cascade-test-agent/run.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

city_path="${GC_CITY_PATH:-${GC_CITY:-}}"
target_agent="${1:-${GC_AGENT:-}}"
target_agent="${target_agent##*/}"
if [ -z "$city_path" ] || [ -z "$target_agent" ]; then
  printf 'cascade test agent: missing GC city or agent identity\n' >&2
  exit 1
fi

run_gc() {
  gc --city "$city_path" "$@"
}

test_state="$city_path/.gc/cascade-test"
mkdir -p "$test_state/gates"

deadline=$((SECONDS + 30))
bead_id=""
while [ -z "$bead_id" ]; do
  for candidate_agent in "$target_agent" cascade-tester-one cascade-tester-two cascade-tester-three; do
    bead_id="$(
      run_gc bd ready \
        --metadata-field "gc.routed_to=$candidate_agent" \
        --unassigned \
        --json \
        --limit 1 \
        | jq -r '.[0].id // empty'
    )"
    if [ -n "$bead_id" ]; then
      target_agent="$candidate_agent"
      break
    fi
  done
  if [ "$SECONDS" -ge "$deadline" ]; then
    printf 'cascade test agent %s: no routed bead became ready\n' "$target_agent" >&2
    exit 1
  fi
  sleep 0.2
done

printf '%s\t%s\n' "$target_agent" "$bead_id" >>"$test_state/agent-runs.tsv"

gate_path="$test_state/gates/$bead_id"
deadline=$((SECONDS + 60))
while [ ! -e "$gate_path" ]; do
  if [ "$SECONDS" -ge "$deadline" ]; then
    printf 'cascade test agent %s: gate did not open for %s\n' "$target_agent" "$bead_id" >&2
    exit 1
  fi
  sleep 0.2
done

run_gc bd close "$bead_id" --reason "cascade test agent $target_agent completed $bead_id"
EOF

  cat >"$city_dir/agents/cascade-test-agent/prompt.template.md" <<'EOF'
# Cascade Test Agent

Run this command exactly once, then exit:

```sh
bash "$GC_CITY/agents/cascade-test-agent/run.sh" "$GC_AGENT"
```
EOF

  cat >"$city_dir/agents/cascade-mail-recorder/run.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

city_path="${GC_CITY_PATH:-${GC_CITY:-}}"
if [ -z "$city_path" ]; then
  printf 'cascade mail recorder: missing GC city\n' >&2
  exit 1
fi

mkdir -p "$city_path/.gc/cascade-test"
gc --city "$city_path" mail inbox mayor >"$city_path/.gc/cascade-test/mayor-inbox.txt" 2>&1 || true
EOF

  chmod +x \
    "$city_dir/agents/cascade-test-agent/run.sh" \
    "$city_dir/agents/cascade-mail-recorder/run.sh"
}

run_orchestrator_once() {
  run_isolated "$ORCHESTRATOR_BIN" \
    --city "$city_dir" \
    --state "$state_path" \
    --idle-sleep-seconds 1 \
    --once
}

start_orchestrator() {
  (exec_isolated "$ORCHESTRATOR_BIN" \
    --city "$city_dir" \
    --state "$state_path" \
    --idle-sleep-seconds 1) \
    >"$root/orchestrator.log" 2>&1 &
  orchestrator_pid="$!"
}

stop_orchestrator() {
  local stopped_orchestrator_pid="$1"
  if kill -0 "$stopped_orchestrator_pid" 2>/dev/null; then
    kill "$stopped_orchestrator_pid" >/dev/null 2>&1
    wait "$stopped_orchestrator_pid" >/dev/null 2>&1 || true
  fi
  if [ "${orchestrator_pid:-}" = "$stopped_orchestrator_pid" ]; then
    orchestrator_pid=""
  fi
}

create_cascade_beads() {
  run_isolated gc --city "$city_dir" bd create "cascade step one" \
    --id oit-s1 \
    --labels cascade-chain \
    --metadata '{"cascade_target_agent":"cascade-tester-one","cascade_position":"1","cascade_next":"oit-s2","cascade_id":"orchestrator-integration"}' \
    --silent >/dev/null
  run_isolated gc --city "$city_dir" bd create "cascade step two" \
    --id oit-s2 \
    --labels cascade-chain \
    --metadata '{"cascade_target_agent":"cascade-tester-two","cascade_position":"2","cascade_next":"oit-s3","cascade_id":"orchestrator-integration"}' \
    --silent >/dev/null
  run_isolated gc --city "$city_dir" bd create "cascade step three" \
    --id oit-s3 \
    --labels cascade-chain \
    --metadata '{"cascade_target_agent":"cascade-tester-three","cascade_position":"3","cascade_final":"true","cascade_id":"orchestrator-integration"}' \
    --silent >/dev/null
}

wait_for_agent_run() {
  local target_agent="$1"
  local bead_id="$2"
  local log_path="$city_dir/.gc/cascade-test/agent-runs.tsv"
  local deadline=$((SECONDS + 60))
  until [ -f "$log_path" ] && grep -Fq "$target_agent	$bead_id" "$log_path"; do
    if [ "$SECONDS" -ge "$deadline" ]; then
      printf 'expected %s to run %s\n' "$target_agent" "$bead_id" >&2
      printf 'test root: %s\n' "$root" >&2
      [ -f "$log_path" ] && cat "$log_path" >&2
      run_isolated gc --city "$city_dir" bd show "$bead_id" --json >&2 || true
      run_isolated gc --city "$city_dir" bd ready \
        --metadata-field "gc.routed_to=$target_agent" \
        --unassigned \
        --json \
        --limit 5 >&2 || true
      run_isolated gc --city "$city_dir" session list --state all --json >&2 || true
      find "$city_dir/.gc" -maxdepth 5 -type f | sort >&2 || true
      cat "$root/orchestrator.log" >&2 || true
      cat "$root/supervisor.log" >&2 || true
      exit 1
    fi
    sleep 0.2
  done
}

agent_run_count() {
  local target_agent="$1"
  local bead_id="$2"
  local log_path="$city_dir/.gc/cascade-test/agent-runs.tsv"
  if [ ! -f "$log_path" ]; then
    printf '0\n'
    return
  fi
  grep -F "$target_agent	$bead_id" "$log_path" | wc -l | tr -d ' '
}

assert_agent_run_count() {
  local target_agent="$1"
  local bead_id="$2"
  local expected="$3"
  local actual
  actual="$(agent_run_count "$target_agent" "$bead_id")"
  if [ "$actual" != "$expected" ]; then
    printf 'expected %s run(s) for %s %s, got %s\n' \
      "$expected" "$target_agent" "$bead_id" "$actual" >&2
    [ -f "$city_dir/.gc/cascade-test/agent-runs.tsv" ] \
      && cat "$city_dir/.gc/cascade-test/agent-runs.tsv" >&2
    exit 1
  fi
}

open_gate() {
  local bead_id="$1"
  touch "$city_dir/.gc/cascade-test/gates/$bead_id"
}

wait_for_bead_closed() {
  local bead_id="$1"
  local deadline=$((SECONDS + 60))
  until run_isolated gc --city "$city_dir" bd show "$bead_id" --json \
    | jq -e '.[0].status == "closed"' >/dev/null; do
    if [ "$SECONDS" -ge "$deadline" ]; then
      run_isolated gc --city "$city_dir" bd show "$bead_id" --json >&2 || true
      exit 1
    fi
    sleep 0.2
  done
}

wait_for_mayor_mail() {
  local deadline=$((SECONDS + 60))
  until run_isolated gc --city "$city_dir" mail inbox mayor \
    | grep -Fq "cascade complete: orchestrator-integration"; do
    if [ "$SECONDS" -ge "$deadline" ]; then
      run_isolated gc --city "$city_dir" mail inbox mayor >&2 || true
      exit 1
    fi
    sleep 0.2
  done
}

seed_supervisor_config
seed_dolt_identity
install_host_command_shims
if [ "$codex_provider_mode" = "shim" ]; then
  install_codex_shim
fi
start_isolated_supervisor

run_isolated gc init \
  --skip-provider-readiness \
  --file "$ORCHESTRATOR_TEST_CITY_TOML" \
  "$city_dir" >/dev/null
run_isolated gc --city "$city_dir" bd config set types.custom session,message,event,convoy >/dev/null
install_test_agents

run_orchestrator_once

start_orchestrator
first_orchestrator_pid="$orchestrator_pid"
create_cascade_beads
wait_for_agent_run cascade-tester-one oit-s1
stop_orchestrator "$first_orchestrator_pid"

start_orchestrator
second_orchestrator_pid="$orchestrator_pid"
sleep 2
assert_agent_run_count cascade-tester-one oit-s1 1

open_gate oit-s1
wait_for_bead_closed oit-s1
wait_for_agent_run cascade-tester-two oit-s2
open_gate oit-s2
wait_for_bead_closed oit-s2
wait_for_agent_run cascade-tester-three oit-s3
open_gate oit-s3
wait_for_bead_closed oit-s3
wait_for_mayor_mail
stop_orchestrator "$second_orchestrator_pid"

run_orchestrator_once
assert_agent_run_count cascade-tester-one oit-s1 1
assert_agent_run_count cascade-tester-two oit-s2 1
assert_agent_run_count cascade-tester-three oit-s3 1

printf 'orchestrator isolated Gas City integration passed\n'
