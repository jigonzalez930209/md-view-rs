# Process lifecycle and memory guard for the E2E run. Sourced by run.sh.
#
# Every process the run starts is registered in a state file. A run that dies without
# cleaning up (SIGKILL, a crashed terminal) leaves that file behind, and the next run
# reaps the processes it lists before starting. The memory guard samples the app's
# resident memory and the machine's available memory, and stops the run before a
# runaway allocation can push the machine into swap.

E2E_STATE="$ROOT/target/e2e-pids"
# Resident memory allowed for the app, in MiB. A normal run peaks near 250 MiB.
MAX_RSS_MB=${MDVIEW_E2E_MAX_RSS_MB:-1024}
# Stop the run when the machine has less than this much memory available, in MiB. The app
# normally peaks near 250 MiB, but keeping a desktop-sized reserve prevents swap thrashing.
MIN_AVAILABLE_MB=${MDVIEW_E2E_MIN_AVAILABLE_MB:-2048}
GUARD_PID=""

# Remember a started process: "<pid> <expected command fragment>".
register_pid() {
  mkdir -p "$(dirname "$E2E_STATE")"
  echo "$1 $2" >>"$E2E_STATE"
}

pid_matches() {
  local pid=$1 fragment=$2
  [[ -r "/proc/$pid/cmdline" ]] && tr '\0' ' ' <"/proc/$pid/cmdline" | grep -qF -- "$fragment"
}

# Stop a process: SIGTERM, up to three seconds to exit, then SIGKILL.
stop_pid() {
  local pid=$1
  [[ -n "$pid" ]] || return 0
  kill -0 "$pid" 2>/dev/null || return 0
  kill "$pid" 2>/dev/null || true
  for _ in $(seq 1 30); do
    kill -0 "$pid" 2>/dev/null || return 0
    sleep 0.1
  done
  echo "E2E: pid $pid ignored SIGTERM; sending SIGKILL" >&2
  kill -9 "$pid" 2>/dev/null || true
}

# Kill processes left behind by an earlier run that could not clean up.
reap_stale() {
  [[ -f "$E2E_STATE" ]] || return 0
  local pid fragment
  while read -r pid fragment; do
    [[ -n "$pid" ]] || continue
    # Only a live process that still runs the registered command: pids are reused.
    if pid_matches "$pid" "$fragment"; then
      echo "E2E: stopping stale pid $pid ($fragment)" >&2
      stop_pid "$pid"
    fi
  # Newest first: stop the app before the display and server it depends on.
  done < <(tac "$E2E_STATE")
  rm -f "$E2E_STATE"
}

meminfo_mb() {
  awk -v key="$1:" '$1 == key {print int($2 / 1024)}' /proc/meminfo
}

rss_mb() {
  awk '$1 == "VmRSS:" {print int($2 / 1024)}' "/proc/$1/status" 2>/dev/null || echo 0
}

# Sample the app every 200 ms. Over a limit, record why, kill the app, and stop the run.
start_ram_guard() {
  local app=$1 main=$$
  echo 0 >"$TMP/rss-peak-mb"
  (
    peak=0
    while kill -0 "$app" 2>/dev/null; do
      rss=$(rss_mb "$app")
      available=$(meminfo_mb MemAvailable)
      if (( rss > peak )); then
        peak=$rss
        echo "$peak" >"$TMP/rss-peak-mb"
      fi
      reason=""
      (( rss > MAX_RSS_MB )) && reason="app resident memory ${rss} MiB exceeds ${MAX_RSS_MB} MiB"
      (( available < MIN_AVAILABLE_MB )) &&
        reason="available memory ${available} MiB is below ${MIN_AVAILABLE_MB} MiB (app ${rss} MiB)"
      if [[ -n "$reason" ]]; then
        echo "$reason" >"$TMP/ram-guard"
        kill -9 "$app" 2>/dev/null || true
        kill -TERM "$main" 2>/dev/null || true
        exit 0
      fi
      sleep 0.2
    done
  ) &
  GUARD_PID=$!
}

cleanup() {
  local status=$?
  [[ -n "$GUARD_PID" ]] && kill "$GUARD_PID" 2>/dev/null || true
  stop_pid "$CLIP_PID"
  stop_pid "$APP_PID"
  stop_pid "$SERVER_PID"
  stop_pid "$XVFB_PID"
  rm -f "$E2E_STATE"
  if [[ -f "$TMP/ram-guard" ]]; then
    echo "E2E FAIL: RAM guard: $(cat "$TMP/ram-guard")" >&2
    echo "Artifacts: $TMP" >&2
    status=1
  fi
  exit "$status"
}

# Bash runs the EXIT trap on a normal exit only; a signal would otherwise kill the
# script and orphan the app, the web server, and Xvfb.
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP
