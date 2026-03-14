# Envoy — Tool Composition Substrate

**Repo**: gbe-envoy (Rust workspace: 5 crates)

Separate from the job execution pipeline. A protocol for wiring Unix tools together through adapters, with a TUI client for interactive use.

---

## Architecture

Dual-channel protocol:
- **Control** — JSON (newline-delimited) via router. Tool registration, subscriptions, capability queries.
- **Data** — Binary frames (u32 length + u64 seq + payload) direct P2P. High-throughput streaming.

```
adapter(cmd) ──registers──► router ◄──subscribes── client
     │                        │                      │
     └── data frames ────────(direct or proxy)──────►┘
```

---

## Components

### Router
Dumb control-plane message broker. Assigns ToolIDs (`"{pid}-{seq:03}"`), manages subscriptions, spawns proxies for fan-out. Listens on `/tmp/gbe-router.sock`.

### Adapter
Wraps any Unix command. Bridges stdout to protocol frames. Registers with router on connect, binds a data socket, waits for subscribers.

### Buffer
Two storage modes:
- **Rope** — seekable, append-only (for replay)
- **Ring** — fixed-capacity circular buffer (for live streams)

### Client
ratatui TUI renderer. Connects to router, subscribes to a target tool, renders data frames with follow mode, scrolling, search.

### Proxy
Data channel tee. Router spawns this when multiple clients subscribe to the same tool. Broadcasts frames from upstream to all downstream connections.

---

## Protocol Flow

```
1. Tool connects to router → Connect { capabilities }
2. Router assigns ToolID   → ConnectAck { tool_id, data_listen_address }
3. Tool binds data socket at assigned address
4. Client connects to router → Connect {}
5. Client subscribes         → Subscribe { target: tool_id }
6. Router returns address    → SubscribeAck { data_connect_address }
   (direct if single subscriber, proxy address if multiple)
7. Client connects to data address, reads DataFrame stream
```

---

## Vision

Multiple interfaces on the same substrate: text shell, AI agent, GUI, visual flow programming. Envoy provides the wiring; interfaces are consumers.
