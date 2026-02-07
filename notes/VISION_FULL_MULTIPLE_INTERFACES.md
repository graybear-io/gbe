# GBE: Universal Tool Composition Platform

**Date:** 2026-02-07
**Status:** Vision Document
**Version:** 3.0

## The Vision

GBE is not a "better shell" or a "text editor with streams."

**GBE is a universal substrate for tool composition with multiple interfaces.**

## Core Concept

Traditional shells (bash/zsh) provide ONE interface to tool composition: text commands.

GBE provides MULTIPLE interfaces to the SAME substrate:
- Text (traditional shell)
- AI (natural language)
- GUI (drag-n-drop)
- Visual programming (flow diagrams)
- Iconic (visual metaphors)

**All interfaces decompose to the same tool composition protocol.**

## Architecture

```
┌──────────────────────────────────────────────────────┐
│  Multiple Interfaces                                 │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │   Text   │ │    AI    │ │   GUI    │            │
│  │  (CLI)   │ │  (LLM)   │ │ (Visual) │            │
│  └──────────┘ └──────────┘ └──────────┘            │
│  ┌──────────┐ ┌──────────┐                         │
│  │  Visual  │ │  Iconic  │                         │
│  │  (Flow)  │ │ (Icons)  │                         │
│  └──────────┘ └──────────┘                         │
└──────────────────────────────────────────────────────┘
                      ↓
            ┌──────────────────┐
            │   Decomposer     │  ← Translates interface → tool chain
            └──────────────────┘
                      ↓
┌──────────────────────────────────────────────────────┐
│  Tool Composition Protocol (Substrate)               │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │  Router  │ │ Adapter  │ │  Buffer  │            │
│  └──────────┘ └──────────┘ └──────────┘            │
│  ┌──────────┐                                       │
│  │  Client  │                                       │
│  └──────────┘                                       │
└──────────────────────────────────────────────────────┘
                      ↓
┌──────────────────────────────────────────────────────┐
│  Tools (Unix + Custom)                               │
│  tail, grep, bash, psql, curl, custom...            │
└──────────────────────────────────────────────────────┘
```

## The Five Interfaces

### 1. Text Interface (Traditional Shell)

**What:** Bash/zsh-like command line
**Who:** Developers, power users
**Input:** Text commands with pipes

```bash
gbe> tail -f /var/log/app.log | grep ERROR | unique
```

**Decomposes to:**
```rust
ToolChain([
    Adapter("tail -f /var/log/app.log"),
    Adapter("grep ERROR"),
    Adapter("uniq"),
])
```

### 2. AI Interface (Natural Language)

**What:** LLM-powered assistant with gbe knowledge
**Who:** Anyone who can describe what they want
**Input:** Natural language requests

```
You: "Show me errors from the app log"

AI:  "I'll set up a pipeline to monitor errors.
      Creating: tail -f /var/log/app.log | grep ERROR
      Starting adapters now..."
      [Shows live output]

You: "Only show unique errors from the last hour"

AI:  "Adding timestamp filter and deduplication.
      Updated pipeline:
      tail -f app.log | timestamp-filter 1h | grep ERROR | unique
      [Updates view]
```

**AI has gbe knowledge:**
- Knows all available tools (Unix + custom)
- Understands composition patterns
- Can explain what chains do
- Can optimize tool chains
- Can debug failing pipelines

**Decomposes to:**
```rust
// AI translates natural language → tool chain
"Show me errors from app log"
  → ToolChain([
      Adapter("tail -f /var/log/app.log"),
      Adapter("grep ERROR"),
    ])
```

### 3. GUI Interface (Drag-n-Drop)

**What:** Visual tool palette and canvas
**Who:** DevOps, visual thinkers, non-programmers
**Input:** Drag tools, connect with arrows

