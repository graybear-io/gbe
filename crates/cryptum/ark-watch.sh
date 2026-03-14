#!/bin/sh
# ark-watch — file-based dev loop for cryptum on the Ark VM
# Watches for control flags in the project directory:
#   .ark-rebuild  → restart the weston display stack, log to .ark-log
#   .ark-stop     → stop the running stack
#
# On start: waits for .ark-rebuild flag to launch.
# All output goes to .ark-log on the shared folder.
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Flags live on a local filesystem to avoid 9p dentry cache issues.
# The host touches these via: ssh ark 'touch /tmp/ark-rebuild'
FLAG_DIR="/tmp/ark-flags"
mkdir -p "$FLAG_DIR"

LOG="$SCRIPT_DIR/.ark-log"
STACK_PID=""

cleanup() {
  echo "[ark-watch] shutting down"
  do_kill
  rm -f "$FLAG_DIR/rebuild" "$FLAG_DIR/stop"
}
trap cleanup EXIT INT TERM

do_run() {
  echo "" > "$LOG"
  echo "[ark-watch] === starting cryptum stack ===" | tee -a "$LOG"
  /bin/sh "$SCRIPT_DIR/start-cryptum.sh" 2>&1 | tee -a "$LOG" &
  STACK_PID=$!
  echo "[ark-watch] cryptum running (pid $STACK_PID)" | tee -a "$LOG"
}

do_kill() {
  echo "[ark-watch] stopping cryptum" | tee -a "$LOG"
  if [ -n "$STACK_PID" ] && kill -0 "$STACK_PID" 2>/dev/null; then
    kill "$STACK_PID" 2>/dev/null
    wait "$STACK_PID" 2>/dev/null || true
  fi
  killall ttyd 2>/dev/null || true
  sleep 1
  STACK_PID=""
}

# clean slate
rm -f "$FLAG_DIR/rebuild" "$FLAG_DIR/stop"
echo "[ark-watch] waiting for $FLAG_DIR/rebuild"

while true; do
  if [ -f "$FLAG_DIR/rebuild" ]; then
    rm -f "$FLAG_DIR/rebuild"
    do_kill
    do_run
  fi

  if [ -f "$FLAG_DIR/stop" ]; then
    rm -f "$FLAG_DIR/stop"
    do_kill
    echo "[ark-watch] stopped, waiting for rebuild flag" | tee -a "$LOG"
  fi

  sleep 1
done
