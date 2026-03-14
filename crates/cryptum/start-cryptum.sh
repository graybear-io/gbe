#!/bin/sh
# start-cryptum — launches ttyd web terminal for remote access
set -e

PORT="${CRYPTUM_PORT:-7681}"

cleanup() {
  echo "[cryptum] shutting down"
  kill "$TTYD_PID" 2>/dev/null || true
  wait 2>/dev/null
}
trap cleanup EXIT INT TERM

echo "[cryptum] starting ttyd on 0.0.0.0:$PORT"
ttyd -p "$PORT" -W /bin/sh &
TTYD_PID=$!

echo "[cryptum] stack ready — http://$(ip addr show eth0 2>/dev/null | grep 'inet ' | awk '{print $2}' | cut -d/ -f1 || echo localhost):$PORT"

wait "$TTYD_PID"
