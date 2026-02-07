# GBE Architecture

A universal tool composition platform with multiple interfaces

## Vision

Traditional shells (bash/zsh) provide one way to compose tools: text commands. GBE provides multiple interfaces to the same powerful substrate, democratizing tool composition for everyone from developers to non-technical users.

- Text (shell)
- AI (natural language)
- GUI (drag-n-drop)
- Visual (flow diagrams)

All interfaces translate to the same tool composition protocol.

## Core Design

### The Adapter Pattern

Everything flows through a single abstraction:

```text
client → router → adapter(command) → stdin/stdout
```

The adapter wraps any tool that can generate lines of text, including structured text, e.g. JSON, JSONL, etc.:

- Terminals (bash, zsh)
- File operations (cat, vim)
- Log streams (tail -f)
- Database queries (psql)
- Network tools (curl, ssh)
- Custom commands (anything)

### Minimal Tool Set

Four simple tools compose the substrate:

1. gbe-router

- Message broker
- Dumb forwarding
- No business logic

1. gbe-adapter

- Universal command wrapper
- Bridges stdin/stdout to protocol
- Handles PTY for interactive shells

1. gbe-buffer

- Optional storage layer
- Rope (files, seekable)
- Ring (streams, fixed capacity)

1. gbe-client

- Terminal UI (ratatui)
- Renders output
- Captures input
- Handles splits locally

### Protocol

Simple message structure:

```rust
struct Message {
    from: ToolId,
    to: ToolId,
    seq: u64,
    payload: Payload,
}

enum Payload {
    Line(String),
    Lines(Vec<String>),
    Input(String),
    Insert { pos: Position, text: String },
    Delete { range: Range },
    GetView { window: ViewWindow },
    ViewUpdate { lines: Vec<String> },
    Subscribe { source: ToolId },
    Unsubscribe,
    ExitCode(i32),
    Error(String),
}
```

## Interfaces

### Text Interface

Bash-like shell with persistence and rich rendering.

```bash
gbe> tail -f /var/log/app.log | grep ERROR
```

Translates to:

```rust
ToolChain([
    Adapter("tail -f /var/log/app.log"),
    Adapter("grep ERROR"),
])
```

### AI Interface

Natural language translated to tool chains.

```text
you: "show me errors from the app log"

ai:  creating pipeline: tail -f /var/log/app.log | grep ERROR
     [shows live output]

you: "only unique errors from last hour"

ai:  updated: tail -f app.log | timestamp-filter 1h | grep ERROR | uniq
     [updates view]
```

AI can be given knowledge of GBE:

- Available tools (Unix + custom)
- Composition patterns
- Optimization strategies
- Debugging techniques

### GUI Interface

Visual tool palette and canvas.

```text
┌─────────────────────────┐
│ tools                   │
│ [tail] [grep] [awk]    │
│ [psql] [curl] [view]   │
└─────────────────────────┘

┌─────────────────────────┐
│ canvas                  │
│ [tail] → [grep] → [view]│
└─────────────────────────┘
```

Drag tools, connect with arrows, configure via properties.
Export to script or save as reusable tool.

### Visual Programming

Node-RED style flow editor.

```text
[file] → [filter] → [buffer]
  |                    |
[db] → [transform] → [merge] → [view]
```

Supports branching, merging, subflows, debugging.

## The Decomposer

Interface-specific translators to universal tool chains:

```rust
trait InterfaceDecomposer {
    fn decompose(&self, request: Request) -> Result<ToolChain>;
    fn validate(&self, chain: &ToolChain) -> Result<()>;
    fn optimize(&self, chain: ToolChain) -> ToolChain;
}

struct ToolChain {
    tools: Vec<Tool>,
    connections: Vec<(ToolId, ToolId)>,
    metadata: ChainMetadata,
}

enum Tool {
    Adapter(String),
    Buffer(BufferSpec),
    Filter(FilterSpec),
    Custom(Box<dyn CustomTool>),
}
```

Each interface implements the decomposer:

- Text: parse bash-like syntax
- AI: LLM with GBE knowledge base
- GUI: visual graph to tool chain
- Visual: node graph to tool chain

## Composability

### Tools Pipe Together

```bash
tail | grep | awk
```

Each tool independent, connected via protocol.

### Chains Pipe Together

```bash
error-chain | unique | alert
```

A saved chain is a tool, can be used in other chains.

### Saved Chains Become Tools

```bash
gbe save error-monitor "tail -f app.log | grep ERROR | uniq"

# Now available everywhere:
error-monitor | send-to-slack  # Text interface
"monitor errors" → AI uses it   # AI interface
[error-monitor] in palette     # GUI interface
```

### Interfaces Compose

Start in AI (describe it), refine in GUI (visualize it), export to script (share it).

## Use Cases

### Terminal Session (tmux replacement)

```bash
gbe-adapter "bash" --pty --id shell1 &
gbe-client --input shell1
```

Multiple adapters for splits:

```bash
gbe-client --split-left shell1 --split-right log1
```