```
┌────────────────────────────────────────┐
│  Tool Palette                          │
│  ┌──────┐ ┌──────┐ ┌──────┐          │
│  │ tail │ │ grep │ │ awk  │          │
│  └──────┘ └──────┘ └──────┘          │
│  ┌──────┐ ┌──────┐ ┌──────┐          │
│  │ psql │ │ curl │ │ view │          │
│  └──────┘ └──────┘ └──────┘          │
└────────────────────────────────────────┘

┌────────────────────────────────────────┐
│  Canvas                                │
│                                        │
│  ┌──────┐      ┌──────┐      ┌──────┐│
│  │ tail │─────→│ grep │─────→│ view ││
│  └──────┘      └──────┘      └──────┘│
│    │                                  │
│    │ Config: /var/log/app.log        │
│                                        │
└────────────────────────────────────────┘

[Run Pipeline] [Save As...] [Export Script]
```

**Features:**
- Drag tools from palette
- Connect with arrows
- Configure via properties panel
- Live output preview
- Save as reusable tool
- Export to text script

**Decomposes to:**
```rust
// GUI graph → tool chain
[tail node] connected to [grep node] connected to [view node]
  → ToolChain([
      Adapter("tail -f /var/log/app.log"),
      Adapter("grep ERROR"),
    ])
```

### 4. Visual Programming (Flow Diagrams)

**What:** Node-RED style flow editor
**Who:** Workflow builders, data engineers
**Input:** Nodes and wires in flow diagram

```
┌─────────────────────────────────────────────────────┐
│  Flow Editor                                        │
│                                                     │
│  [File Source] ──→ [Filter: grep "ERROR"] ──→ [Buffer: ring(1000)]
│       │                                              │
│       │            [Transform: jq]                   │
│       │                  ↓                           │
│  [DB Query]   ──────────→ [Merge] ───────→ [View]  │
│                                                     │
│  [HTTP Poll] ──→ [Alert on Change]                 │
│                                                     │
└─────────────────────────────────────────────────────┘
```

**Each node:**
- Represents a tool or adapter
- Has input/output ports
- Configurable via properties
- Can have multiple inputs (merge)
- Can have multiple outputs (split)

**Advanced features:**
- Conditional routing
- Loops and iteration
- Stateful nodes
- Subflows (composed nodes)
- Debug breakpoints

**Decomposes to:**
```rust
// Flow graph → tool chain with branching
Graph {
    nodes: [
        Node("file", Adapter("tail -f app.log")),
        Node("filter", Adapter("grep ERROR")),
        Node("buffer", Buffer(ring(1000))),
    ],
    edges: [
        ("file", "filter"),
        ("filter", "buffer"),
    ],
}
```

### 5. Iconic Programming (Visual Metaphors)

**What:** Icon-based composition
**Who:** Non-technical users, quick prototyping
**Input:** Drag icons representing tool categories

```
┌────────────────────────────────────────┐
│  Icon Palette                          │
│  📄 Files     🔍 Search   📊 Charts   │
│  🗄️ Database  ⚙️ Transform 📝 Edit    │
│  🌐 Network   🔔 Alert    💾 Save     │
│  🎨 Format    🔄 Loop     ⏱️ Schedule │
└────────────────────────────────────────┘

┌────────────────────────────────────────┐
│  Composition                           │
│                                        │
│  📄 ──→ 🔍 ──→ 📊                     │
│  (file)  (find)  (chart)              │
│                                        │
│  🗄️ ──→ ⚙️ ──→ 📝                     │
│  (db)   (filter) (edit as table)      │
│                                        │
│  🌐 ──→ 🔄 ──→ 🔔                     │
│  (API)  (poll)  (alert on change)     │
└────────────────────────────────────────┘
```

**Each icon:**
- Opens configuration dialog
- Shows preview of output
- Has intelligent defaults
- Can be named and saved

**Decomposes to:**
```rust
// Icons → tool chain
📄(app.log) → 🔍(ERROR) → 📊(count by hour)
  → ToolChain([
      Adapter("tail -f app.log"),
      Adapter("grep ERROR"),
      Adapter("awk script to count by hour"),
      Chart(type: bar, x: hour, y: count),
    ])
```

