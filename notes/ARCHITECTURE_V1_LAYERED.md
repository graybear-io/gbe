# Line Stream Architecture Vision

## Overview

Generalization from "file editor" to "universal line-oriented stream multiplexer with composable filters."

**Core Abstraction:** Everything that flows across a PTY (line-oriented text) can be treated as a stream or buffer, filtered, transformed, and rendered in a unified interface.

**Philosophy:** Unix composition model—small parts, text streams, composable filters—extended to become the primary interface for daily work.

## Layer Architecture

```
┌─────────────────────────────────────────────┐
│  Layer 8: Client (Presentation)             │
│  - Render streams                           │
│  - Handle input                             │
│  - UI multiplexing (panes/splits)           │
└─────────────────────────────────────────────┘
              ↕ Protocol API
┌─────────────────────────────────────────────┐
│  Layer 7: Session Manager                   │
│  - Named sessions                           │
│  - Multi-client coordination                │
│  - View state per client                    │
└─────────────────────────────────────────────┘
              ↕ Session API
┌─────────────────────────────────────────────┐
│  Layer 6: Buffer Manager                    │
│  - Buffer lifecycle (create/destroy)        │
│  - View management (cursors/windows)        │
│  - Buffer metadata (type, state, position)  │
└─────────────────────────────────────────────┘
              ↕ Buffer API
┌─────────────────────────────────────────────┐
│  Layer 5: Filter Pipeline                   │
│  - Composable transforms                    │
│  - grep/sed/awk-like operations             │
│  - Stateful aggregations                    │
└─────────────────────────────────────────────┘
              ↕ Filter API
┌─────────────────────────────────────────────┐
│  Layer 4: Line Buffer                       │
│  - Rope (seekable, mutable)                 │
│  - Ring (live streams, sliding window)      │
│  - Operations: read, write, seek, append    │
└─────────────────────────────────────────────┘
              ↕ Buffer Storage API
┌─────────────────────────────────────────────┐
│  Layer 3: Stream Adapter                    │
│  - Normalize to line streams                │
│  - Handle source lifecycle                  │
│  - Buffering/backpressure                   │
└─────────────────────────────────────────────┘
              ↕ Adapter API
┌─────────────────────────────────────────────┐
│  Layer 2: Source Connector                  │
│  - Source-specific protocol                 │
│  - Connection management                    │
│  - Error handling/reconnection              │
└─────────────────────────────────────────────┘
              ↕ Raw Data
┌─────────────────────────────────────────────┐
│  Layer 1: Sources                           │
│  - File, Socket, Command, Log, DB, API      │
└─────────────────────────────────────────────┘
```

## Contract APIs

### Layer 1→2: Source → Stream Adapter

```rust
trait SourceConnector {
    fn connect() -> Result<Source>;
    fn read(&mut self) -> Result<Bytes>;
    fn reconnect(&mut self) -> Result<()>;
    fn close(&mut self);
}

// Examples:
// - FileConnector: Read from filesystem
// - SocketConnector: TCP/Unix socket streams
// - CommandConnector: Spawn process, capture stdout/stderr
// - DbConnector: Execute queries, stream results
// - LogConnector: tail -f style following
// - ApiConnector: HTTP streams, SSE, WebSocket
```

### Layer 2→3: Stream Adapter → Line Buffer

```rust
trait StreamAdapter {
    fn next_line(&mut self) -> Option<Line>;
    fn is_live(&self) -> bool;      // true = infinite stream
    fn is_seekable(&self) -> bool;  // true = can seek backwards
}

struct Line {
    content: String,
    metadata: LineMetadata,  // timestamp, source, tags, etc.
}
```

### Layer 3→4: Line Buffer Storage

```rust
trait LineBuffer {
    fn append(&mut self, line: Line);
    fn read(&self, range: Range) -> Vec<Line>;
    fn seek(&mut self, pos: Position);
    fn len(&self) -> usize;
    fn buffer_type(&self) -> BufferType;
}

enum BufferType {
    Rope,         // Seekable, mutable (files, DB results)
    Ring(usize),  // Live stream, fixed capacity, sliding window
}
```

**Key distinction:**
- **Rope buffers:** Full editing (insert/delete), seekable, used for files
- **Ring buffers:** Append-only, fixed size, auto-scroll, used for live streams

### Layer 4→5: Filter Pipeline

