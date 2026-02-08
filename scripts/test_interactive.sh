#!/bin/bash
# Interactive end-to-end test for GBE
#
# This script demonstrates the full GBE stack:
# - Router (message broker)
# - Adapter (wraps commands)
# - Client (terminal UI)

set -e

echo "=== GBE Interactive Test ==="
echo ""
echo "This will start:"
echo "  1. Router (background)"
echo "  2. Adapter wrapping 'tail -f /var/log/system.log' (background)"
echo "  3. Client (interactive TUI)"
echo ""
echo "Press Ctrl+C to stop all processes"
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

# Check if router started
if [ ! -S /tmp/gbe-router.sock ]; then
    echo "Error: Router socket not created"
    exit 1
fi
echo "✓ Router started (PID: $ROUTER_PID)"

# Start adapter with a streaming command
echo "Starting adapter (tail -f)..."

# Choose a log file that exists
if [ -f /var/log/system.log ]; then
    LOG_FILE="/var/log/system.log"
elif [ -f /var/log/syslog ]; then
    LOG_FILE="/var/log/syslog"
else
    # Fallback: create a test stream
    LOG_FILE="/tmp/gbe-test.log"
    (while true; do echo "$(date): Test log entry"; sleep 1; done > "$LOG_FILE") &
    LOGGER_PID=$!
fi

cargo run --package gbe-adapter --quiet -- tail -f "$LOG_FILE" &
ADAPTER_PID=$!
sleep 1
echo "✓ Adapter started (PID: $ADAPTER_PID)"

# Discover adapter ToolId
ROUTER_ID=$(pgrep -f gbe-router | head -1)
ADAPTER_ID="${ROUTER_ID}-001"
echo "✓ Adapter ToolId: $ADAPTER_ID"

# Instructions
echo ""
echo "=== Starting Client ==="
echo ""
echo "Keyboard shortcuts:"
echo "  q     - Quit"
echo "  f     - Toggle follow mode"
echo "  ↑/↓   - Scroll"
echo "  End   - Jump to bottom"
echo ""
sleep 2

# Start client (foreground, interactive)
cargo run --package gbe-client -- --target "$ADAPTER_ID"

# Client exited, cleanup
echo ""
echo "Client exited"