## The Decomposer

**Key abstraction:** Interface → Tool Chain translator

```rust
trait InterfaceDecomposer {
    /// Convert interface-specific input into universal tool chain
    fn decompose(&self, request: Request) -> Result<ToolChain>;

    /// Validate that the tool chain is valid
    fn validate(&self, chain: &ToolChain) -> Result<()>;

    /// Optimize the tool chain (combine filters, etc.)
    fn optimize(&self, chain: ToolChain) -> ToolChain;
}

struct ToolChain {
    tools: Vec<Tool>,
    connections: Vec<(ToolId, ToolId)>,
    metadata: ChainMetadata,
}

enum Tool {
    Adapter(String),           // Wrap Unix command
    Buffer(BufferSpec),        // Store lines
    Filter(FilterSpec),        // Transform/filter
    Custom(Box<dyn CustomTool>), // User-defined
}
```

### Text Decomposer

```rust
struct TextShellDecomposer;

impl InterfaceDecomposer for TextShellDecomposer {
    fn decompose(&self, request: Request) -> Result<ToolChain> {
        // Parse bash-like syntax
        let cmd = request.text;

        // "tail -f app.log | grep ERROR | view"
        let parts = parse_pipeline(cmd)?;

        let tools = parts.iter()
            .map(|part| Tool::Adapter(part.to_string()))
            .collect();

        Ok(ToolChain {
            tools,
            connections: sequential_connections(&tools),
            metadata: ChainMetadata::from_text(cmd),
        })
    }
}
```

### AI Decomposer

```rust
struct AIAssistantDecomposer {
    llm: LLMClient,
    tool_catalog: ToolCatalog,
}

impl InterfaceDecomposer for AIAssistantDecomposer {
    fn decompose(&self, request: Request) -> Result<ToolChain> {
        // Call LLM with gbe knowledge
        let prompt = format!(
            "Convert this request to gbe tool chain: {}\n\
             Available tools: {}\n\
             Previous context: {}",
            request.text,
            self.tool_catalog.list(),
            request.context,
        );

        let response = self.llm.call(prompt)?;

        // LLM returns structured tool chain
        let chain: ToolChain = parse_llm_response(response)?;

        // Validate and optimize
        self.validate(&chain)?;
        Ok(self.optimize(chain))
    }
}
```

### GUI Decomposer

```rust
struct DragDropGUIDecomposer;

impl InterfaceDecomposer for DragDropGUIDecomposer {
    fn decompose(&self, request: Request) -> Result<ToolChain> {
        // Request contains visual graph
        let graph = request.graph;

        let tools = graph.nodes.iter()
            .map(|node| Tool::from_node_type(node))
            .collect();

        let connections = graph.edges.iter()
            .map(|edge| (edge.from, edge.to))
            .collect();

        Ok(ToolChain {
            tools,
            connections,
            metadata: ChainMetadata::from_graph(graph),
        })
    }
}
```

## Composability at Every Level

### Tools Compose

```bash
tail | grep | awk
```

Each tool is independent, connected via protocol.

### Chains Compose

```bash
error-chain | unique | alert
```

A chain is a tool. Can be used in other chains.

### Saved Chains Become Tools

```bash
# Define once
gbe save error-monitor "tail -f app.log | grep ERROR | unique"

# Use everywhere
error-monitor | send-to-slack
```

The saved chain appears in all interfaces:
- Text: `error-monitor` command
- AI: Knows about "error-monitor" tool
- GUI: "error-monitor" appears in palette
- Visual: "error-monitor" node available
- Iconic: 🔴 icon for error-monitor

### Interfaces Compose

```bash
# Start with AI
AI: "Monitor errors"
  → Creates tool chain

# Refine with GUI
[Open in visual editor]
[Drag additional filters]
[Save changes]

# Export to script
[Export as bash script]
[Share with team]

# Someone else edits in text
vim error-monitor.gbe
# Changes visible in all interfaces
```