### Log Monitoring (less/tail replacement)

```bash
gbe-adapter "tail -f /var/log/app.log | grep ERROR" \
  --id log1 --output buf1 &

gbe-buffer --id buf1 --type ring --size 1000 &

gbe-client --input buf1
```

Continuous stream with bounded memory, seekable history.

### File Editing (vim replacement)

```bash
gbe-buffer --id buf1 --type rope &
gbe-adapter "cat config.toml" --output buf1 &
gbe-client --input buf1
```

User edits send Insert/Delete messages, buffer updates rope.

### Database Query

```bash
gbe-adapter "psql -c 'SELECT * FROM users'" --output buf1 &
gbe-buffer --id buf1 --type rope &
gbe-client --input buf1 --renderer table
```

Query results as seekable table.

## Coordination Without Managers

No long-running coordinator processes. Configuration + startup tools.

### Session Config

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

Start session:

```bash
gbe-session start dev
# Reads config, starts router/adapters/client, exits
```

No manager process. Tools read from sources and write to targets. They appear as pipes to users but communicate via messages.

## Implementation Roadmap

### Phase 5: Core Substrate

- gbe-router (message broker)
- gbe-adapter (universal wrapper)
- gbe-buffer (rope + ring)
- gbe-client (basic TUI)
- Protocol definitions
- Text interface works

### Phase 6: Tool Composition

- ToolChain abstraction
- Chain save/load/catalog
- Saved chains as tools
- Chain library

### Phase 7: AI Interface

- Decomposer abstraction
- AI decomposer with LLM
- Tool catalog for AI
- Conversation management
- Natural language → tool chains

### Phase 8: GUI Interface

- Visual canvas (web-based)
- Tool palette
- Drag-n-drop composition
- Properties panels
- Visual tool composition

### Phase 9: Visual Programming

- Node-RED style flow editor
- Advanced routing (branches, merges)
- Subflows (composed nodes)
- Debug/breakpoints
- Complex flow composition

## Success Criteria

### Phase 5 (Substrate)

- Wrap any Unix command
- Message passing <10ms latency
- Sessions survive disconnects
- Smooth terminal rendering

### Phase 6 (Composition)

- Save tool chains
- Saved chains act as tools
- Compose chains of chains
- Chain library searchable

### Phase 7 (AI)

- Natural language → tool chains
- AI explains what chains do
- AI suggests optimizations
- Conversational refinement

### Phase 8 (GUI)

- Drag-drop creates valid chains
- Visual matches text output
- Export to script
- Import from script

### Phase 9 (Visual)

- Build complex flows
- Branching and merging
- Error handling
- Performance visualization

## Why This Matters

### Current State

Tool composition requires:

- Text-based shell (barrier for non-programmers)
- Command syntax knowledge (learning curve)
- Manual connection (no visualization)
- No AI assistance (must know what exists)

### With GBE

Tool composition becomes:

- Multiple interfaces (choose your style)
- Natural language (just describe it)
- Visual feedback (see what you're building)
- AI guidance (learn as you go)
- Universal (same tools, any interface)

### Impact

Democratizes tool composition:

- Developers: more productive (AI assistance)
- DevOps: visual dashboards (GUI)
- Managers: ask questions (AI)
- Analysts: data pipelines (visual programming)

Lowers barrier to automation:

- "Show me errors" → automated monitoring
- Drag tools → instant dashboard
- Save workflow → reusable tool
- Share with team → everyone benefits

## Comparison With Existing Tools

### Inspiration

- Yahoo Pipes: visual web service composition (RIP)
- Node-RED: visual IoT flow programming
- Zapier/IFTTT: GUI workflow automation
- Jupyter: notebook-style data analysis
- bash/zsh: text-based tool composition

### Differences

GBE combines:

- Unix philosophy (compose small tools)
- Multiple interfaces (text/AI/visual)
- Universal substrate (any tool, any interface)
- Open protocol (extensible by anyone)

No single existing tool does all of this.

## The Name: GBE Builds Everything

Recursive acronym in the spirit of GNU (GNU's Not Unix).
Captures the essence of composing tools to build anything.

The recursive nature reflects the core concept:
tools pipe into chains, chains pipe into tools,
infinitely composable—building everything from small parts.

## Design Evolution

### V1: Layered Architecture (rejected)

8-layer design with specialized tools and coordinator layers.
Problems: too complex, reimplemented Unix tools, violated "small tools" philosophy.

### V2: Adapter-Centric (current)

4 simple tools + existing Unix utilities.
Key insight: use existing tools via adapter. No coordinators, just message passing.
Why better: simpler, more flexible, true Unix philosophy.

## Open Questions

1. AI model: self-hosted (Llama) vs cloud (GPT-4)?
2. GUI technology: web (Electron/Tauri) vs native?
3. Visual editor: custom vs existing (Node-RED fork)?
4. Protocol: binary (speed) vs JSON (debug)?
5. Security: how to sandbox untrusted chains?
6. Sharing: public tool chain registry?
