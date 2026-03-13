#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 4 ]]; then
  echo "usage: $0 <log-file> <diagnostics-dir> <label> <command> [args...]" >&2
  exit 64
fi

log_file=$1
diagnostics_dir=$2
label=$3
shift 3

mkdir -p "$(dirname "$log_file")" "$diagnostics_dir"
: >"$log_file"

"$@" >"$log_file" 2>&1 &
root_pid=$!

tail -n +1 -F "$log_file" &
tail_pid=$!

collect_diagnostics() {
  local stamp=$1
  local ps_file="$diagnostics_dir/${label}-${stamp}-ps.txt"
  local top_file="$diagnostics_dir/${label}-${stamp}-top.txt"

  {
    echo "timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "root_pid=$root_pid"
    echo "label=$label"
    ps -axww -o pid=,ppid=,pgid=,stat=,time=,%cpu=,%mem=,command=
  } >"$ps_file" || true

  if command -v top >/dev/null 2>&1; then
    top -l 1 -stats pid,command,state,cpu,mem,time -o cpu >"$top_file" 2>&1 || true
  fi

  local pids=()
  while IFS= read -r pid; do
    [[ -n "$pid" ]] && pids+=("$pid")
  done < <(
    {
      echo "$root_pid"
      pgrep -P "$root_pid" || true
    } | awk 'NF { print $1 }' | sort -u
  )

  printf 'fixture watchdog: label=%s stamp=%s root_pid=%s pids=%s\n' \
    "$label" "$stamp" "$root_pid" "${pids[*]:-none}" >>"$log_file"

  if ! command -v sample >/dev/null 2>&1; then
    return
  fi

  for pid in "${pids[@]}"; do
    if kill -0 "$pid" 2>/dev/null; then
      sample "$pid" 5 -file "$diagnostics_dir/${label}-${stamp}-pid${pid}.sample.txt" >/dev/null 2>&1 || true
    fi
  done
}

watchdog() {
  local elapsed_secs=0
  while kill -0 "$root_pid" 2>/dev/null; do
    sleep 60
    kill -0 "$root_pid" 2>/dev/null || break
    elapsed_secs=$((elapsed_secs + 60))
    collect_diagnostics "t${elapsed_secs}s"
  done
}

watchdog &
watchdog_pid=$!

set +e
wait "$root_pid"
status=$?
set -e

kill "$watchdog_pid" 2>/dev/null || true
wait "$watchdog_pid" 2>/dev/null || true
kill "$tail_pid" 2>/dev/null || true
wait "$tail_pid" 2>/dev/null || true

exit "$status"
