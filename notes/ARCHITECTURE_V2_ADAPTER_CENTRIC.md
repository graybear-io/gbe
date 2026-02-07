# Architecture V2: Adapter-Centric Design

**Date:** 2026-02-07
**Status:** Proposed
**Supersedes:** [V1 Layered Architecture](ARCHITECTURE_V1_LAYERED.md)

## Core Insight

**The adapter is the universal abstraction.**

Everything—terminals, log viewers, file editors, database queries, command output—is just:
1. A command to wrap
2. Input flowing down (stdin)
3. Output flowing up (stdout/stderr)
4. A protocol for structured messages
5. A client that renders

## Simplified Architecture

### Minimal Tool Set

```
gbe-router       Message broker (dumb forwarding)
gbe-adapter      Wrap any command (THE core tool)
gbe-client       Render + input
gbe-buffer       (Optional) Persistent storage for seeking/editing
```

That's it. Everything else is existing Unix tools.

### Architecture Diagram

```
┌─────────────┐
│ gbe-client  │  ← Your keyboard/screen
│             │  ← Renders output
│             │  ← Sends input
└─────────────┘
      ↕ Protocol (structured messages)
┌─────────────┐
│ gbe-router  │  ← Dumb message broker
│             │  ← Just forwards based on address
└─────────────┘
      ↕
┌─────────────┐
│ gbe-adapter │  ← Universal wrapper
│             │  ← Runs any command
│             │  ← Bridges stdin/stdout to protocol
└─────────────┘
      ↕ stdin/stdout
┌─────────────┐
│   Command   │  ← Any Unix tool or program
│             │  ← tail, grep, bash, psql, etc.
└─────────────┘
```

### Everything is Adapter + Command

**Terminal session:**
```
client → router → adapter("bash --pty")
  [user types "ls"]
  → stdin to bash
  ← stdout from bash
  ← adapter sends Lines([...])
  ← client renders
```

**Log viewing:**
```
client → router → adapter("tail -f /var/log/app.log")
  ← continuous stream of lines
  ← client renders (follow mode)
```

**Database query:**
```
client → router → adapter("psql -c 'SELECT * FROM users'")
  ← result rows as lines
  ← client renders as table
```

**File editing:**
```
client → router → adapter("file-buffer config.toml")
  [user edits line 10]
  → Edit(pos=10, text="new content")
  ← ViewUpdate(lines=[...])
  ← client renders
```

**Filtered pipeline:**
```
client → router → adapter("tail -f app.log | grep ERROR | awk '{print $3}'")
  ← filtered/transformed lines
  ← client renders
```

## Tool Responsibilities

### gbe-router

**What:** Dumb message broker
**Does:** Forward messages based on address
**Doesn't:** Any business logic, state management, coordination

```rust
fn route(msg: Message) {
    let conn = connections.get(msg.to);
    conn.send(msg);  // Just forward
}
```

### gbe-adapter

**What:** Universal command wrapper
**Does:**
- Spawn any command
- Read stdout → emit as Lines
- Receive Input → write to stdin
- Handle PTY for interactive commands
- Emit metadata (exit codes, stderr)

**Doesn't:**
- Know about other adapters
- Coordinate anything
- Have business logic

```rust
struct Adapter {
    command: String,
    child: Child,
    output_id: ToolId,
}

impl Adapter {
    fn run(&mut self) {
        // Read stdout line by line
        for line in self.child.stdout.lines() {
            self.send(Message {
                from: self.id,
                to: self.output_id,
                payload: Payload::Line(line),
            });
        }
    }

    fn handle_input(&mut self, text: String) {
        // Write to stdin
        self.child.stdin.write_all(text.as_bytes());
    }
}
```

### gbe-client

**What:** Render + input handler
**Does:**
- Render lines in terminal UI
- Capture keyboard/mouse input
- Send commands to adapter/buffer
- Handle splits/panes locally

**Doesn't:**
- Know about sources
- Know about other clients
- Coordinate anything

```rust
impl Client {
    fn render(&self, lines: Vec<Line>) {
        // Draw to terminal with ratatui
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key {
            Key::Char(c) => {
                self.send(Message {
                    to: self.buffer_id,
                    payload: Payload::Input(c.to_string()),
                });
            }
            // ... other keys
        }
    }
}
```

### gbe-buffer (Optional)

**What:** Persistent line storage with seek/edit
**Does:**
- Store lines (Rope for files, Ring for streams)
- Handle seek operations
- Handle insert/delete for editing
- Emit view updates

