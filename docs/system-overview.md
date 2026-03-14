# GBE System Overview

**Date**: 2026-03-01
**Purpose**: Quick-reference map of all projects and how they connect.

---

## Projects

| Project | What it is | Key trait/role |
|---------|-----------|----------------|
| **gbe-nexus** | Message bus + KV state store | Transport backbone |
| **gbe-oracle** | DAG walker, emits tasks as deps resolve | Task routing |
| **gbe-operative** | Executes tasks inside VMs, reports outcomes | Task execution |
| **gbe-harness** | Python agentic LLM loop (Anthropic API + tools) | Operative impl (planned) |
| **gbe-sentinel** | Per-host VM lifecycle (create, destroy, fence) | Boundary enforcement |
| **gbe-watcher** | Sweep/archive, retention, anomaly detection | Event monitoring |
| **gbe-envoy** | Composable tool plumbing (router, adapter, buffer, proxy) | Data piping |
| **gbe-client** | TUI renderer, subscribes to a single envoy source | Display sink |
| **gbe-overseer** | Source discovery + surface orchestration | Human command interface |
| **gbe-cryptum** | Display plane: ttyd-connect (native client) + future Metarch compositor | Display provider |
| **gbe-ark** | Alpine VM constructor (shell scripts + ISO tooling) | Test bed for Cryptum |

---

## How They Connect

```text
  ── job execution ─────────────────────────────────────────

                         ┌──────────────────────────────────┐
                         │           gbe-nexus               │
                         │       (bus + state store)          │
                         └──┬──────────┬──────────┬──────────┘
                            │          │          │
                     tasks  │   events │   state  │
                            ▼          ▼          ▼
                      ┌──────────┐ ┌─────────┐ ┌──────────┐
                      │  oracle  │ │ watcher │ │ sentinel │
                      │ (route)  │ │ (sweep) │ │ (VMs)    │
                      └────┬─────┘ └─────────┘ └──────────┘
                           │
                    assign │
                           ▼
                      ┌──────────┐
                      │operative │  e.g. gbe-harness
                      │ (execute)│
                      └────┬─────┘
                           │
                    spawns  │  each task wraps a command
                           ▼
  ── data plane (envoy) ────────────────────────────────────

                      ┌──────────┐
                      │ adapter  │  stdout → protocol frames
                      │ (wrap)   │
                      └────┬─────┘
                           │ registers with
                           ▼
                      ┌──────────────┐
                      │ envoy router │  tracks sources,
                      │ (control)    │  wires subscriptions
                      └──────┬───────┘
                             │ subscribe
                             ▼
  ── display plane (ttyd) ─────────────────────────────────

      ┌──────────┐  "what sources exist?"       │
      │ overseer │◄─────────────────────────────┘
      │ (pick)   │
      └────┬─────┘
           │ "show source X"
           ▼
      ┌──────────┐  PTY/WS   ┌──────────────┐
      │  client  │──────────►│     ttyd      │
      │ (render) │  via ttyd │ (VM, :7681+)  │
      └──────────┘           └──────┬────────┘
                                    │ WebSocket
                              ┌─────▼────────┐
                              │ ttyd-connect  │  macOS
                              │ (or Metarch)  │  native terminal
                              └──────────────┘
```

---

## Layer Boundaries

- **Three planes**: control (envoy JSON, Unix sockets), data (envoy binary frames, Unix sockets), display (ttyd PTY bytes, WebSocket). Each is independent.
- **Overseer** is display-agnostic. It discovers sources and tells the display plane what to show.
- **Cryptum** provides the display plane. Today: `ttyd-connect` (single-stream native client). Future: Metarch (multi-stream terminal compositor). Runs on the client side (macOS), not on the VM.
- **ttyd** runs on the VM, wrapping the envoy client's PTY. It's a dumb transport — no awareness of envoy protocol.
- **Envoy** components (router, adapter, buffer, proxy) are the data plane. Client renders to a terminal; ttyd ships that terminal output remotely.
- **Nexus** is the control plane for job execution. Envoy's router is a separate control plane for tool data streams.
- **Operative → Adapter** is the bridge between stacks. When an operative spawns adapters to run tools, those adapters register as sources on the envoy router — making task output visible to the display plane without the operative knowing or caring who's watching.

---

## See Also

- [ecumene-roles.md](ecumene-roles.md) — trait contracts in detail
- [naming-themes.md](naming-themes.md) — Forerunner/Colony Ship/Defense Grid name mappings
