# GBE - GBE Builds Everything

**A universal tool composition platform with multiple interfaces.**

GBE is a substrate for composing Unix tools and commands through multiple interfaces: traditional shell (text), AI (natural language), GUI (drag-n-drop), and visual programming (flow diagrams).

> *GBE Builds Everything* - A recursive acronym in the spirit of GNU (GNU's Not Unix)

## Project Vision

Traditional shells (bash/zsh) provide one way to compose tools: text commands. GBE provides multiple interfaces to the same powerful substrate, democratizing tool composition for everyone from developers to non-technical users.

See [Architecture Document](notes/ARCHITECTURE.md) for complete vision and design.

## Documentation

### Vision & Architecture
**Start here to understand GBE:**
- [Architecture](notes/ARCHITECTURE.md) - Complete vision and design
- [Design Evolution](notes/diary/) - Design thinking journal

### Project Information
- [STATUS.md](STATUS.md) - Current project status and roadmap
- [AGENTS.md](AGENTS.md) - Development workflow and conventions
- [docs/](docs/) - User-facing documentation (to be written)

## Getting Started

GBE is currently in the vision and design phase. Implementation will begin with Phase 5 (core substrate).

See [STATUS.md](STATUS.md) for detailed project status and [ARCHITECTURE.md](notes/ARCHITECTURE.md) for the complete design.

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
```

**Project structure:**
```
gbe/
├── notes/          # Vision & architecture docs
├── docs/           # User documentation (TBD)
├── AGENTS.md       # Development workflow
└── STATUS.md       # Current status & roadmap
```

## Project Status

**Current Phase:** Vision & Planning

- ✅ Vision defined
- ✅ Architecture designed
- 🎯 Next: Phase 5 - Build core substrate
  - gbe-router (message broker)
  - gbe-adapter (universal wrapper)
  - gbe-buffer (rope + ring)
  - gbe-client (terminal UI)
  - Protocol for tool composition
  - Text interface (shell-like)

## Development

### Issue Tracking

This project uses [Beads](https://github.com/cablehead/beads) for issue tracking:

```bash
bd ready              # Show available work
bd show <id>          # View issue details
bd list --status=open # List all open issues
```

### Workflow

See [AGENTS.md](AGENTS.md) for development workflow and conventions.

## License

See workspace Cargo.toml for license information.
