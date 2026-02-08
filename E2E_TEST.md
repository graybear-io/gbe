# End-to-End Test Guide

Test the full pipeline: Router → Adapter → Subscriber

## Quick Test (3 terminals)

### Terminal 1: Start Router
```bash
cargo run --package gbe-router
```

Expected output:
```
Starting gbe-router v0.1.0
Listening on /tmp/gbe-router.sock
```

### Terminal 2: Start Adapter
```bash
cargo run --package gbe-adapter -- seq 1 5
```

Expected output:
```
gbe-adapter v0.1.0
Command: ["seq", "1", "5"]
Connecting to router...
Assigned ToolId: 12345-001    <-- Note this ToolId
Data address: unix:///tmp/gbe-12345-001.sock
Data listener bound at /tmp/gbe-12345-001.sock
Spawning command: seq ["1", "5"]
Data subscriber connected
...
```

### Terminal 3: Run Subscriber
```bash
cargo run --package gbe-adapter --example e2e_test
```

When prompted, enter the ToolId from Terminal 2 (e.g., `12345-001`)

Expected output:
```
=== End-to-End Test: Adapter Subscriber ===

1. Connecting to router...
   ✓ My ToolId: 12345-002

2. Enter the ToolId of the adapter to subscribe to:
   ToolId: 12345-001

3. Subscribing to 12345-001...
   ✓ Data address: unix:///tmp/gbe-12345-001.sock

4. Connecting to data channel...
   ✓ Connected!

5. Receiving data frames:

---
1
2
3
4
5
---

✓ Received 5 frames
✓ End-to-end test complete!
```

## What's Happening

1. **Router** accepts connections, assigns ToolIds, coordinates subscriptions
2. **Adapter** wraps `seq 1 5`, spawns it, streams output as DataFrame messages
3. **Subscriber** connects to router, subscribes to adapter, receives data frames

## Protocol Flow

```
Subscriber                Router                  Adapter
    |                       |                        |
    |-- Connect ----------->|                        |
    |<- ConnectAck (ID2) ---|                        |
    |                       |<- Connect -------------|
    |                       |-- ConnectAck (ID1) --->|
    |                       |                        |
    |-- Subscribe(ID1) ---->|                        |
    |<- SubscribeAck -------|                        |
    |                                                 |
    |-- [data channel] --------------------------->  |
    |<- DataFrame(seq=0, "1\n") ---------------------|
    |<- DataFrame(seq=1, "2\n") ---------------------|
    |<- DataFrame(seq=2, "3\n") ---------------------|
    |<- DataFrame(seq=3, "4\n") ---------------------|
    |<- DataFrame(seq=4, "5\n") ---------------------|
    |<- [EOF] ----------------------------------------|
```

## Other Commands to Try

```bash
# List files
cargo run --package gbe-adapter -- ls -la

# Show date every second (Ctrl+C to stop)
cargo run --package gbe-adapter -- sh -c 'while true; do date; sleep 1; done'

# Echo hello
cargo run --package gbe-adapter -- echo "Hello from GBE!"

# Generate numbers
cargo run --package gbe-adapter -- seq 1 100
```

## Troubleshooting

**"Failed to connect to router"**
- Make sure Terminal 1 (router) is running first

**"Tool not found"**
- Make sure you entered the correct ToolId from the adapter output
- ToolId format is `PID-SEQ` (e.g., `12345-001`)

**"Failed to connect to data channel"**
- Router may be routing to wrong address
- Check adapter logs for data socket path
