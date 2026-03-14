#!/bin/sh
# ROLE: Oracle + Overseer
#
# In the real system:
#   - Oracle walks the DAG, publishes ready tasks to gbe.tasks.shell.queue
#   - Sentinel claims from queue, boots VM, starts operative
#   - Overseer discovers sources via envoy QueryTools
#   - Overseer tells cryptum "allocate surface for source X"
#
# For this POC: builds binaries, starts envoy router, then sentinel.
# Sentinel registers its own envoy streams (lifecycle + progress) and
# starts an adapter for the operative output stream.
#
# Three envoy streams:
#   1. sentinel-lifecycle  — sentinel host events
#   2. task-progress       — per-task heartbeats and state changes
#   3. task-output         — operative stdout (via adapter)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GBE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ROUTER_SOCK="/tmp/gbe-router.sock"
NEXUS_DIR="/tmp/nexus"
PIDS=""

cleanup() {
  echo ""
  echo "[poc] cleaning up..."
  for pid in $PIDS; do
    kill "$pid" 2>/dev/null || true
  done
  wait 2>/dev/null
  rm -f "$ROUTER_SOCK"
  echo "[poc] done"
}
trap cleanup EXIT INT TERM

echo "=== GBE POC: Distributed Task Lifecycle ==="
echo ""

# Prepare nexus dir
mkdir -p "$NEXUS_DIR"

# Envoy tracing respects RUST_LOG
export RUST_LOG="${RUST_LOG:-off}"

# Build envoy + sentinel on macOS
echo "[poc] building envoy + sentinel (macOS)..."
cargo build --manifest-path "$GBE_ROOT/Cargo.toml" -p gbe-router -p gbe-adapter -p gbe-client -q
cargo build --manifest-path "$SCRIPT_DIR/Cargo.toml" -p poc-sentinel -q

# Build operative on ark
echo "[poc] building operative (ark)..."
ssh ark '. $HOME/.cargo/env && cargo build --manifest-path /mnt/projects/gbe/poc/Cargo.toml -p poc-operative -q'

ROUTER="$GBE_ROOT/target/debug/gbe-router"
CLIENT="$GBE_ROOT/target/debug/gbe-client"
SENTINEL="$SCRIPT_DIR/target/debug/poc-sentinel"

# Start router
echo "[poc] starting router..."
"$ROUTER" --socket "$ROUTER_SOCK" &
PIDS="$! $PIDS"
sleep 1

# Start sentinel — it registers its own streams and starts an adapter
echo "[poc] starting sentinel..."
export GBE_ROUTER="$ROUTER_SOCK"
export ENVOY_DIR="$GBE_ROOT"
"$SENTINEL" &
SENTINEL_PID=$!
PIDS="$SENTINEL_PID $PIDS"
sleep 3

# Discover streams
echo ""
echo "=== POC is running ==="
echo ""
echo "Streams:"
"$CLIENT" --router "$ROUTER_SOCK" --list | while read -r line; do
  echo "  $line"
done
echo ""
echo "Connect to a stream:"
echo "  $CLIENT --router $ROUTER_SOCK --target <TOOL_ID>"
echo ""
echo "Stop:"
echo "  ./poc/stop-poc.sh"
echo ""
echo "Traces:"
echo "  tail -f $NEXUS_DIR/gbe.trace.sentinel"
echo "  tail -f $NEXUS_DIR/gbe.sentinel.ark.lifecycle"
echo "  tail -f $NEXUS_DIR/gbe.tasks.shell.progress.system-monitor"
echo ""

# Wait for sentinel to exit
wait "$SENTINEL_PID" 2>/dev/null