```rust
trait Filter {
    fn process(&mut self, line: Line) -> Option<Line>;
    fn reset(&mut self);
}

struct FilterPipeline {
    filters: Vec<Box<dyn Filter>>,
}

// Built-in filters:
// - GrepFilter(pattern): Match lines by regex
// - SedFilter(pattern, replacement): Transform line content
// - AwkFilter(script): Extract/process fields
// - TailFilter(n): Keep last N lines
// - HeadFilter(n): Keep first N lines
// - UniqueFilter: Deduplicate consecutive lines
// - TimestampFilter: Parse and filter by time ranges
```

### Layer 5→6: Buffer Manager

```rust
trait BufferManager {
    fn create_buffer(&mut self, source: Source) -> BufferId;
    fn attach_filter(&mut self, buffer_id: BufferId, filter: Box<dyn Filter>);
    fn get_view(&self, buffer_id: BufferId, window: ViewWindow) -> Vec<Line>;
    fn insert(&mut self, buffer_id: BufferId, pos: Position, text: String);
    fn destroy_buffer(&mut self, buffer_id: BufferId);
}

struct ViewWindow {
    start: usize,     // Line offset
    count: usize,     // Lines to return
    follow: bool,     // Auto-scroll to end (live streams)
}
```

### Layer 6→7: Session Manager

```rust
trait SessionManager {
    fn create_session(&mut self, name: String) -> SessionId;
    fn attach_client(&mut self, session_id: SessionId) -> ClientId;
    fn detach_client(&mut self, client_id: ClientId);
    fn list_buffers(&self, session_id: SessionId) -> Vec<BufferInfo>;
}
```

### Layer 7→8: Protocol → Client

```rust
enum Message {
    // Buffer lifecycle
    BufferCreated { id: BufferId, buffer_type: BufferType },
    BufferClosed { id: BufferId },

    // Stream events
    LinesAppended { buffer_id: BufferId, lines: Vec<Line> },
    LinesChanged { buffer_id: BufferId, range: Range, lines: Vec<Line> },

    // View updates
    ViewUpdate { buffer_id: BufferId, lines: Vec<Line>, cursor: Position },

    // Filter control
    FilterAttached { buffer_id: BufferId, filter: FilterSpec },
    FilterResults { buffer_id: BufferId, matched: usize, total: usize },
}
```

### Layer 8: Client Rendering

```rust
trait BufferRenderer {
    fn render(&self, buffer: &BufferView, area: Rect) -> Widget;
}

// Different renderers for different buffer types:
// - FileRenderer: Editable, line numbers, syntax highlighting
// - LogRenderer: Timestamps, log levels, follow mode
// - TableRenderer: Columnar data, headers, sorting
// - JsonRenderer: Syntax highlighting, collapsible trees
// - TerminalRenderer: ANSI escape codes, scrollback
```

## Key Insights

### Everything Becomes a Buffer

**Files → Rope buffer:**
- Seekable, editable
- Full text manipulation
- Persistent storage

**Live logs → Ring buffer:**
- Append-only, sliding window
- Follow mode (auto-scroll)
- Fixed memory footprint

**Command output → Ring buffer:**
- Finite stream (process exits)
- Captured stdout/stderr
- Seekable after completion

**DB queries → Rope buffer:**
- Seekable result set
- Read-only or editable (with UPDATE)
- Can re-query to refresh

**API streams → Ring buffer:**
- Live events (SSE, WebSocket)
- Fixed capacity, discard old
- Follow mode for real-time

### Composable Filter Pipelines

Unix-style composition at the buffer level:

```bash
# Example 1: Live log monitoring
source: tail -f /var/log/app.log
  | filter: grep "ERROR"
  | filter: sed 's/timestamp//'
  | filter: awk '{print $3}'
  → buffer: ring(1000)
  → client: render with follow=true

# Example 2: Database exploration
source: psql "SELECT * FROM users"
  | filter: grep "@gmail.com"
  | filter: awk '{print $1, $3}'
  → buffer: rope (seekable)
  → client: editable table view

# Example 3: API monitoring
source: curl -N https://api.example.com/events
  | filter: jq '.level == "error"'
  | filter: unique
  → buffer: ring(500)
  → client: JSON renderer
```

### Client Agnostic to Source

The client:
- **Doesn't care** about source type
- **Receives** `ViewUpdate` messages
- **Renders** based on `BufferType` + metadata
- **Sends** input commands back to server