## This is Yahoo Pipes + Unix Philosophy

**Yahoo Pipes (RIP):**
- Visual web service composition
- Cloud-based
- Web services only
- Single interface (visual)

**GBE:**
- Visual (+ text + AI + iconic) tool composition
- Local or remote
- Any tool (Unix + custom + web)
- Multiple interfaces
- Open protocol

**But better:**
- Not limited to web services—any command/tool
- Not just visual—any interface you prefer
- Not just cloud—local, remote, or hybrid
- Open protocol—anyone can build interfaces
- Composable at every level

## Use Cases by Persona

### Developer (Text Interface)

**Daily workflow:**
```bash
gbe> ssh server1
gbe> tail -f /var/log/app.log | grep ERROR
gbe> psql -c "SELECT * FROM users WHERE created_at > NOW() - INTERVAL '1 hour'"
```

Like bash but with:
- All output buffered (unlimited scrollback)
- Persistent sessions (survives disconnects)
- Rich rendering (syntax highlighting)
- Built-in multiplexing (splits/panes)

### DevOps (GUI Interface)

**Build monitoring dashboard:**
1. Open GUI
2. Drag tools: [tail] [grep] [alert] [chart]
3. Connect: tail → grep → alert
4.           tail → chart
5. Configure each tool
6. Save as "prod-monitor"
7. Share with team

**Result:** Reusable monitoring tool that anyone can run.

### Manager (AI Interface)

**Ask questions:**
```
"Show me error trends for last week"
  → AI queries logs, aggregates, charts

"Alert me when errors spike above 100/hour"
  → AI sets up monitoring with threshold alert

"Compare prod vs staging errors"
  → AI creates split view with both environments
```

**No need to know commands—just describe what you want.**

### Data Analyst (Visual Programming)

**Build data pipeline:**
1. [DB Query] → [Transform: jq] → [Filter] → [Chart]
2. Configure each node
3. Add error handling
4. Schedule to run hourly
5. Export results to dashboard

**Like Jupyter notebooks but with tool composition.**

### Non-technical User (Iconic)

**Quick tasks:**
```
📄 (my-file.csv) → 🔍 (find "error") → 📧 (email to team)
🌐 (API endpoint) → 🔄 (poll every 5m) → 🔔 (notify on change)
```

**No programming required—just drag icons and configure.**

## AI Integration Details

### AI with GBE Knowledge

The AI assistant has deep knowledge of:
- All available tools (Unix + custom)
- Tool capabilities and flags
- Composition patterns
- Performance implications
- Best practices

**System prompt includes:**
```
You are a GBE assistant. You help users compose tools.

Available tools:
- Unix: tail, grep, sed, awk, curl, psql, ssh, ...
- Custom: gbe-filter-json, gbe-chart, gbe-alert, ...

You can:
- Translate natural language → tool chains
- Explain what tool chains do
- Suggest optimizations
- Debug failing pipelines
- Teach composition patterns

Always decompose requests to explicit tool chains.
Prefer Unix tools over custom tools.
Compose tools rather than writing complex commands.
```

### AI Conversation Examples

**Example 1: Simple request**
```
You: "Show me the last 100 lines of app.log"
AI:  "tail -n 100 /var/log/app.log"
     [Runs: gbe-adapter "tail -n 100 app.log"]
```

**Example 2: Complex workflow**
```
You: "I need to monitor three log files and alert on errors"
AI:  "I'll set up a multi-source monitoring pipeline:

     Pipeline design:
     [tail app1.log] ─┐
     [tail app2.log] ─┼→ [grep ERROR] → [unique] → [alert] → [view]
     [tail app3.log] ─┘

     This will:
     1. Monitor all three logs simultaneously
     2. Filter for ERROR lines
     3. Deduplicate repeated errors
     4. Alert when new errors appear
     5. Display in unified view

     Shall I start this?"

You: "Yes, but only alert if more than 5 errors in 1 minute"
AI:  "Updated with threshold alert:
     [...] → [alert --threshold 5 --window 1m] → [view]

     Starting now..."
```

