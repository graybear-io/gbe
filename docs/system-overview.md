# GBE System Overview

**GBE (Gray Bear Ecumene)** is a distributed system for composing, orchestrating, and executing computational work inside ephemeral, isolated virtual machines. Components communicate exclusively through a shared bus — no component calls another directly.

Naming follows Halo's **Forerunner** civilization theme. The "Ecumene" is the collective.

---

## The Story in One Paragraph

A user defines a **job** as a DAG of tasks. The **Oracle** walks that DAG, publishing ready tasks to the bus. **Operatives** claim and execute those tasks (shell commands, HTTP calls, LLM completions, or nested sub-DAGs). **Sentinel** boots a fresh Firecracker microVM per task, injects the operative, and tears it down when done — every task gets a pristine sandbox. **Nexus** is the message bus and state store binding everything together. **Watcher** detects stuck jobs and retries them; its **Archiver** drains audit streams to cold storage. **Envoy** is a separate composition substrate — a protocol for wiring Unix tools together through adapters. **Cryptum** is the display plane — ttyd serves PTY streams from the VM, and `ttyd-connect` renders them natively on macOS. **Harness** is a Python framework for Anthropic API agentic loops.

---

## Projects

| Project | What it is | Key role |
|---------|-----------|----------|
| **gbe-nexus** | Message bus + KV state store | Transport backbone |
| **gbe-oracle** | DAG walker, emits tasks as deps resolve | Task routing |
| **gbe-operative** | Executes tasks inside VMs | Task execution |
| **gbe-sentinel** | Per-host VM lifecycle | Boundary enforcement |
| **gbe-watcher** | Sweep/archive, anomaly detection | Event monitoring |
| **gbe-envoy** | Tool composition (router, adapter, buffer, proxy) | Data piping |
| **gbe-cryptum** | Display plane: ttyd-connect + future Metarch | Display provider |
| **gbe-ark** | Alpine VM constructor | Runtime environment |
| **gbe-harness** | Python agentic LLM loop | Operative impl (planned) |
| **gbe-overseer** | Source discovery + surface orchestration | Human interface (planned) |

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

## Three Planes

| Plane | Protocol | Transport | Scope |
|-------|----------|-----------|-------|
| **Control** | Envoy JSON (newline-delimited) | Unix sockets | Tool registration, subscriptions |
| **Data** | Envoy binary frames (len+seq+payload) | Unix sockets (P2P) | Tool output streaming |
| **Display** | ttyd (prefix byte + payload) | WebSocket | Remote terminal rendering |

Each plane is independent. ttyd has no awareness of envoy. Envoy has no awareness of ttyd.

---

## Information Topology

Components have deliberately constrained views of the world:

```text
                    ┌─────────────────────────────────┐
                    │        core nexus (Redis)        │
                    │   oracle, watcher, overseer      │
                    └──────┬──────────────┬────────────┘
                           │              │
                    ┌──────▼──────┐ ┌─────▼───────┐
                    │  sentinel A │ │ sentinel B  │
                    │  [bridge]   │ │ [bridge]    │
                    │  edge nexus │ │ edge nexus  │
                    └──┬──────┬───┘ └──┬──────┬───┘
                       │      │        │      │
                   [vsock] [vsock]  [vsock] [vsock]
                       │      │        │      │
                    ┌──▼──┐┌──▼──┐  ┌──▼──┐┌──▼──┐
                    │op-1 ││op-2 │  │op-3 ││op-4 │
                    └─────┘└─────┘  └─────┘└─────┘
```

- **Operative** sees only sentinel (via vsock). No network, no bus access, no awareness of other operatives.
- **Sentinel** sees its operatives (vsock inward) and some path to core nexus (outward). It bridges between the two tiers using `nexus-bridge`.
- **Oracle/Overseer** see only core nexus. They never address operatives or sentinels directly.

This works across disparate connected networks. A sentinel behind a firewall with only port 22 can bridge through an SSH tunnel. A fleet of VMs reachable only through their sentinel's vsock needs nothing more. The operative's world is sentinel-sized — deliberately.

---

## Design Principles

1. **Bus, not calls** — Components never call each other. All communication via Nexus subjects.
2. **Traits are contracts** — Oracle, Operative, Transport, StateStore, Sentinel are all traits. Implementations swap freely.
3. **Envelope/payload separation** — Transport owns metadata. Domains own data.
4. **Ephemeral isolation** — Every task gets a fresh VM. No side effects leak.
5. **Fail-stop DAGs** — One task failure halts the job. Simple, safe, deterministic.
6. **Claim-check for large data** — Bus carries references. Blobs live externally.
7. **Config-driven onboarding** — New task types via config, not code.
8. **Edge-native transport** — Components work across network boundaries. No operative requires direct bus access. Sentinel bridges edge ↔ core.

---

## How a Job Executes

```
1. Job submitted (YAML/JSON DAG of tasks)
2. Oracle validates DAG, emits root tasks → gbe.tasks.{type}.queue
3. Sentinel claims task (CAS on state store), boots Firecracker VM
4. Operative inside VM executes task, publishes events via vsock to sentinel
5. Sentinel bridges events from edge nexus → core nexus (gbe.tasks.{type}.terminal)
6. Oracle hears completion, unblocks dependents, emits next tasks
7. Repeat 3-6 until DAG exhausted
8. Oracle publishes JobCompleted → gbe.jobs.{type}.completed
```

If a task stalls, Watcher detects it via `updated_at` scan, retries up to budget, then fails the job.

---

## Component Details

- [Job Pipeline](components/job-pipeline.md) — Nexus, Oracle, Operative, Sentinel, Watcher
- [Envoy](components/envoy.md) — Tool composition substrate
- [Display Plane](components/display.md) — Cryptum, Ark, Harness, Overseer
- [Implementation Status](components/status.md) — Progress table and known gaps

## See Also

- [ecumene-roles.md](ecumene-roles.md) — trait contracts in detail
- [naming-themes.md](naming-themes.md) — Forerunner/Colony Ship/Defense Grid name mappings