**Doesn't:**
- Know about sources
- Coordinate other buffers
- Have business logic

```rust
enum BufferType {
    Rope(Rope),      // Seekable, mutable (files)
    Ring(RingBuffer), // Fixed size, append-only (streams)
}

impl Buffer {
    fn append(&mut self, line: Line) {
        match &mut self.buffer_type {
            Rope(r) => r.insert(r.len(), line),
            Ring(r) => r.push(line),  // Drops oldest if full
        }
    }

    fn get_view(&self, window: ViewWindow) -> Vec<Line> {
        // Return requested range
    }
}
```

## Protocol

**All messages have same structure:**

```rust
struct Message {
    from: ToolId,
    to: ToolId,
    seq: u64,
    payload: Payload,
}

enum Payload {
    // Data flow (adapter → buffer/client)
    Line(String),
    Lines(Vec<String>),

    // Input (client → adapter)
    Input(String),

    // Edit operations (client → buffer)
    Insert { pos: Position, text: String },
    Delete { range: Range },

    // View requests (client → buffer)
    GetView { window: ViewWindow },
    ViewUpdate { lines: Vec<String> },

    // Control
    Subscribe { source_id: ToolId },
    Unsubscribe,

    // Metadata
    ExitCode(i32),
    Error(String),
}
```

**Binary or text?**
- Binary (bincode): Fast, compact
- Text (JSON): Debuggable, shell-friendly
- **Proposal:** Binary with debug mode

## Use Cases

### 1. Terminal Session (tmux replacement)

```bash
# Start router
gbe-router --socket /tmp/gbe.sock &

# Start shell adapter
gbe-adapter "bash" \
  --id shell1 \
  --pty \
  --router /tmp/gbe.sock &

# Connect client
gbe-client --input shell1 --router /tmp/gbe.sock
```

**What happens:**
- User types `ls -la`
- Client sends `Input("ls -la\n")` to shell1
- Adapter writes to bash stdin
- Adapter reads bash stdout, sends `Lines([...])`
- Client renders output

### 2. Log Monitoring (less/tail replacement)

```bash
# Start log adapter
gbe-adapter "tail -f /var/log/app.log | grep ERROR" \
  --id log1 \
  --output buf1 &

# Start ring buffer (keep last 1000 lines)
gbe-buffer --id buf1 --type ring --size 1000 &

# View it
gbe-client --input buf1
```

**What happens:**
- Adapter continuously reads tail output
- Sends Lines to buf1
- Buffer stores in ring (drops old lines)
- Client requests view, renders with follow mode

### 3. File Editing (vim replacement)

```bash
# Start file buffer (rope for editing)
gbe-buffer --id buf1 --type rope &
gbe-adapter "cat config.toml" --output buf1 &  # Load file

# Edit it
gbe-client --input buf1
```

**What happens:**
- User edits line 10
- Client sends `Insert(pos=10, text="new")`
- Buffer updates rope
- Buffer sends `ViewUpdate` back
- Client renders

**Or optimized:**
```bash
# Direct file adapter (no cat)
gbe-source-file config.toml --buffer buf1 &
gbe-client --input buf1
```

### 4. Database Query

```bash
# Run query
gbe-adapter "psql -c 'SELECT * FROM users WHERE active = true'" \
  --output buf1 &

# Buffer results
gbe-buffer --id buf1 --type rope &

# View as table
gbe-client --input buf1 --renderer table
```

### 5. Split View (multiple adapters)

```bash
# Client handles splits locally
gbe-client \
  --split-left shell1 \
  --split-right log1

# Each split connects to different adapter
```

## Coordination Without Managers

### Session Management (no manager process)

**Old way (V1):** SessionManager process coordinates everything

**New way (V2):** Session config file + startup tool

```toml
# ~/.config/gbe/sessions/dev.toml
router = "/tmp/gbe.sock"

[[adapter]]
id = "shell1"
command = "bash"
pty = true

[[adapter]]
id = "log1"
command = "tail -f /var/log/app.log | grep ERROR"
buffer = { type = "ring", size = 1000 }

[[client]]
split = "left"
input = "shell1"

[[client]]
split = "right"
input = "log1"
```

**Start session:**
```bash
gbe-session start dev
# 1. Reads config
# 2. Starts router
# 3. Starts adapters
# 4. Starts client with splits
# 5. Exits (no long-running process)
```

### Pipeline Wiring (no manager process)

**Old way (V1):** Pipeline manager coordinates flow

**New way (V2):** Tools specify outputs

