# GBE End-to-End Testing

Complete guide for testing the full GBE stack: router + adapter + client.

## Quick Start

### Automated Test (Recommended)

The automated test verifies the complete data flow through all components.

```bash
# Run full stack integration test
cargo test --package gbe-router --test e2e_full_stack -- --ignored --nocapture
```

**What it tests:**
1. Router startup and socket creation
2. Adapter connection and registration
3. Client subscription to adapter
4. Data streaming (10 lines via `seq 1 10`)
5. Message ordering and correctness
6. Clean shutdown

**Expected output:**
```
=== GBE Full Stack E2E Test ===

✓ Router ready
✓ Adapter started
✓ Adapter ToolId: xxxxx-001
✓ Client ToolId: xxxxx-002
✓ Data stream connected
  [0] 1
  [1] 2
  ...
  [9] 10
✓ All lines correct

=== ✓ Full Stack Test Passed ===
```

### Interactive Tests

#### Simple Test (Best for Verification)

Tests with `seq 1 100` - numbers are easy to verify.

```bash
./test_simple.sh
```

**What you'll see:**
- Terminal UI displays numbers 1-100
- Smooth scrolling with no flicker
- Status bar shows mode and keyboard shortcuts

**Try these features:**
- **Follow mode**: Lines auto-scroll as they arrive (default on)
- **Toggle follow**: Press `f` to stop auto-scrolling
- **Scroll**: Use `↑`/`↓` to navigate history
- **Jump to bottom**: Press `End`
- **Quit**: Press `q`

#### Streaming Test

Tests with a live log file for continuous streaming.

```bash
./test_interactive.sh
```

## Manual Component Testing

### 1. Router Only

```bash
# Terminal 1: Start router
cargo run --package gbe-router

# Verify socket created
ls -l /tmp/gbe-router.sock
```

Expected output:
```
Starting gbe-router v0.1.0
Listening on /tmp/gbe-router.sock
```

### 2. Router + Adapter

```bash
# Terminal 1: Start router
cargo run --package gbe-router

# Terminal 2: Start adapter
cargo run --package gbe-adapter -- seq 1 5
```

Adapter output:
```
gbe-adapter v0.1.0
Command: ["seq", "1", "5"]
Connecting to router...
Assigned ToolId: 12345-001    <-- Note this ToolId
Data address: unix:///tmp/gbe-12345-001.sock
Data listener bound at /tmp/gbe-12345-001.sock
Spawning command: seq ["1", "5"]
```

### 3. Full Stack (Manual - 3 Terminals)

```bash
# Terminal 1: Router
cargo run --package gbe-router

# Terminal 2: Adapter with streaming command
cargo run --package gbe-adapter -- sh -c 'for i in {1..20}; do echo "Line $i"; sleep 0.5; done'

# Terminal 3: Get the adapter ToolId from Terminal 2 output
# If router PID is 12345, adapter ToolId is 12345-001

# Terminal 3: Start client
cargo run --package gbe-client -- --target 12345-001
```

Client keyboard shortcuts:
- `q` - Quit
- `f` - Toggle follow mode
- `↑`/`↓` - Scroll up/down
- `End` - Jump to bottom

## Architecture Verification

The tests verify these architectural properties:

### Message Flow
```
┌─────────┐         ┌─────────┐         ┌─────────┐
│ Adapter │────────▶│ Router  │◀────────│ Client  │
└─────────┘         └─────────┘         └─────────┘
     │                                        │
     │                                        │
     └────────────Data Channel────────────────┘
                  (after Subscribe)
```

### Protocol Sequence
1. **Connect**: Adapter → Router (control channel)
2. **ConnectAck**: Router → Adapter (assigns ToolId)
3. **Data Listen**: Adapter creates data socket
4. **Connect**: Client → Router (control channel)
5. **ConnectAck**: Router → Client (assigns ToolId)
6. **Subscribe**: Client → Router (target: adapter ToolId)
7. **SubscribeAck**: Router → Client (data socket address)
8. **Data Connect**: Client → Adapter (data channel)
9. **Data Frames**: Adapter → Client (streamed output)
10. **Disconnect**: Client/Adapter → Router

### Performance Metrics

Expected performance (from acceptance criteria):

| Metric | Target | Observed |
|--------|--------|----------|
| Message passing latency | <10ms | ✓ Pass |
| Smooth rendering | No flicker | ✓ Pass |
| Input latency | <10ms | ✓ Pass |
| Buffer view queries | <10ms | ✓ Pass (0.2ms) |

## Command Examples

Try different commands with the adapter:

```bash
# List files
cargo run --package gbe-adapter -- ls -la

# Show date every second
cargo run --package gbe-adapter -- sh -c 'while true; do date; sleep 1; done'

# Echo message
cargo run --package gbe-adapter -- echo "Hello from GBE!"

# Generate numbers
cargo run --package gbe-adapter -- seq 1 100

# Monitor log file
cargo run --package gbe-adapter -- tail -f /var/log/system.log

# Run a script
cargo run --package gbe-adapter -- bash myscript.sh
```

## Troubleshooting

### Router won't start
```bash
# Check if old socket exists
rm -f /tmp/gbe-*.sock

# Check for process conflicts
lsof | grep gbe-router
```

### Client can't subscribe
```bash
# Verify adapter ToolId
pgrep -f gbe-router
# Use ROUTER_PID-001 as adapter ToolId

# Check router logs for errors
# Router shows all connections and subscriptions
```

### No data flowing
```bash
# Check data socket exists
ls -l /tmp/gbe-*.sock

# Verify adapter is still running
ps aux | grep gbe-adapter

# Check if command produced output
cargo run --package gbe-adapter -- echo "test"
```

### Terminal display issues
```bash
# Reset terminal if garbled
reset

# Check terminal size
echo $COLUMNS $LINES

# Ensure terminal supports color
echo $TERM
```

### Common Issues

**"Failed to connect to router"**
- Make sure router is running first
- Check socket exists: `ls /tmp/gbe-router.sock`

**"Tool not found"**
- Verify ToolId format: `PID-SEQ` (e.g., `12345-001`)
- Check adapter is still running

**"Failed to connect to data channel"**
- Check data socket path in adapter logs
- Verify permissions on socket file

## Phase 1 Success Criteria

- [x] Can wrap any Unix command
- [x] Message passing latency <10ms
- [x] Smooth terminal rendering (no flicker)
- [x] Real-time data streaming works
- [x] Keyboard input handled correctly
- [x] Follow mode functional
- [x] Clean shutdown and cleanup
- [ ] Sessions survive disconnects (requires buffer integration)

## Next Steps

After verifying the full stack works:

1. **Phase 1 Completion**:
   - Integration test suite ✓
   - Performance benchmarks ✓
   - Code review
   - User acceptance testing

2. **Future Enhancements** (Phase 2+):
   - Multiple data subscribers (proxy/tee)
   - Buffer integration for persistence
   - Tool chains and composition
   - Session persistence
   - Split panes and multiple views
