# Phase 3: Multiplexing

## Goal
Add split panes and PTY management for tmux-like terminal multiplexing.

## Target
Support split panes with both text buffers and shell terminals.

## Core Features
1. Split panes (horizontal/vertical)
2. PTY management for shell processes
3. Window/tab system
4. Pane focus and navigation
5. Resize panes

## Architecture
- **Layout engine** - Calculate pane positions and sizes
- **Pane types** - Text buffer panes vs PTY panes
- **PTY manager** - Spawn and manage shell processes
- **Input routing** - Send input to focused pane

## Key Files
- `server/src/pty.rs` - PTY management
- `server/src/layout.rs` - Split pane layout calculations
- `client/src/pane.rs` - Pane rendering
- `client/src/layout_ui.rs` - Layout UI rendering
- `common/src/layout.rs` - Layout types (Tree, Split, Leaf)

## Protocol Extensions
```rust
enum Message {
    // ... existing messages ...

    // PTY operations
    SpawnShell { pane_id: u64 },
    ShellInput { pane_id: u64, data: Vec<u8> },
    ShellOutput { pane_id: u64, data: Vec<u8> },
    ShellExit { pane_id: u64, exit_code: i32 },

    // Layout operations
    SplitPane { pane_id: u64, direction: Direction },
    ClosePane { pane_id: u64 },
    FocusPane { pane_id: u64 },
    ResizePane { pane_id: u64, size: u16 },

    // Window operations
    CreateWindow,
    SwitchWindow { window_id: u64 },
    CloseWindow { window_id: u64 },
}
```

## Dependencies
```toml
# server/Cargo.toml
[dependencies]
portable-pty = "0.8"
```

## Implementation Tasks
1. Implement layout engine in `server/src/layout.rs`
   - Tree structure for panes
   - Split operations (horizontal/vertical)
   - Size calculations
   - Pane lifecycle
2. Add PTY management in `server/src/pty.rs`
   - Spawn shell processes
   - Capture stdout/stderr
   - Send stdin
   - Handle process exit
3. Extend protocol for multiplexing
   - Add pane and PTY messages
   - Update protocol version
4. Update client rendering in `client/src/pane.rs`
   - Render layout tree
   - Draw pane borders
   - Show focused pane
5. Add input routing in `client/src/input.rs`
   - Pane navigation keybindings
   - Route input to focused pane
   - Split/close pane commands
6. Implement window system
   - Window manager in server
   - Window switching UI
   - Per-window layouts

## Keybindings
- `Ctrl+B S` - Split horizontal
- `Ctrl+B V` - Split vertical
- `Ctrl+B Arrow` - Navigate panes
- `Ctrl+B X` - Close pane
- `Ctrl+B C` - New window
- `Ctrl+B N` - Next window
- `Ctrl+B P` - Previous window

## Success Criteria
- Can split panes horizontally and vertically
- Can spawn shell in a pane
- Can interact with shell (type commands, see output)
- Can navigate between panes
- Can close panes
- Can create and switch between windows
- Layout persists across detach/attach
- Pane borders render correctly
- Focused pane is highlighted