This allows:
- Same UI for files, logs, commands, DB queries, APIs
- Different renderers based on content type
- Consistent editing model across sources

## Use Cases

### Replace Existing Tools

**vim → File editing:**
```
gbe edit config.toml
```

**less/tail → Log viewing:**
```
gbe tail /var/log/app.log | grep ERROR
```

**tmux → Multiplexing:**
```
gbe session work
  split: gbe edit main.rs
  split: gbe tail logs/debug.log
  split: gbe run "cargo test --watch"
```

**psql/mysql → Database:**
```
gbe db "postgresql://localhost/mydb" \
  --query "SELECT * FROM users WHERE active = true"
```

**curl → API monitoring:**
```
gbe stream https://api.example.com/events \
  | jq '.type == "error"'
```

### New Capabilities

**Persistent filtered views:**
```
# Create named filter, survives restarts
gbe tail /var/log/app.log \
  --save-filter "errors" \
  --filter "grep ERROR | grep -v EXPECTED"
```

**Multi-source aggregation:**
```
# Merge multiple log sources
gbe merge \
  /var/log/app1.log \
  /var/log/app2.log \
  ssh://server2/var/log/app.log \
  | grep "transaction_id" \
  | sort-by-timestamp
```

**Collaborative viewing:**
```
# Multiple clients see same view
gbe attach-session production-logs
```

## Implementation Phases

### Phase 5: Stream Abstraction (Foundation)
- Implement `LineBuffer` trait (Rope + Ring)
- Build `StreamAdapter` interface
- Create basic filters (grep, sed, awk)
- Refactor current buffer to use new abstraction

### Phase 6: Source Connectors
- File (current implementation)
- Command (spawn process, capture output)
- Socket (TCP/Unix stream)
- Log (tail -f style following)

### Phase 7: Filter Pipeline
- Composable filter chain
- Filter persistence (save named filters)
- Performance optimization (lazy evaluation)

### Phase 8: Advanced Renderers
- Log renderer (timestamps, levels)
- Table renderer (columnar data)
- JSON renderer (syntax + collapse)
- Terminal renderer (ANSI escapes)

### Phase 9: Extended Sources
- Database connectors (PostgreSQL, MySQL)
- HTTP streams (SSE, WebSocket)
- Cloud logs (CloudWatch, Stackdriver)

## Technical Considerations

### Performance

**Ring buffers:**
- Fixed memory footprint
- O(1) append, O(1) read recent lines
- Configurable capacity (default 10K lines)

**Filter pipelines:**
- Lazy evaluation where possible
- Parallel processing for independent filters
- Caching for expensive transforms

**Backpressure:**
- Slow clients don't block fast sources
- Ring buffer auto-discards old data
- Client gets "data skipped" notification

### State Management

**Seekable buffers (Rope):**
- Full history in memory or mmap
- Undo/redo stack
- Dirty tracking for saves

**Live streams (Ring):**
- No full history (bounded memory)
- Position = offset from current tail
- "Follow mode" locks to tail

### Error Handling

**Source failures:**
- Connector retries with backoff
- Buffer remains open, shows "disconnected"
- Client can trigger manual reconnect

**Filter errors:**
- Log to stderr, continue pipeline
- Option to halt on error
- Partial results visible to client

## Open Questions

1. **Filter composition syntax:** Rust API vs. shell-style pipes?
2. **Binary protocols:** Handle non-text (images, audio) or stay text-only?
3. **Persistence:** Save buffer contents to disk? Which buffer types?
4. **Security:** Sandbox filters? Limit source types?
5. **Performance:** When does a ring buffer become too expensive?

## Related Work

- **tmux/screen:** Terminal multiplexing (inspiration for sessions)
- **less/tail:** Stream viewing (inspiration for ring buffers)
- **vim/emacs:** Text editing (inspiration for rope buffers)
- **Unix pipes:** Composable filters (inspiration for filter pipelines)
- **Kafka/Pulsar:** Distributed log streams (inspiration for live streams)
- **jq/yq:** JSON/YAML processing (inspiration for structured filters)

## Status

**Current Phase:** Research/Exploration (Phase 5 planning)
**Branch:** `research/line-stream-architecture`
**Blocking:** No current work blocked, exploratory only
**Next Steps:** Prototype `LineBuffer` trait, validate assumptions
