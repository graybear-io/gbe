#!/bin/sh
# ROLE: Operative — executes the task, emits output
#
# In the real system:
#   - ShellOperative receives TaskDefinition from sentinel via vsock
#   - Executes params.command, captures stdout
#   - Returns TaskOutcome (success/failure + output) via vsock
#
# For this POC: runs a periodic command, emits to stdout (which the
# adapter captures as envoy data frames)

ITERATION=0
echo '{"event":"task_started","task":"system-monitor","type":"shell"}'

while [ ! -f /tmp/ark-flags/poc-stop ]; do
  ITERATION=$((ITERATION + 1))
  echo "--- iteration $ITERATION at $(date -Iseconds) ---"
  uptime
  df -h / | tail -1
  cat /proc/meminfo | head -3
  # FUTURE: this output would be structured as TaskOutcome.output
  # and published to gbe.tasks.shell.progress via nexus
  echo ""
  sleep 30
done

echo '{"event":"task_completed","task":"system-monitor","iterations":'$ITERATION'}'
# FUTURE: operative sends TaskOutcome::Success to sentinel via vsock
# sentinel publishes to gbe.tasks.shell.terminal
