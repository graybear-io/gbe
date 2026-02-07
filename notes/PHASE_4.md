# Phase 4: Remote Support

## Goal
Add SSH connectivity to edit files and run shells on remote machines.

## Target
Connect to remote server via SSH and edit files/run shells seamlessly.

## Core Features
1. SSH client integration
2. Remote file editing (file contents transferred over SSH)
3. Remote shell spawning
4. Connection management (connect/disconnect)
5. Multiple remote hosts

## Architecture
- **SSH client** - Establish and maintain SSH connections
- **Remote file sync** - Transfer file contents bidirectionally
- **Remote PTY** - Spawn shells on remote host
- **Connection pool** - Manage multiple SSH connections

## Key Files
- `client/src/ssh.rs` - SSH client implementation
- `server/src/remote.rs` - Remote file handling
- `server/src/remote_pty.rs` - Remote PTY management
- `common/src/remote.rs` - Remote connection types

## Protocol Extensions
```rust
enum Message {
    // ... existing messages ...

    // Remote connection
    ConnectSSH { host: String, user: String, auth: AuthMethod },
    DisconnectSSH { connection_id: u64 },
    ListConnections,

    // Remote file operations
    OpenRemoteFile { connection_id: u64, path: String },
    SaveRemoteFile { connection_id: u64, buffer_id: u64 },

    // Remote shell
    SpawnRemoteShell { connection_id: u64, pane_id: u64 },
}

enum AuthMethod {
    Password(String),
    PublicKey { private_key_path: String, passphrase: Option<String> },
    Agent,
}
```

## Dependencies
```toml
# client/Cargo.toml
[dependencies]
russh = "0.45"
russh-keys = "0.45"
```

## Implementation Tasks
1. Implement SSH client in `client/src/ssh.rs`
   - Connect to remote host
   - Authenticate (password, key, agent)
   - Maintain connection
   - Handle disconnection/reconnection
2. Add remote file operations in `server/src/remote.rs`
   - Open remote file via SSH
   - Read file content
   - Write file content
   - Handle remote file errors
3. Add remote PTY in `server/src/remote_pty.rs`
   - Spawn remote shell process
   - Forward stdin/stdout over SSH
   - Handle shell exit
4. Extend protocol for remote operations
   - Connection management messages
   - Remote file operation messages
5. Add connection UI in `client/src/ui.rs`
   - Connection status indicator
   - Host selector
   - Connection error display
6. Add SSH configuration support
   - Read ~/.ssh/config
   - Support SSH agent
   - Private key management

## SSH Configuration
Support standard SSH config files:
```
Host example
    HostName example.com
    User myuser
    Port 22
    IdentityFile ~/.ssh/id_rsa
```

## Keybindings
- `Ctrl+B R` - Connect to remote host
- `Ctrl+B D` - Disconnect from remote
- `Ctrl+B L` - List connections

## Success Criteria
- Can connect to remote host via SSH
- Can authenticate with password
- Can authenticate with SSH key
- Can authenticate with SSH agent
- Can open and edit remote files
- Can save changes back to remote
- Can spawn remote shell in pane
- Remote shell is interactive (type commands, see output)
- Handles network disconnection gracefully
- Can reconnect after temporary network failure
- Multiple remote hosts can be connected simultaneously
- Connection status visible in UI
