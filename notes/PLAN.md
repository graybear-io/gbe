# Terminal Editor Implementation Plan

## Architecture
**Model:** Hybrid client-server with remote support
- Server: Daemon managing buffers, files, PTYs
- Client: Terminal UI for rendering/input
- Protocol: Custom over SSH/TCP socket

## Tech Stack
- **Language:** Rust (memory safety, async)
- **UI:** ratatui + crossterm (cross-platform)
- **Async:** tokio (network + UI events)
- **Text:** ropey (rope data structure)
- **PTY:** portable-pty (shell management)
- **Remote:** russh (SSH connections)

## Project Structure
```
editor/
├── client/      # Terminal UI (ratatui)
├── server/      # Daemon (buffers, files, PTYs)
├── common/      # Shared protocol, types
└── protocol/    # Client-server messages
```

## Implementation Phases

### Phase 1: Local TUI Prototype
- Basic text editing (insert, delete, navigate)
- File open/save via `src/editor.rs:1`
- Rope buffer in `src/buffer.rs:1`
- Crossterm input handling
- **Target:** 10-line file editing

### Phase 2: Client-Server Split
- Server daemon in `server/src/main.rs:1`
- Unix socket IPC
- Buffer synchronization protocol
- Session persistence
- **Target:** Detach/reattach locally

### Phase 3: Multiplexing
- Split panes layout engine
- PTY management for shells
- Window/tab system
- **Target:** tmux-like splits

### Phase 4: Remote Support
- SSH client integration
- Remote file editing
- Remote shell spawning
- **Target:** Edit files over SSH

## Key Files

### Phase 1 (Local Prototype)
- `client/src/main.rs` - Entry point, event loop
- `client/src/editor.rs` - Core editing logic
- `client/src/buffer.rs` - Rope-based text buffer
- `client/src/ui.rs` - Ratatui rendering
- `client/src/input.rs` - Keyboard/mouse handling

### Phase 2 (Client-Server)
- `server/src/daemon.rs` - Server process
- `server/src/session.rs` - Session management
- `common/src/protocol.rs` - Message definitions
- `common/src/types.rs` - Shared types

### Phase 3 (Multiplexing)
- `server/src/pty.rs` - PTY management
- `client/src/layout.rs` - Split pane layout
- `client/src/pane.rs` - Pane rendering

### Phase 4 (Remote)
- `client/src/ssh.rs` - SSH client
- `server/src/remote.rs` - Remote file handling

## Protocol Messages
```rust
// common/src/protocol.rs
enum Message {
    OpenFile { path: String },
    FileContent { id: u64, data: Vec<u8> },
    Edit { buffer_id: u64, change: Change },
    SpawnShell { pane_id: u64 },
    ShellOutput { pane_id: u64, data: Vec<u8> },
    Attach { session: String },
    Detach,
}
```

## Dependencies (Cargo.toml)
```toml
[workspace]
members = ["client", "server", "common"]

# client/Cargo.toml
[dependencies]
ratatui = "0.28"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }
ropey = "1.6"

# server/Cargo.toml
[dependencies]
tokio = { version = "1", features = ["full"] }
portable-pty = "0.8"
russh = "0.45"
```

## Performance Targets
- Keystroke latency: <10ms
- File size: Up to 100MB
- Scrolling: 60fps smooth

## MVP Features (Phase 1)
1. Open/edit/save single file
2. Basic navigation (arrows, page up/down)
3. Insert/delete characters
4. Line numbers
5. Status bar

## Future Enhancements
- Syntax highlighting (tree-sitter)
- Multiple cursors
- Search/replace
- Plugin system
- Collaborative editing

## Next Actions
1. `cargo init --lib common` in `~/projects/editor/`
2. `cargo new client` for TUI client
3. `cargo new server` for daemon
4. Configure workspace in root `Cargo.toml`
5. Implement basic buffer in `client/src/buffer.rs`
