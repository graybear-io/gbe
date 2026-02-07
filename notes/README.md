# GBE Design & Vision Documents

Design documents and architectural notes for GBE (GBE Builds Everything) - a universal tool composition platform with multiple interfaces.

## Vision & Architecture

**Start here to understand GBE:**

- **[VISION_FULL_MULTIPLE_INTERFACES.md](VISION_FULL_MULTIPLE_INTERFACES.md)** - Complete platform vision
  - Multiple interfaces: Text (CLI), AI (LLM), GUI, Visual Programming, Iconic
  - Universal tool composition substrate
  - Democratizing automation for all users

- **[ARCHITECTURE_V2_ADAPTER_CENTRIC.md](ARCHITECTURE_V2_ADAPTER_CENTRIC.md)** - Current design ⭐
  - 4 core tools: router, adapter, buffer, client
  - Adapter wraps ANY Unix command
  - No coordinators, pure message passing

- **[LINE_STREAM_VISION.md](LINE_STREAM_VISION.md)** - Overview & evolution
  - Introduction to line-oriented streams
  - Links to all architectural versions

- **[ARCHITECTURE_V1_LAYERED.md](ARCHITECTURE_V1_LAYERED.md)** - Archived
  - Original 8-layer design (superseded by V2)
  - Kept for historical context

## Design Evolution

**[diary/](diary/)** - Design thinking journal
- `2026-02-07_vision_breakthrough.md` - How we arrived at multiple interfaces vision
- Session notes and architectural discoveries

## Quick Reference

**Core Concept:**
```
Traditional Shell:  Text commands → Tool execution → Text output
GBE:               Multiple interfaces → Tool chains → Rich rendering
```

**The Five Interfaces:**
1. Text (CLI) - bash-like shell
2. AI (LLM) - natural language
3. GUI (Web) - drag-n-drop
4. Visual Programming - Node-RED style flows
5. Iconic - visual metaphor composition

**All interfaces decompose to the same tool composition protocol.**

## Project Status

- ✅ Vision defined
- ✅ Architecture V2 designed
- 🎯 Next: Phase 5 - Build core substrate
  - gbe-router (message broker)
  - gbe-adapter (universal wrapper)
  - gbe-buffer (storage layer)
  - gbe-client (terminal UI)
