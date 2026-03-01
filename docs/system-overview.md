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
| **gbe-cryptum** | Wayland compositor hosting TUI surfaces (via foot) | Display provider |
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
  ── display stack ─────────────────────────────────────────

      ┌──────────┐  "what sources exist?"       │
      │ overseer │◄─────────────────────────────┘
      │ (pick)   │
      └────┬─────┘
           │ "show source X"
           ▼
      ┌──────────┐  frames   ┌─────────┐
      │ cryptum  │◄──────────│  client  │
      │ (surface)│  via foot │ (render) │
      └──────────┘           └─────────┘
```

---

## Layer Boundaries

- **Overseer** is display-agnostic. It never touches pixels — it tells Cryptum "allocate a surface for source X."
- **Cryptum** is gbe-ignorant (today). It hosts foot terminals; gbe-client runs inside them. A future built-in shim could consume gbe frames directly.
- **Envoy** components (router, adapter, buffer, proxy) are the data plane. Client is a terminal sink on that plane.
- **Nexus** is the control plane for job execution. Envoy's router is a separate control plane for tool data streams.
- **Operative → Adapter** is the bridge between stacks. When an operative spawns adapters to run tools, those adapters register as sources on the envoy router — making task output visible to the display stack without the operative knowing or caring who's watching.

---

## See Also

- [ecumene-roles.md](ecumene-roles.md) — trait contracts in detail
- [naming-themes.md](naming-themes.md) — Forerunner/Colony Ship/Defense Grid name mappings
