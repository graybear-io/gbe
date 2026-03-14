#!/bin/sh
# ROLE: Oracle sending task cancellation
#
# In the real system:
#   - Oracle or user publishes CancelTask to nexus
#   - Sentinel receives, initiates graceful shutdown
#
# For this POC: send SIGTERM to the sentinel process

SENTINEL_PID=$(pgrep -x poc-sentinel)
if [ -z "$SENTINEL_PID" ]; then
  echo "[poc] no sentinel process found"
  exit 1
fi

echo "[poc] sending SIGTERM to sentinel (pid $SENTINEL_PID)..."
kill -TERM "$SENTINEL_PID"
echo "[poc] stop signal sent — sentinel will shut down gracefully"
