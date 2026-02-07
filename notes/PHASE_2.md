# Phase 2: Client-Server Split

## Goal
Split the monolithic editor into client and server processes with IPC communication.

## Target
Support detach/reattach to editing sessions locally (like tmux attach).

## Core Features
1. Server daemon manages buffers and files
2. Client connects via Unix socket
3. Session persistence (survives client disconnect)
4. Multiple clients can attach to same session

## Architecture
- **Server daemon** - Long-running process managing state
- **Client** - Terminal UI connects to server
- **Protocol** - Custom message format over Unix socket
- **IPC** - Unix domain sockets for local communication

## Key Files
- `server/src/main.rs` - Server entry point
- `server/src/daemon.rs` - Server process management
- `server/src/session.rs` - Session state management
- `server/src/buffer_manager.rs` - Buffer lifecycle
- `common/src/protocol.rs` - Message definitions
- `common/src/types.rs` - Shared types
- `client/src/connection.rs` - Server connection handling

## Protocol Messages
```rust
enum Message {
    // Session management
    CreateSession { name: String },
    AttachSession { name: String },
    DetachSession,
    ListSessions,

    // File operations
    OpenFile { path: String },
    FileContent { id: u64, data: Vec<u8> },
    SaveFile { id: u64 },
    CloseFile { id: u64 },

    // Editing operations
    Edit { buffer_id: u64, change: Change },
    CursorMove { buffer_id: u64, position: Position },
}
```

## Dependencies
```toml
# server/Cargo.toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
bincode = "1"

# common/Cargo.toml
[dependencies]
serde = { version = "1", features = ["derive"] }
```

## Implementation Tasks
1. Define protocol in `common/src/protocol.rs`
   - Message types for all operations
   - Serialization/deserialization
2. Implement server daemon in `server/src/daemon.rs`
   - Listen on Unix socket
   - Handle multiple client connections
   - Route messages to sessions
3. Implement session manager in `server/src/session.rs`
   - Create/destroy sessions
   - Manage session state
   - Handle client attach/detach
4. Migrate buffer logic to server in `server/src/buffer_manager.rs`
   - Move Phase 1 buffer code to server
   - Expose via protocol messages
5. Update client to use protocol in `client/src/connection.rs`
   - Connect to server
   - Send/receive messages
   - Handle disconnection/reconnection
6. Add server lifecycle management
   - Server start/stop scripts
   - Socket cleanup
   - Graceful shutdown

## Success Criteria
- Can start server daemon
- Client connects to server successfully
- Can create named sessions
- Can detach and reattach to sessions
- Session state persists across client disconnects
- Multiple clients can view same session (readonly for now)
- Server handles client crashes gracefully