```bash
# Adapter knows its output
gbe-adapter "tail -f app.log" --output buf1

# Buffer knows its ID
gbe-buffer --id buf1

# Client knows its input
gbe-client --input buf1

# Router just forwards messages
```

**Or use helper tool:**
```bash
gbe-pipeline create \
  "tail -f app.log" \
  | buffer ring:1000 \
  | view

# Translates to above commands
```

## Why This is Better Than V1

### V1 Problems

**8 layers with coordinators:**
- Layer 6: Buffer Manager (coordinates buffers)
- Layer 7: Session Manager (coordinates clients)
- Monolithic, stateful, complex

**Over-engineered:**
- `gbe-source-tail` reimplements `tail`
- `gbe-filter-grep` reimplements `grep`
- Custom filters instead of Unix tools

### V2 Advantages

**4 tools, all simple:**
- Each does one thing
- No coordinators
- No reimplementation

**True Unix philosophy:**
- Use existing tools (`tail`, `grep`, `bash`)
- Compose with protocol
- Small, testable components

**Flexibility:**
- Any command works immediately
- No need to implement new source types
- Just wrap with adapter

## Terminal as Adapter Pattern

**Key insight:** A terminal session is just an adapter wrapping a shell.

```
┌─────────────┐
│ gbe-client  │  ← Your keyboard/screen
└─────────────┘
      ↕
┌─────────────┐
│ gbe-adapter │  ← PTY wrapper
│   (PTY)     │
└─────────────┘
      ↕
┌─────────────┐
│   bash      │  ← The shell (existing tool)
└─────────────┘
```

**tmux/screen replacement:**
- Multiple adapters (one per shell)
- All connected to same router
- Client renders splits
- No central coordinator

## Adapter Variations

### 1. One-shot Command
```bash
gbe-adapter "ls -la"
```
- Run once
- Stream output
- Exit when done

### 2. Following Stream
```bash
gbe-adapter "tail -f /var/log/app.log"
```
- Run forever
- Stream lines continuously
- Client follow mode

### 3. Interactive (PTY)
```bash
gbe-adapter "bash" --pty
```
- Bidirectional
- Handle terminal control codes
- Full shell experience

### 4. With Buffer
```bash
gbe-adapter "cat large.log" --buffer ring:10000
```
- Stream through buffer
- Client can seek
- Fixed memory

### 5. Editable (File)
```bash
gbe-adapter "file-buffer config.toml" --buffer rope
```
- Bidirectional edit commands
- Full insert/delete/seek
- Save support

## Implementation Plan

### Phase 5: Core Protocol + Adapter

**Build:**
- `gbe-router` (message broker)
- `gbe-adapter` (universal wrapper)
- Protocol definitions
- Basic message passing

**Validate:**
- Can wrap `tail -f`
- Can wrap `bash` with PTY
- Performance (typing latency)

### Phase 6: Buffer + Client

**Build:**
- `gbe-buffer` (rope + ring)
- `gbe-client` (basic rendering)
- View updates
- Input handling

**Validate:**
- Can view live logs
- Can edit files
- Smooth rendering

### Phase 7: Multiplexing

**Build:**
- Client splits (local)
- Multiple adapters
- Session config
- `gbe-session` tool

**Validate:**
- tmux-like experience
- Multiple shells
- Detach/reattach

### Phase 8: Polish

**Build:**
- `gbe` CLI wrapper
- Better renderers
- Error handling
- Documentation

## Open Questions

1. **Protocol format:** Binary (fast) vs text (debuggable)?
2. **PTY handling:** How complex is interactive shell support?
3. **Performance:** Can we hit <10ms typing latency?
4. **Discovery:** How do tools find router socket?
5. **Security:** Sandbox adapters? Limit commands?

## Comparison with V1

| Aspect | V1 (Layered) | V2 (Adapter-Centric) |
|--------|--------------|----------------------|
| Core tools | 8+ specialized | 4 simple |
| Coordinators | Yes (managers) | No (just router) |
| Unix tools | Reimplemented | Used directly |
| Complexity | High (8 layers) | Low (4 tools) |
| Flexibility | Need new tools | Any command works |
| Maintenance | Many tools | Few tools |

## Why We Pivoted

**Original concern:** "Are we layering complexity for complexity's sake?"

**Answer:** Yes. V1 had too many layers and coordinators.

**Realization:** The adapter is the abstraction. Everything else is just:
- What command to wrap
- Whether we buffer it
- How we render it

**Result:** V2 is simpler, more flexible, more Unix-like.

---

**Status:** Proposed architecture for Phase 5+
**Next:** Define protocol in detail, build PoC