**Example 3: Teaching**
```
You: "How do I filter JSON?"
AI:  "For JSON filtering, use jq. Here are common patterns:

     Extract field:
       cat data.json | jq '.users[].name'

     Filter by condition:
       cat data.json | jq '.[] | select(.age > 18)'

     Transform:
       cat data.json | jq '{name: .name, email: .contact.email}'

     Want me to set up a specific filter?"
```

**Example 4: Optimization**
```
You: "tail -f app.log | grep ERROR | grep -v EXPECTED | grep -v WARNING"
AI:  "I can optimize that with a single grep:
     tail -f app.log | grep 'ERROR' | grep -v 'EXPECTED\|WARNING'

     Or use awk for better performance:
     tail -f app.log | awk '/ERROR/ && !/EXPECTED/ && !/WARNING/'

     Which would you prefer?"
```

### AI Learning from Usage

**The AI can:**
- Learn user preferences (always use awk over multiple greps)
- Remember common workflows (save as aliases)
- Suggest improvements based on patterns
- Adapt to team conventions

**Example:**
```
AI: "I notice you often monitor logs with 'tail | grep ERROR'.
     Would you like me to save this as 'watch-errors' for quick access?"
```

## Protocol Substrate

The substrate we designed (V2 architecture) supports all interfaces:

### Core Components

```
gbe-router       Message broker (forwards between tools)
gbe-adapter      Universal wrapper (wraps any command)
gbe-buffer       Storage layer (rope/ring buffers)
gbe-client       Render + input (terminal UI)
```

### Protocol Messages

```rust
struct Message {
    from: ToolId,
    to: ToolId,
    seq: u64,
    payload: Payload,
}

enum Payload {
    // Data flow
    Line(String),
    Lines(Vec<String>),

    // Control
    Subscribe { source: ToolId },
    Unsubscribe,

    // Edit operations
    Insert { pos: Position, text: String },
    Delete { range: Range },

    // View requests
    GetView { window: ViewWindow },
    ViewUpdate { lines: Vec<String> },

    // Chain management
    CreateChain { spec: ToolChain },
    DestroyChain { id: ChainId },

    // Metadata
    ExitCode(i32),
    Error(String),
}
```

### All Interfaces Use Same Protocol

**Text interface:**
```bash
gbe> tail -f app.log
```
↓
```rust
CreateChain(ToolChain([Adapter("tail -f app.log")]))
Subscribe(adapter1)
// ... Lines([...])
```

**AI interface:**
```
"Show me app.log"
```
↓
```rust
// AI decomposer produces:
CreateChain(ToolChain([Adapter("tail -f app.log")]))
Subscribe(adapter1)
// ... Lines([...])
```

**GUI interface:**
```
[Drag tail node] [Connect to view]
```
↓
```rust
// GUI decomposer produces:
CreateChain(ToolChain([Adapter("tail -f app.log")]))
Subscribe(adapter1)
// ... Lines([...])
```

**Same protocol, different interfaces.**

## Implementation Roadmap

### Phase 5: Core Substrate (V2 Architecture)
- `gbe-router` (message broker)
- `gbe-adapter` (universal wrapper)
- `gbe-buffer` (rope + ring)
- `gbe-client` (basic TUI)
- Protocol definitions
- **Deliverable:** Text interface works

### Phase 6: Tool Composition
- ToolChain abstraction
- Chain serialization/persistence
- Saved chains as tools
- Chain library/catalog
- **Deliverable:** Can save and reuse chains

### Phase 7: AI Interface
- Decomposer abstraction
- AI decomposer with LLM
- Tool catalog for AI
- Conversation management
- **Deliverable:** Natural language → tool chains

