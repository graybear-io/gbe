#!/bin/bash
# Simple end-to-end test for GBE
#
# Tests with "seq 1 100" for easy verification

set -e

echo "=== GBE Simple Test ==="
echo ""

# Cleanup function
cleanup() {
    echo ""
    echo "Cleaning up..."
    kill $ROUTER_PID 2>/dev/null || true
    kill $ADAPTER_PID 2>/dev/null || true
    rm -f /tmp/gbe-*.sock
    echo "✓ Cleanup complete"
    exit 0
}

trap cleanup EXIT INT TERM

# Clean up old sockets
rm -f /tmp/gbe-*.sock

# Start router
echo "Starting router..."
cargo run --package gbe-router --quiet &
ROUTER_PID=$!
sleep 1

if [ ! -S /tmp/gbe-router.sock ]; then
    echo "Error: Router socket not created"
    exit 1
fi
echo "✓ Router started (PID: $ROUTER_PID)"

# Start adapter with "seq 1 100"
echo "Starting adapter (seq 1 100)..."
cargo run --package gbe-adapter --quiet -- seq 1 100 &
ADAPTER_PID=$!
sleep 1
echo "✓ Adapter started (PID: $ADAPTER_PID)"

# Discover adapter ToolId
ROUTER_ID=$(pgrep -f gbe-router | head -1)
ADAPTER_ID="${ROUTER_ID}-001"
echo "✓ Adapter ToolId: $ADAPTER_ID"

echo ""
echo "=== Starting Client ==="
echo "You should see numbers 1-100 streaming in."
echo "Try the keyboard shortcuts:"
echo "  f     - Toggle follow mode"
echo "  ↑/↓   - Scroll through history"
echo "  End   - Jump to bottom"
echo "  q     - Quit when done"
echo ""
sleep 2

# Start client
cargo run --package gbe-client -- --target "$ADAPTER_ID"

echo ""
echo "✓ Test complete"
