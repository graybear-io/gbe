# GBE - GBE Builds Everything

**A universal tool composition platform with multiple interfaces.**

GBE is a substrate for composing Unix tools and commands through multiple interfaces: traditional shell (text), AI (natural language), GUI (drag-n-drop), visual programming (flow diagrams), and iconic (visual metaphors).

> *GBE Builds Everything* - A recursive acronym in the spirit of GNU (GNU's Not Unix)

## Project Vision

**From:** Text editor → **To:** Universal tool composition platform

Traditional shells (bash/zsh) provide one way to compose tools: text commands. GBE provides multiple interfaces to the same powerful substrate, democratizing tool composition for everyone from developers to non-technical users.

See [Vision Document](notes/VISION_FULL_MULTIPLE_INTERFACES.md) for complete vision.

## Project Status

**Current Phase:** Phase 2 - Client-Server Split (Complete) → Phase 5 Planning

- ✅ Phase 1: Terminal UI Editor (Complete)
- ✅ Phase 2: Client-Server Split (Complete)
- 📋 Phase 3: Terminal Multiplexing (Deferred)
- 📋 Phase 4: Collaborative Editing (Deferred)
- 🎯 Phase 5: Core Substrate - Adapter/Router/Protocol (Next)
- 📋 Phase 6: Tool Composition & Chain Library
- 📋 Phase 7: AI Interface (Natural Language)
- 📋 Phase 8: GUI Interface (Drag-n-Drop)
- 📋 Phase 9: Visual Programming (Flow Editor)
- 📋 Phase 10: Iconic Interface

## Documentation

### 🎯 Vision & Architecture
**Start here to understand GBE:**
- [Vision: Multiple Interfaces](notes/VISION_FULL_MULTIPLE_INTERFACES.md) - Complete platform vision
- [Architecture V2: Adapter-Centric](notes/ARCHITECTURE_V2_ADAPTER_CENTRIC.md) - Current design
- [Architecture V1: Layered](notes/ARCHITECTURE_V1_LAYERED.md) - Historical (archived)
- [Line Stream Vision](notes/LINE_STREAM_VISION.md) - Overview and evolution

### 📚 [docs/](docs/)
User-facing documentation:
- [Manual Test Guide](docs/MANUAL_TEST_GUIDE.md) - Testing procedures

### 📝 [notes/](notes/)
Design documents and planning:
- [Project Plan](notes/PLAN.md) - Overall roadmap
- [Session Handoffs](notes/SESSION_HANDOFF.md) - Progress tracking
- [Agent Instructions](notes/AGENTS.md) - Development workflow
- [Diary](notes/diary/) - Design evolution journal

## Quick Start

### Building

```bash
cargo build --release --workspace
```

### Running

**Start the server:**
```bash
./target/release/gbe-server [socket-path]
# Default socket: /tmp/gbe-server.sock
```

**Start the client:**
```bash
./target/release/gbe-client <session-name> [filename]
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run specific package tests
cargo test -p gbe-client
cargo test -p gbe-server
cargo test -p gbe-common
```

## Architecture (Phase 5 Target)

**Core tools (minimal):**
```
gbe-router       Message broker (dumb forwarding)
gbe-adapter      Universal wrapper (wraps any command)
gbe-buffer       Storage layer (rope/ring buffers)
gbe-client       Render + input (terminal UI)
```

**Multiple interfaces:**
```
Text (CLI)       Traditional bash-like shell
AI (LLM)         Natural language → tool chains
GUI (Web)        Drag-n-drop visual composition
Visual (Flow)    Node-RED style flow programming
Iconic (Icons)   Visual metaphor composition
```

**Project structure:**
```
gbe/
├── client/         # Terminal UI client (ratatui)
├── server/         # Router + adapter + buffer
├── common/         # Protocol definitions
├── docs/           # User documentation
└── notes/          # Vision & design docs
```

## Current Capabilities (Phase 2)

### Phase 1 (Complete)
- ✅ Terminal UI with ratatui
- ✅ Basic text editing
- ✅ File operations
- ✅ Rope-based text buffer

### Phase 2 (Complete)
- ✅ Unix socket IPC
- ✅ Protocol definitions
- ✅ Session management
- ✅ Client-server communication
- ✅ Integration testing

### Phases 3-4 (Deferred)
Deferred in favor of Phase 5 (universal substrate)

### Phase 5 (Next - Core Substrate)
- 📋 gbe-router (message broker)
- 📋 gbe-adapter (universal wrapper)
- 📋 gbe-buffer (rope + ring)
- 📋 Protocol for tool composition
- 📋 Text interface (shell-like)

## Development

### Issue Tracking

This project uses [Beads](https://github.com/cablehead/beads) for issue tracking:

```bash
bd ready              # Show available work
bd show <id>          # View issue details
bd list --status=open # List all open issues
```

### Workflow

See [notes/AGENTS.md](notes/AGENTS.md) for development workflow and conventions.

## License

See workspace Cargo.toml for license information.
