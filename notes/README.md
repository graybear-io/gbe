# gbe design documents

design and architectural notes for gbe (gbe builds everything).

---

## architecture

**[ARCHITECTURE.md](ARCHITECTURE.md)** - complete vision and design

covers:
- vision: multiple interfaces for tool composition
- core design: adapter pattern, minimal tool set, protocol
- interfaces: text, ai, gui, visual, iconic
- implementation roadmap
- success criteria

---

## design evolution

**[diary/](diary/)** - design thinking journal

- `2026-02-07_vision_breakthrough.md` - how we arrived at multiple interfaces vision
- session notes and architectural discoveries

---

## quick reference

core concept:
```
traditional shell: text commands → tool execution → text output
gbe:              multiple interfaces → tool chains → rich rendering
```

five interfaces:
1. text (cli) - bash-like shell
2. ai (llm) - natural language
3. gui (web) - drag-n-drop
4. visual programming - node-red style flows
5. iconic - visual metaphor composition

all interfaces decompose to the same tool composition protocol.

---

## project status

- vision defined
- architecture designed
- next: phase 5 - build core substrate
  - gbe-router (message broker)
  - gbe-adapter (universal wrapper)
  - gbe-buffer (storage layer)
  - gbe-client (terminal ui)
