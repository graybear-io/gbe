# Display Plane & Support Components

---

## Cryptum — The Display Plane

Crate: `crates/cryptum` (ttyd-connect)

Remote terminal access via ttyd (PTY over WebSocket) + native Rust client. Runs on the **client side** (macOS), not on the VM.

### Current Stack
- **ttyd** on Alpine VM — serves PTY over WebSocket (:7681)
- **ttyd-connect** (~150 lines Rust) — native macOS client, tungstenite WebSocket, raw terminal mode, SIGWINCH resize propagation

### ttyd Wire Protocol
1. Client sends JSON text on connect: `{"AuthToken":"","columns":N,"rows":N}`
2. Binary frames after handshake — ASCII prefix bytes:
   - `b'0'` (0x30) = terminal I/O (input from client, output from server)
   - `b'1'` (0x31) = resize (client→server JSON) / title (server→client)
   - `b'2'` (0x32) = preferences (server→client) / pause (client→server)

### How It Connects to Envoy
```
VM side                              macOS
────────                             ─────
adapter(cmd) ──envoy──► client       ttyd-connect
                         │              │
                      renders TUI    renders in
                         │           native terminal
                      ttyd :7681
                         │
                    ~~~WebSocket~~~►
```

The envoy client renders to a PTY. ttyd transports that PTY remotely. ttyd-connect displays it. Envoy has no awareness of ttyd.

### Future: Metarch
Multi-stream terminal compositor. Connects to N ttyd instances, renders each as a pane. Layout, focus, resize across streams.

### History
Six Wayland/VNC/RDP attempts failed on headless Alpine (no GPU). See `crates/cryptum/docs/vnc-migration-notes.md`. Smithay POC (phases 0-4) archived.

---

## Ark — The Foundation

**Location**: `gbe-ark/` (shell scripts, not part of Cargo workspace)

Alpine Linux VM provisioning for UTM on macOS. `constructor.sh` installs ttyd, openssh, fonts. Shared folder via 9p mounts macOS `~/projects` at `/mnt/projects`. ark-watch OpenRC service manages the ttyd lifecycle.

Ark is a **development** VM. It is not the production VM path — Sentinel uses Firecracker with ephemeral rootfs. The two are independent.

---

## Harness — The Learning Loop

**Location**: `gbe-harness/` (Python, not part of Cargo workspace)

Educational agentic loop framework. Calls Anthropic API with tool definitions, parses tool_use blocks, executes tools, feeds results back. Uses `anthropic` SDK directly. Built with uv + just. 80 tests.

Planned as the Python implementation of the Operative trait — wrapping the agent loop inside `Operative.execute()`.

---

## Overseer — The Interface

**Status**: Planned, not yet created.

Human command interface. Discovers available envoy sources, tells the display plane what to show. The bridge between envoy's control plane and cryptum's display plane.
