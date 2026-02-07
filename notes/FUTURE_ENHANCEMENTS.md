# Future Enhancements

Design notes for features deferred to Phase 3+.

## Keybinding Extensibility (Phase 3+)

### Current Architecture (Phase 1-2)
```
KeyEvent → InputHandler.handle_key() → EditorAction → Editor.handle_action()
```

**Status:** Hardcoded keybindings (arrows, Ctrl+S/Q)
**Location:** `client/src/input.rs:62-99`

### Cursor Control Types to Support

1. **Character-based** ✓ (current)
   - Arrow keys for single-character movement

2. **Word-based** (Phase 3+)
   - `MoveWordForward/Backward`
   - Typical bindings: `Ctrl+Left/Right` or `Alt+Left/Right`
   - Add to `EditorAction` enum

3. **Line-based** (Phase 3+)
   - `MoveToLineStart/End` (Home/End keys)
   - `MoveToFirstNonWhitespace` (vim's `^`)
   - Emacs: `Ctrl+A/E`

4. **Block-based** (Phase 4+, vim-style)
   - `MoveToNextParagraph/PreviousParagraph` (vim's `{/}`)
   - `MoveToMatchingBracket` (vim's `%`)
   - Requires: Modal editing system

5. **Screen-based**
   - `PageUp/PageDown` ✓ (current)
   - `MoveToScreenTop/Middle/Bottom` (vim's `H/M/L`)

### Extensibility Approaches

#### Option A: Mode-based System (vim-like)
```rust
enum EditorMode { Normal, Insert, Visual }

// Normal mode: 'h'→MoveLeft, 'j'→MoveDown
// Insert mode: 'h'→InsertChar('h')
```

**Use case:** vim emulation
**Complexity:** High
**Phase:** 4+

#### Option B: Keymap Profiles
```rust
enum KeymapProfile { Default, Vim, Emacs }
```

**Use case:** Multiple built-in keybinding schemes
**Complexity:** Medium
**Phase:** 3+

#### Option C: User-configurable Keybindings
```toml
# ~/.config/gbe-editor/keymap.toml
[normal_mode]
"h" = "move_left"
"<C-d>" = "page_down"
```

**Use case:** Full customization
**Complexity:** High
**Phase:** 4+

### Recommendation for Phase 3

**Add basic word/line navigation:**
- Extend `EditorAction` enum with word/line movements
- Keep hardcoded bindings (`Ctrl+Left/Right`, Home/End)
- Defer modal editing and config files to Phase 4+

**Rationale:** Phase 3 focuses on multiplexing. Simple navigation extensions don't require complex architecture changes.

---

## Visual Testing Strategy (Phase 3+)

### Current Approach (Phase 1-2)
- **Unit tests:** Buffer operations, cursor logic (26 tests)
- **Integration tests:** Behavior testing (12 tests)
- **Manual testing:** `MANUAL_TEST_GUIDE.md`
- **Visual rendering:** Trust ratatui library

**Status:** ✓ Adequate for simple single-pane editor

### Testing Options for Future Phases

#### Option 1: TestBackend + Snapshot Testing (Phase 3)

**What:** Ratatui's `TestBackend` captures rendered output as text

```rust
use ratatui::backend::TestBackend;
use ratatui::Terminal;

#[test]
fn test_split_pane_rendering() {
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| {
        render_split_layout(frame, &layout);
    }).unwrap();

    let buffer = terminal.backend().buffer();
    insta::assert_snapshot!(buffer);
}
```

**Dependencies:**
```toml
[dev-dependencies]
insta = "1.34"
```

**Pros:**
- Fast (no real terminal)
- Catches visual regressions
- Good for layout testing

**Cons:**
- No color support (limitation of TestBackend)
- Requires manual snapshot review
- Doesn't test real terminal compatibility

**Use case:** Split pane layouts in Phase 3

**References:**
- [Testing with insta snapshots | Ratatui](https://ratatui.rs/recipes/testing/snapshots/)
- [TestBackend docs](https://docs.rs/ratatui/latest/ratatui/backend/struct.TestBackend.html)

#### Option 2: ratatui-testlib (Phase 4+)

**What:** Runs TUI in real PTY with terminal emulator

```toml
[dev-dependencies]
ratatui-testlib = "0.3"
```

**Use case:** Integration tests with real terminal behavior

**Pros:**
- Tests actual terminal interactions
- Better color/escape sequence handling
- Ergonomic assertion API

**Cons:**
- Slower (spawns PTY)
- More complex setup
- Platform-specific behavior

**When needed:** Phase 4 remote support, complex PTY interactions

**References:**
- [ratatui-testlib docs](https://docs.rs/ratatui-testlib/latest/ratatui_testlib/)

#### Option 3: Manual Testing (Always Required)

**What can't be automated:**
- Cross-platform terminal compatibility (iTerm, Windows Terminal, Alacritty)
- Unicode rendering (emoji, CJK characters)
- Performance feel (60fps smoothness)
- Cursor blinking behavior
- Color scheme appearance

**Current coverage:** `MANUAL_TEST_GUIDE.md` ✓

### Recommendation for Phase 3

**Add snapshot testing for layouts:**
1. Add `insta` dependency
2. Create `TestBackend` tests for split pane rendering
3. Snapshot test: 2-way split, 3-way split, nested splits
4. Test pane borders, focus highlighting

**Defer to Phase 4+:**
- `ratatui-testlib` for PTY testing
- Complex terminal emulation tests

---

## Phase 1-2 Enhancement Opportunities (Deferred)

**Status:** Deferred per user decision

These are **not bugs**, just UX improvements:

1. **Page scroll size hardcoded** (20 lines)
   - Could use viewport height
   - Priority: Low

2. **No save confirmation**
   - Could show status bar message
   - Priority: Low

3. **No modified indicator**
   - Status bar doesn't show unsaved changes
   - Could add asterisk or "[Modified]"
   - Priority: Medium

4. **No confirm-on-quit**
   - Ctrl+Q exits immediately, even with unsaved changes
   - Priority: Medium (data loss prevention)

**Decision:** Defer all to Phase 3+ to avoid scope creep

---

## Server Lifecycle Scripts (Phase 2 - Deferred)

**Issue:** gbe-wwc
**Status:** Deferred - polish/convenience feature, not core Phase 2 functionality

### Requirements

**Problem:** Server process management is currently manual
- Users run `./target/release/gbe-server /tmp/gbe-server.sock`
- No standard way to check if server is running
- No easy graceful shutdown mechanism
- No PID tracking for the daemon

### Proposed Solution

Create lifecycle management scripts/commands:

#### 1. Start Script
```bash
gbe-server start [--socket <path>]
```

**Features:**
- Launch daemon in background (daemonize)
- Create PID file (e.g., `~/.gbe/server.pid`)
- Verify server started successfully
- Handle case where server is already running
- Default socket: `/tmp/gbe-server.sock` or `~/.gbe/server.sock`

#### 2. Stop Script
```bash
gbe-server stop
```

**Features:**
- Send SIGTERM to server process
- Wait for graceful shutdown (timeout: 5s)
- Remove PID file
- Clean up stale socket if needed
- Fallback to SIGKILL if timeout

#### 3. Status Command
```bash
gbe-server status
```

**Output:**
- Server running: Yes/No
- PID: 12345
- Socket: /tmp/gbe-server.sock
- Uptime: 2h 15m
- Active sessions: 3

#### 4. Socket Cleanup
```bash
gbe-server cleanup
```

**Features:**
- Detect stale sockets (no process listening)
- Remove orphaned socket files
- Clear stale PID files

### Implementation Options

**Option A: Shell Scripts**
```bash
scripts/
├── gbe-server-start.sh
├── gbe-server-stop.sh
└── gbe-server-status.sh
```

**Pros:** Simple, portable
**Cons:** Platform-specific (Unix only)

**Option B: Rust Subcommands**
```rust
// server/src/main.rs
enum Command {
    Start { socket: PathBuf },
    Stop,
    Status,
    Run { socket: PathBuf }, // current behavior
}
```

**Pros:** Cross-platform, integrated
**Cons:** More complex

**Option C: Separate CLI Tool**
```bash
gbe-ctl start
gbe-ctl stop
gbe-ctl status
```

**Pros:** Clean separation, tmux-like
**Cons:** Extra binary

### Why Deferred

**Current state is functional:**
- Server already handles graceful shutdown (Drop impl cleans socket)
- Socket cleanup works correctly
- Users can manually manage process (sufficient for Phase 2 MVP)

**Deferral rationale:**
- Focus Phase 2 on core client-server communication
- Lifecycle management is polish/UX, not core functionality
- Can be added post-Phase 2 without architectural changes
- Integration testing doesn't require it

### When to Implement

**Priority: Medium**
- Before Phase 3 public release
- When packaging for distribution
- If user feedback indicates difficulty managing server

**Recommended approach:** Option B (Rust subcommands)
- Better cross-platform support
- Consistent with cargo/git command patterns
- Can reuse existing daemon code

---

## Implementation Checklist

### Phase 3: Multiplexing
- [ ] Add word/line navigation to `EditorAction` enum
- [ ] Implement Home/End, Ctrl+Left/Right keybindings
- [ ] Add `insta` for snapshot testing
- [ ] Create layout rendering snapshot tests
- [ ] Test split pane borders and focus highlighting

### Phase 4: Remote Support
- [ ] Evaluate `ratatui-testlib` for PTY tests
- [ ] Consider modal editing system (vim emulation)
- [ ] Design keybinding config file format
- [ ] Implement keymap customization

---

## References

**Testing:**
- [Testing with insta snapshots | Ratatui](https://ratatui.rs/recipes/testing/snapshots/)
- [TestBackend in ratatui::backend](https://docs.rs/ratatui/latest/ratatui/backend/struct.TestBackend.html)
- [ratatui-testlib](https://lib.rs/crates/ratatui-testlib)

**Keybindings:**
- Current implementation: `client/src/input.rs:62-99`
- Phase 3 keybindings: `PHASE_3.md:87-94`
- Phase 4 keybindings: `PHASE_4.md:98-101`

---

**Last Updated:** 2026-02-06
**Status:** Design documentation for Phase 3+
