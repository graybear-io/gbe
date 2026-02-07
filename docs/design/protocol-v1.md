# GBE Protocol v1 Design

**Status:** Planning
**Date:** 2026-02-07
**Authors:** bear, claude

## Overview

GBE uses a dual-channel architecture: control channel for coordination, data channel for streaming.

**Key principle:** Separate control topology (star via router) from data topology (pipeline with optional proxy).

## Architecture

```
Control Plane (star topology):
        [router]
       /   |    \
   toolA toolB toolC

Data Plane (pipeline topology):
toolA ──data──> toolB ──data──> toolC
  │
  └──tee──> toolD (via proxy)
```

## Design Decisions

### 1. Dual Channel Model

**Control Channel:**
- Bidirectional
- All messages flow through router
- JSON initially, binary later
- Low frequency, high complexity

**Data Channel:**
- Unidirectional point-to-point
- Binary, optimized for streaming
- High frequency, low complexity
- 99% of traffic

**Rationale:** Separates hot path (data) from coordination (control). Allows independent optimization.

### 2. Router-Assigned Addresses

**Decision:** Router assigns data listen addresses to tools.

**Flow:**
1. Tool connects to router (control channel)
2. Router assigns ToolId and data address
3. Tool binds listener at assigned address
4. Subscribers connect via router coordination

**Rationale:**
- Router already coordinating - natural fit
- Simpler for tools - no address allocation logic
- Router can manage namespace (avoid conflicts)

**Alternative considered:** Tools self-assign addresses
- Rejected: requires conflict resolution, more complexity

### 3. ToolId Format: PID+Sequence

**Format:** `{pid}-{seq}`
- Example: `"12345-001"`, `"12345-002"`

**Properties:**
- Compact (vs UUID bloat)
- Debuggable (see actual PID)
- Unique within router lifetime
- No global coordination needed

**Rationale:** Simple, practical, sufficient for router lifetime scope.

### 4. Proxy Subprocess (Phase 1)

**Decision:** Implement proxy as subprocess initially.

**Phase 1 Implementation:**
```bash
# Router spawns when tee needed
gbe-proxy --upstream toolA --listen /tmp/proxy-001.sock
```

**Proxy responsibilities:**
- Connect to upstream data socket
- Accept downstream connections
- Duplicate messages (tee functionality)
- Monitor TCP backpressure
- Send flow control messages to router

**Future considerations (document for later revisit):**
- Thread-per-proxy (lighter weight)
- Async task in router process (single-threaded)
- Kernel-level tee (splice/tee syscalls)
- Performance benchmarking to guide decision

**Rationale:**
- Start simple - subprocess is clear separation
- Proxy can monitor TCP flow control naturally
- Can send control messages independently
- Optimize later based on real-world usage

**Document:** This decision should be revisited after Phase 1 performance testing.

### 5. Data Framing: Capability-Driven

**Rule:**
```
Capability "raw" present → Raw byte stream (Unix pipe semantics)
Capability "raw" absent  → Length-prefixed framing (default)
```

**Raw mode:**
- Direct passthrough, no framing
- Traditional Unix pipe behavior
- For legacy tools or simple streams

**Framed mode (default):**
```
Wire format:
[u32: length][u64: seq][bytes: payload]
```

**Router/proxy behavior:**
- Reads capabilities from both ends
- Routes raw to raw (passthrough)
- Can transform if needed (strip/add framing)

**Rationale:**
- Default framing enables structured communication
- Raw mode preserves Unix pipe semantics
- Capability negotiation keeps it simple
- Router can bridge modes if needed

### 6. Capabilities: Freeform Strings

**Decision:** Capabilities are freeform strings, no schema initially.

**Flow:**
```
toolA → router: capabilities: ["pty", "color", "raw"]
router → toolB: upstream has capabilities: ["pty", "color", "raw"]
toolB adapts behavior accordingly
```

**Rationale:**
- Emergent structure - let patterns develop
- No premature standardization
- Easy to extend
- Schema can be added later if needed

### 7. Direct vs Proxy Routing

