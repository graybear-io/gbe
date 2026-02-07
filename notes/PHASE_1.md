# Phase 1: Local TUI Prototype

## Goal
Build a minimal single-file text editor with basic editing capabilities in a terminal UI.

## Target
Edit and save a 10-line text file with basic navigation and modification.

## Core Features
1. Open/edit/save single file
2. Basic navigation (arrows, page up/down)
3. Insert/delete characters
4. Line numbers
5. Status bar

## Architecture
- **Single binary** - No client-server split yet
- **Rope data structure** - Efficient text buffer (ropey crate)
- **Terminal UI** - ratatui + crossterm for rendering
- **Event loop** - Handle keyboard input and render updates

## Key Files
- `client/src/main.rs` - Entry point, event loop
- `client/src/editor.rs` - Core editing logic
- `client/src/buffer.rs` - Rope-based text buffer
- `client/src/ui.rs` - Ratatui rendering
- `client/src/input.rs` - Keyboard/mouse handling

## Dependencies (client/Cargo.toml)
```toml
[dependencies]
ratatui = "0.28"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }
ropey = "1.6"
```

## Setup Tasks
1. Initialize cargo workspace structure
2. Create `client` binary crate
3. Create `common` library crate (for future phases)
4. Configure workspace in root `Cargo.toml`

## Implementation Tasks
1. Implement rope-based buffer in `client/src/buffer.rs`
   - Load file into rope
   - Insert/delete operations
   - Save to disk
2. Build terminal UI in `client/src/ui.rs`
   - Render buffer content
   - Line numbers
   - Status bar (filename, cursor position)
3. Handle input in `client/src/input.rs`
   - Arrow key navigation
   - Character insertion
   - Backspace/delete
   - Page up/down
4. Wire up event loop in `client/src/main.rs`
   - Initialize terminal
   - Input → Buffer → Render cycle
   - Exit handling (Ctrl+Q)

## Performance Targets
- Keystroke latency: <10ms
- File size: Up to 100MB
- Scrolling: 60fps smooth

## Success Criteria
- Can open a text file
- Can navigate with arrow keys
- Can insert and delete characters
- Can save changes to disk
- Can exit cleanly
- Handles files up to 10MB without lag
