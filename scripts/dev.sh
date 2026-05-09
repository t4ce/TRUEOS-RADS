#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${RADS_PORT:-7377}"
HOST="${RADS_HOST:-127.0.0.1}"
POLL_SECONDS="${RADS_DEV_POLL_SECONDS:-1}"
KILL_STALE="${RADS_DEV_KILL_STALE:-0}"

child_pid=""

log() {
  printf '[rads-dev] %s\n' "$*"
}

port_pids() {
  ss -ltnp "sport = :${PORT}" 2>/dev/null \
    | sed -n 's/.*pid=\([0-9][0-9]*\).*/\1/p' \
    | sort -u
}

stop_child() {
  if [[ -n "${child_pid}" ]] && kill -0 "${child_pid}" 2>/dev/null; then
    log "stopping server pid ${child_pid}"
    kill "${child_pid}" 2>/dev/null || true
    wait "${child_pid}" 2>/dev/null || true
  fi
  child_pid=""
}

cleanup() {
  stop_child
}

fingerprint() {
  (
    cd "${ROOT}"
    {
      find src static -type f -print
      printf '%s\n' Cargo.toml Cargo.lock .env.local
    } | while IFS= read -r path; do
      [[ -f "${path}" ]] || continue
      stat -c '%n %Y %s' "${path}"
    done | sort
  )
}

build_and_start() {
  stop_child

  mapfile -t existing_pids < <(port_pids)
  if ((${#existing_pids[@]} > 0)); then
    if [[ "${KILL_STALE}" == "1" ]]; then
      log "stopping stale process(es) on ${HOST}:${PORT}: ${existing_pids[*]}"
      kill "${existing_pids[@]}" 2>/dev/null || true
      sleep 0.25
    else
      log "${HOST}:${PORT} is already in use by pid(s): ${existing_pids[*]}"
      log "stop that process first, or run RADS_DEV_KILL_STALE=1 scripts/dev.sh"
      exit 1
    fi
  fi

  log "building"
  cargo build

  log "starting http://${HOST}:${PORT}"
  (
    cd "${ROOT}"
    ./target/debug/trueos-rads
  ) &
  child_pid="$!"
}

trap cleanup EXIT INT TERM

cd "${ROOT}"
build_and_start
last_fingerprint="$(fingerprint)"

while true; do
  sleep "${POLL_SECONDS}"

  if [[ -n "${child_pid}" ]] && ! kill -0 "${child_pid}" 2>/dev/null; then
    wait "${child_pid}" 2>/dev/null || true
    child_pid=""
    log "server exited; waiting for a source change before restart"
  fi

  current_fingerprint="$(fingerprint)"
  if [[ "${current_fingerprint}" != "${last_fingerprint}" ]]; then
    last_fingerprint="${current_fingerprint}"
    log "change detected"
    build_and_start
  fi
done