**Router decides per-subscription:**

```rust
if subscribers.len() == 1 {
    // Direct: return upstream's actual address
    return upstream.data_address;
} else {
    // Tee needed: spawn proxy, return proxy address
    let proxy = spawn_proxy(upstream);
    return proxy.data_address;
}
```

**Tools don't know the difference** - protocol identical either way.

**Rationale:**
- Transparent optimization
- No tool refactoring later
- Router can inject proxies for monitoring/debugging
- Future: router could use heuristics for direct vs proxy

### 8. Backpressure Handling

**Mechanism:** TCP flow control + optional control messages.

**Proxy behavior:**
```rust
// Proxy detects blocked write (slow consumer)
if write_blocked {
    router.send(FlowControl {
        source: tool_id,
        status: "backpressure"
    });
}
```

**Router options:**
- Log/alert (observability)
- Throttle source (future)
- Drop messages (future, with policy)

**Phase 1:** Let TCP handle it naturally. Log via control messages.

**Future:** Active backpressure propagation if needed.

**Rationale:** Start simple, TCP does most of the work. Add complexity only if needed.

## Protocol Specification

### Control Channel Messages

**Format:** JSON (Phase 1), binary later (bincode/postcard)

```rust
enum ControlMessage {
    // Connection lifecycle
    Connect {
        capabilities: Vec<String>,
    },
    ConnectAck {
        tool_id: ToolId,              // "12345-001"
        data_listen_address: String,   // "unix:///tmp/gbe-12345-001.sock"
    },
    Disconnect,

    // Subscription management
    Subscribe {
        target: ToolId,
    },
    SubscribeAck {
        data_connect_address: String,  // Where to send data
        capabilities: Vec<String>,      // Upstream capabilities
    },
    Unsubscribe {
        target: ToolId,
    },

    // Flow control (from proxy)
    FlowControl {
        source: ToolId,
        status: String,  // "backpressure" | "flowing"
    },

    // Metadata queries
    QueryCapabilities {
        target: ToolId,
    },
    CapabilitiesResponse {
        capabilities: Vec<String>,
    },

    // Error handling
    Error {
        code: String,
        message: String,
    },
}
```

### Data Channel Format

**Framed mode (default):**

```
Wire format:
┌──────────┬──────────┬─────────────┐
│ length   │ seq      │ payload     │
│ (u32)    │ (u64)    │ (length)    │
└──────────┴──────────┴─────────────┘
 4 bytes    8 bytes    variable
```

```rust
struct DataFrame {
    len: u32,       // Length of payload (not including header)
    seq: u64,       // Sequence number
    payload: [u8],  // Raw bytes (len bytes)
}
```

**Raw mode (capability "raw" present):**

```
Direct byte stream - no framing, no headers
Traditional Unix pipe semantics
```

### Connection Flow Example

**Simple chain (direct):**

```
1. toolA connects to router (control)
   → Connect { capabilities: [] }

2. Router assigns address
   ← ConnectAck {
       tool_id: "12345-001",
       data_listen_address: "unix:///tmp/gbe-12345-001.sock"
     }

3. toolA binds listener at assigned address

4. toolB connects to router (control)
   → Connect { capabilities: [] }

5. Router assigns address
   ← ConnectAck {
       tool_id: "12345-002",
       data_listen_address: "unix:///tmp/gbe-12345-002.sock"
     }

6. toolB subscribes to toolA
   → Subscribe { target: "12345-001" }

7. Router returns toolA's actual address (direct route)
   ← SubscribeAck {
       data_connect_address: "unix:///tmp/gbe-12345-001.sock",
       capabilities: []
     }

8. toolB connects to toolA's data address
   Data flows: toolA → toolB (direct, no router)
```

**Tee scenario (proxied):**