### Phase 8: GUI Interface
- Visual canvas (web-based)
- Tool palette
- Drag-n-drop composition
- Properties panels
- **Deliverable:** Visual tool composition

### Phase 9: Visual Programming
- Node-RED style flow editor
- Advanced routing (branches, merges)
- Subflows (composed nodes)
- Debug/breakpoints
- **Deliverable:** Complex flow composition

### Phase 10: Iconic Interface
- Icon design system
- Icon-to-tool mapping
- Quick composition UI
- Smart defaults
- **Deliverable:** Non-technical user access

## Success Criteria

### Phase 5 (Substrate)
- Can wrap any Unix command
- Message passing <10ms latency
- Sessions survive disconnects
- Smooth terminal rendering

### Phase 6 (Composition)
- Can save tool chains
- Saved chains act as tools
- Can compose chains of chains
- Chain library searchable

### Phase 7 (AI)
- Natural language → tool chains
- AI explains what chains do
- AI suggests optimizations
- Conversational refinement

### Phase 8 (GUI)
- Drag-drop creates valid chains
- Visual matches text output
- Can export to script
- Can import from script

### Phase 9 (Visual)
- Can build complex flows
- Branching and merging
- Error handling
- Performance visualization

### Phase 10 (Iconic)
- Non-programmer can compose
- Icons are intuitive
- Defaults are sensible
- Quick prototyping

## Why This Matters

### Current State
**Tool composition requires:**
- Text-based shell (barrier for non-programmers)
- Command syntax knowledge (learning curve)
- Manual connection (no visualization)
- No AI assistance (must know what exists)

### With GBE
**Tool composition becomes:**
- Multiple interfaces (choose your style)
- Natural language (just describe it)
- Visual feedback (see what you're building)
- AI guidance (learn as you go)
- Universal (same tools, any interface)

### Impact

**Democratizes tool composition:**
- Developers: More productive (AI assistance)
- DevOps: Visual dashboards (GUI)
- Managers: Ask questions (AI)
- Analysts: Data pipelines (visual programming)
- Everyone: Access to power tools (iconic)

**Lowers barrier to automation:**
- "Show me errors" → automated monitoring
- Drag tools → instant dashboard
- Save workflow → reusable tool
- Share with team → everyone benefits

## Open Questions

1. **AI Model:** Self-hosted (Llama) vs cloud (GPT-4)?
2. **GUI Technology:** Web (Electron/Tauri) vs native?
3. **Visual Editor:** Custom vs existing (Node-RED fork)?
4. **Icon Design:** How many icons? Categories?
5. **Protocol:** Binary (speed) vs JSON (debug)?
6. **Security:** How to sandbox untrusted chains?
7. **Sharing:** Public tool chain registry?

## Related Work

### Inspiration
- **Yahoo Pipes:** Visual web service composition (RIP)
- **Node-RED:** Visual IoT flow programming
- **Zapier/IFTTT:** GUI workflow automation
- **Jupyter:** Notebook-style data analysis
- **bash/zsh:** Text-based tool composition

### Differences
GBE combines:
- Unix philosophy (compose small tools)
- Multiple interfaces (text/AI/visual)
- Universal substrate (any tool, any interface)
- Open protocol (extensible by anyone)

No single existing tool does all of this.

## The Name

**GBE Builds Everything**

A recursive acronym in the spirit of GNU (GNU's Not Unix), capturing the essence of composing tools to build anything.

Like the best Unix tools, GBE has layered meanings:
- **Official:** GBE Builds Everything (recursive)
- **Technical:** General Buffer Environment
- **Personal:** A nod to the creator's roots

The recursive nature reflects the core concept: tools compose into chains, chains compose into tools, infinitely composable—building everything from small parts.

---

**Status:** Vision document for full GBE platform
**Next:** Build Phase 5 (substrate), then incrementally add interfaces
**Timeline:** Substrate (Phase 5) → 2-3 months, AI (Phase 7) → +2 months, GUI (Phase 8) → +3 months
