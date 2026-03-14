# gbe-cryptum Roadmap

Created: 2026-02-25
Updated: 2026-03-14

## Completed — Smithay POC (Phases 0–4)

Phases 0–4 proved the compositor concept with a custom Smithay build.
Archived — replaced by ttyd display plane.

## Completed — Display Migration (Phase 5)

Six compositor/VNC attempts failed on Alpine VM (no GPU). Pivoted to
ttyd (PTY over WebSocket) + native Rust client. See
[vnc-migration-notes.md](vnc-migration-notes.md) for the full history.

**What works now:**
- ttyd on Alpine VM serves PTY over WebSocket
- `ttyd-connect` (Rust) connects from macOS, full interactive terminal
- Resize propagation, clean exit, no browser needed

## Current — Display Plane Integration (Phase 6)

| Task | Priority | Description |
|------|----------|-------------|
| Stream launcher | H | `start-cryptum.sh` launches `ttyd` wrapping `gbe-client` |
| Stream discovery | M | Advertise ttyd URLs via envoy `ToolInfo` metadata |
| Envoy wiring | M | End-to-end: adapter → router → client → ttyd → ttyd-connect |

## Future — Terminal Compositor (Phase 7 / Metarch)

| Task | Priority | Description |
|------|----------|-------------|
| Multi-stream client | M | Connect to N ttyd streams, render as panes |
| Overseer integration | M | Surface allocation API for gbe-overseer |
| Layout engine | L | Tiling, focus, resize across panes |
| libghostty render | L | Native terminal rendering instead of raw PTY passthrough |

## Stack

- **VM side:** ttyd (PTY server, WebSocket transport)
- **Client side:** ttyd-connect (Rust, tungstenite)
- **Future compositor:** Metarch (multi-stream terminal compositor)
- **Target VM OS:** Alpine Linux (UTM)

## Naming

- **Cryptum** — the display surface project (repo name)
- **Metarch** — the terminal compositor / multi-stream client (future)
- **ttyd-connect** — the native single-stream client (current binary)
