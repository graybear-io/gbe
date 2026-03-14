#!/bin/sh
# ROLE: Sentinel — manages task lifecycle, reports status
#
# In the real system:
#   - Sentinel claims task from gbe.tasks.shell.queue (CAS on state store)
#   - Boots Firecracker VM, injects operative
#   - Monitors health, publishes status to gbe.tasks.shell.progress
#   - On completion/stop: tears down VM, publishes to terminal stream
#
# For this POC: monitors the operative process, writes lifecycle events
# to a log file (what would be nexus bus events in the real system).
# Operative stdout passes through to this script's stdout (captured
# by the envoy adapter as data frames).

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# FUTURE: these events would be published to nexus via
# gbe.tasks.shell.progress and gbe.tasks.shell.terminal
#
# For POC: write to /tmp/nexus/<subject> — named after the nexus
# subject hierarchy so the pattern scales as we add more streams.
TASK_ID="system-monitor"
NEXUS_DIR="/tmp/nexus"
PROGRESS_LOG="$NEXUS_DIR/gbe.tasks.shell.progress.$TASK_ID"
TERMINAL_LOG="$NEXUS_DIR/gbe.tasks.shell.terminal.$TASK_ID"

rm -f /tmp/ark-flags/poc-stop
mkdir -p /tmp/ark-flags "$NEXUS_DIR"
: > "$PROGRESS_LOG"
: > "$TERMINAL_LOG"

progress() {
  echo "$1" >> "$PROGRESS_LOG"
}

terminal() {
  echo "$1" >> "$TERMINAL_LOG"
}

progress '{"event":"sentinel_claimed","task":"'$TASK_ID'"}'

# FUTURE: sentinel would boot a VM here and inject the operative
# For POC: start the operative. Its stdout flows through to our stdout
# (which the adapter captures as envoy data frames).
"$SCRIPT_DIR/poc-operative.sh" &
OP_PID=$!
progress '{"event":"operative_started","pid":'$OP_PID'}'

TICK=0
while kill -0 "$OP_PID" 2>/dev/null; do
  if [ -f /tmp/ark-flags/poc-stop ]; then
    progress '{"event":"stop_received","tick":'$TICK'}'
    # FUTURE: sentinel sends SIGTERM to VM, waits for graceful shutdown
    wait "$OP_PID" 2>/dev/null
    progress '{"event":"operative_exited","tick":'$TICK'}'
    terminal '{"event":"task_terminal","outcome":"stopped","task":"'$TASK_ID'"}'
    # FUTURE: publishes to gbe.tasks.shell.terminal
    exit 0
  fi

  progress '{"event":"heartbeat","state":"running","pid":'$OP_PID',"tick":'$TICK'}'
  # FUTURE: this heartbeat updates state store (updated_at field)
  # watcher scans for stale updated_at to detect stuck tasks
  TICK=$((TICK + 1))
  sleep 5
done

progress '{"event":"operative_exited","tick":'$TICK'}'
terminal '{"event":"task_terminal","outcome":"completed","task":"'$TASK_ID'"}'