```
1-5. Same as above (toolA and toolB connect)

6. toolB subscribes to toolA
   → Subscribe { target: "12345-001" }

7. toolC also subscribes to toolA
   → Subscribe { target: "12345-001" }

8. Router detects multiple subscribers, spawns proxy
   proxy_pid = spawn("gbe-proxy", "--upstream", "unix:///tmp/gbe-12345-001.sock")
   proxy binds: "unix:///tmp/gbe-proxy-56789.sock"

9. Router returns proxy address to both subscribers
   → toolB: SubscribeAck { data_connect_address: "unix:///tmp/gbe-proxy-56789.sock" }
   → toolC: SubscribeAck { data_connect_address: "unix:///tmp/gbe-proxy-56789.sock" }

10. toolB and toolC connect to proxy
    Data flows: toolA → proxy → {toolB, toolC}
```

## Address Format

**Unix sockets (Phase 1):**
```
unix:///tmp/gbe-{pid}-{seq}.sock
```

**Future transport options:**
```
tcp://localhost:9000
vsock://3:9000
shm:///gbe-shared-mem-001
```

## Implementation Notes

### Router Responsibilities

```rust
struct Router {
    // Connection tracking
    connections: HashMap<ToolId, Connection>,
    capabilities: HashMap<ToolId, Vec<String>>,

    // Routing tables
    subscriptions: HashMap<ToolId, Vec<ToolId>>,  // source → subscribers
    proxies: HashMap<ToolId, ProxyProcess>,       // source → proxy (if needed)

    // Address management
    next_seq: AtomicU64,
}
```

**Router logic:**
- Assign ToolIds and addresses
- Track connections and capabilities
- Route control messages
- Spawn proxies when needed (multiple subscribers)
- Clean up on disconnect

### Tool Implementation Pattern

```rust
// 1. Connect to router (control channel)
let control = UnixStream::connect("/tmp/gbe-router.sock")?;
control.send(Connect { capabilities: vec!["pty"] })?;

// 2. Receive assigned address
let ack: ConnectAck = control.recv()?;
let tool_id = ack.tool_id;
let data_addr = ack.data_listen_address;

// 3. Bind data listener
let data_listener = UnixListener::bind(data_addr)?;

// 4. Accept data connections & stream output
spawn(|| {
    for stream in data_listener.incoming() {
        let mut conn = stream?;
        for line in output_lines {
            conn.write_frame(seq, &line)?;  // Or raw if "raw" capability
        }
    }
});

// 5. Handle control messages
loop {
    let msg: ControlMessage = control.recv()?;
    match msg {
        // Handle subscriptions, queries, etc.
    }
}
```

## Open Questions

1. **Proxy lifecycle:** How long does proxy live after last subscriber disconnects?
   - Immediate cleanup?
   - Timeout (allow reconnect)?
   - Explicit disconnect message?

2. **Sequence numbers:** Who assigns? Per-tool or global?
   - Tool assigns (simple, no coordination)
   - Router assigns (global ordering)
   - Decision: Tool assigns (simpler)

3. **Error propagation:** How do downstream errors reach upstream?
   - Control message from router?
   - Broken pipe (SIGPIPE)?
   - Decision: Broken pipe for data, control for coordination

4. **Multi-router:** Can routers federate for distributed systems?
   - Phase 1: Single router
   - Future: Router-to-router protocol

5. **Session persistence:** How do sessions survive router restart?
   - Phase 1: No persistence (restart = new session)
   - Future: Checkpoint/restore via control messages

## Future Enhancements

### Phase 2+
- Binary control messages (speed)
- Direct data connections (bypass router entirely)
- Shared memory transport (zero-copy)
- Backpressure propagation (active throttling)
- Router federation (multi-host)
- Session persistence

### Performance Targets

**Phase 1 goals:**
- Control message latency: <10ms
- Data throughput: >100MB/s per connection
- Proxy overhead: <5% CPU
- Memory: <10MB per tool

**To be validated with benchmarks.**

## References

- Architecture: [notes/ARCHITECTURE.md](../../notes/ARCHITECTURE.md)
- Implementation: [Phase 1 Epic](../../.beads/issues.jsonl#gbe-1dv)
- Related: Buffer design, Client UI, Adapter wrapper

## Changelog

- 2026-02-07: Initial design (planning phase)
