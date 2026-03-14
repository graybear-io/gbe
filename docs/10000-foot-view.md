# GBE: The 10,000-Foot View

**GBE (Gray Bear Ecumene)** is a distributed system for composing, orchestrating, and executing computational work inside ephemeral, isolated virtual machines. It provides a message-driven architecture where independent components communicate exclusively through a shared bus — no component calls another directly.

The naming follows Halo's **Forerunner** civilization theme. The "Ecumene" is the collective.

---

## The Story in One Paragraph

A user defines a **job** as a DAG of tasks. The **Oracle** walks that DAG, publishing ready tasks to the bus. **Operatives** claim and execute those tasks (shell commands, HTTP calls, LLM completions, or nested sub-DAGs). **Sentinel** boots a fresh Firecracker microVM per task, injects the operative, and tears it down when done — every task gets a pristine sandbox. **Nexus** is the message bus and state store binding everything together. **Watcher** detects stuck jobs and retries them; its **Archiver** drains audit streams to cold storage. **Envoy** is a separate composition substrate — a protocol for wiring Unix tools together through adapters, with a TUI client for interactive use. **Harness** is a Python learning framework for Anthropic API agentic loops. **Ark** builds the Alpine VM environment where Rust projects compile. **Cryptum** is the display plane — ttyd serves PTY streams from the VM, and `ttyd-connect` renders them natively on macOS.

---

## Component Map

```
                         ┌─────────────────────────────┐
                         │      NEXUS (bus + state)     │
                         │  Redis Streams / KV store    │
                         └──────┬──────┬──────┬────────┘
                                │      │      │
                 ┌──────────────┤      │      ├──────────────┐
                 │              │      │      │              │
            ┌────▼────┐   ┌────▼────┐ │ ┌────▼────┐   ┌────▼────┐
            │  ORACLE  │   │OPERATIVE│ │ │ WATCHER │   │SENTINEL │
            │ DAG walk │   │ execute │ │ │sweep/arc│   │ VM life │
            └─────────┘   └─────────┘ │ └─────────┘   └─────────┘
                                      │
                              ┌───────▼───────┐
                              │     ENVOY     │
                              │ tool compose  │
                              └───────────────┘

   Separate concerns:
   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
   │ HARNESS  │  │   ARK    │  │ CRYPTUM  │  │ OVERSEER │
   │ LLM loop │  │ VM build │  │ Wayland  │  │ (planned)│
   └──────────┘  └──────────┘  └──────────┘  └──────────┘
```

---

## How a Job Executes

```
1. Job submitted (YAML/JSON DAG of tasks)
2. Oracle validates DAG, emits root tasks → gbe.tasks.{type}.queue
3. Sentinel claims task (CAS on state store), boots Firecracker VM
4. Operative inside VM executes task, returns outcome via vsock
5. Sentinel publishes result → gbe.tasks.{type}.terminal
6. Oracle hears completion, unblocks dependents, emits next tasks
7. Repeat 3-6 until DAG exhausted
8. Oracle publishes JobCompleted → gbe.jobs.{type}.completed
```

If a task stalls, Watcher detects it via `updated_at` scan, retries up to budget, then fails the job.

---

## Components in Detail

### Nexus — The Backbone
**Repo**: gbe-nexus (Rust workspace: 6 crates)

Three abstractions:
- **Transport** trait — publish/subscribe over Redis Streams (NATS planned)
- **StateStore** trait — KV with field-level ops, CAS, scan
- **Jobs Domain** — TaskDefinition, JobDefinition, state machines, lifecycle events

Three-stream pattern per task type: `queue` (workers), `progress` (orchestrator), `terminal` (monitors). Envelope is transport-owned (message_id, subject, timestamp, trace_id). Payload is domain-owned, wrapped in `DomainPayload<T>` with schema version and dedup ID.

### Oracle — The Coordinator
**Repo**: gbe-oracle (Rust, single crate)

`SimpleOracle` is a **sync state machine** — two HashSets (`dispatched`, `completed`) and a filter. `ready_tasks()` returns tasks whose dependencies are all completed and that haven't been dispatched yet. Failure is terminal: one task fails, no more dispatch. `OracleDriver` wraps it with bus event emission. 16 tests cover linear, diamond, and wide fan-out DAGs.

### Operative — The Executor
**Repo**: gbe-operative (Rust, single crate)

Trait: `fn handles() -> &[TaskType]` + `async fn execute(task) -> TaskOutcome`. Six implementations:
- **ShellOperative** — `sh -c <command>`, stdout capture, JSON auto-detect
- **HttpOperative** — structured HTTP calls via reqwest
- **LlmOperative** — OpenAI-compatible chat completions (works with local ollama)
- **MoleculeOperative** — runs a sub-DAG as a single task (recursive composition)
- **CompositeOperative** — router: dispatches to child operatives by task type
- **MockOperative** — testing

Driver resolves `input_from` references between tasks (dot-notation into upstream output JSON), enabling typed data flow through the DAG.

### Sentinel — The Boundary
**Repo**: gbe-sentinel (Rust workspace)

Boots ephemeral Firecracker VMs per task. Communicates with guest operative over vsock (JSON-lines protocol). Three-phase network security evolution: NAT → proxy → zero-trust tool proxy. Slot-based capacity model. Core structures and contracts defined; run loop and VM management are stubs.

### Watcher — The Watchdog
**Repo**: gbe-watcher (Rust workspace: 2 crates)

Two subsystems: **Watcher** (stuck job detection, stream trimming with distributed Redis lock) and **Archiver** (batch consumption from streams, gzip JSONL to cold storage, ack-after-write). TUI monitor for real-time event observation (ratatui). 15+ integration tests.

### Envoy — The Substrate
**Repo**: gbe-envoy (Rust workspace: 5 crates)

Separate from the job execution pipeline. A **tool composition platform**:
- **Router** — dumb control-plane message broker, assigns ToolIDs, manages subscriptions
- **Adapter** — wraps any Unix command, bridges stdin/stdout to protocol
- **Buffer** — rope (seekable) and ring (fixed-capacity) storage
- **Client** — ratatui TUI renderer
- **Proxy** — tee for multiple subscribers

Dual-channel protocol: JSON control (via router) + binary data (direct P2P). Vision: multiple interfaces (text, AI, GUI, visual flow) on the same substrate.

### Harness — The Learning Loop
**Repo**: gbe-harness (Python)

Educational agentic loop framework. Calls Anthropic API with tool definitions, parses tool_use blocks, executes tools, feeds results back. Uses `anthropic` SDK directly, no abstractions. Built with uv + just.

### Ark — The Foundation
**Repo**: gbe-ark (Shell)

Alpine Linux VM provisioning. `constructor.sh` installs build deps, Wayland libraries, Rust toolchain. Provides the environment where Cryptum and other Rust projects build natively. Phases 0-3 complete.

### Cryptum — The Display Plane
**Repo**: gbe-cryptum (Rust)

Display transport for remote terminal access. After six failed Wayland/VNC/RDP attempts on headless Alpine (no GPU), pivoted to ttyd (PTY over WebSocket) + native Rust client. `ttyd-connect` connects to a ttyd instance on the VM and renders in a native macOS terminal — no browser, no compositor. Future: Metarch, a multi-stream terminal compositor that arranges multiple ttyd streams as panes. Smithay POC (phases 0-4) archived.

### Overseer — The Interface
**Status**: Planned, not yet created.

Human command interface. Source discovery + surface orchestration.

---

## Design Principles

1. **Bus, not calls** — Components never call each other. All communication via Nexus subjects.
2. **Traits are contracts** — Oracle, Operative, Transport, StateStore, Sentinel are all traits. Implementations swap freely.
3. **Envelope/payload separation** — Transport owns metadata. Domains own data.
4. **Ephemeral isolation** — Every task gets a fresh VM. No side effects leak.
5. **Fail-stop DAGs** — One task failure halts the job. Simple, safe, deterministic.
6. **Claim-check for large data** — Bus carries references. Blobs live externally.
7. **Config-driven onboarding** — New task types via config, not code.

---

## Implementation Status

| Component | Core Contracts | Implementation | Tests |
|-----------|---------------|----------------|-------|
| Nexus | Complete | Redis backends working | Yes |
| Oracle | Complete | SimpleOracle + Driver | 16 tests |
| Operative | Complete | Shell, HTTP, LLM, Molecule | Yes |
| Sentinel | Complete | Stubs (SlotTracker, config, protocol done) | Partial |
| Watcher | Complete | Sweep + archiver working | 15+ tests |
| Envoy | Complete | Router, adapter, buffer, client, proxy | Yes |
| Harness | Complete | Agent loop working | Yes |
| Ark | Complete | Provisioning done | N/A |
| Cryptum | Complete | ttyd-connect working, Metarch planned | Manual |
| Overseer | Not started | — | — |

---

## Intent-vs-Code Review

### Aligned
- **Nexus** docs describe three-stream pattern, envelope/payload separation, CAS claims — code implements all of this faithfully
- **Oracle** docs describe sync state machine with fail-stop semantics — `SimpleOracle` matches exactly
- **Operative** docs describe pluggable executors with input resolution — `CompositeOperative` + `driver.rs` deliver this
- **Watcher** docs describe sweep/archive with distributed lock — implementation matches with comprehensive tests
- **Envoy** architecture describes adapter-centric, dual-channel protocol — code reflects this design

### Gaps and Observations

**Rust Core (Nexus, Oracle, Operative, Watcher, Envoy):**
- **Sentinel**: Architecture doc is detailed and mature, but most runtime code is stubs. The contracts (config, protocol types, slot tracker, claim logic) are solid. The gap is in `Sentinel::run()`, VM spawning, and vsock listener — the "last mile" to a working system.
- **Oracle ↔ Sentinel integration**: Docs describe oracle emitting to bus and sentinel claiming from bus. Currently, `gbe-operative/src/driver.rs` runs the oracle+operative loop **in-process** (no bus, no sentinel). This is the dev-mode path. The bus-mediated path (oracle publishes, sentinel claims, operative runs in VM) is designed but not yet wired end-to-end.
- **Envoy's relationship to the job pipeline** is architecturally separate. Docs position it as "Phase 5 substrate" for tool composition, but its protocol and data model don't intersect with Nexus subjects or job domain types. These are two parallel systems that may converge at the Overseer layer.
- **NATS JetStream** is described as a planned transport alternative throughout docs. No NATS implementation crate exists yet.
- **Overseer** is referenced in naming docs and system overview but has no code or detailed design.
- **Archiver → S3**: The `ArchiveWriter` trait exists with a filesystem implementation. S3 writer is documented but not implemented.

**Ark (Alpine VM):**
- Docs and code are fully aligned. All 4 roadmap phases complete. `constructor.sh` (46 lines) does exactly what docs claim.
- Ark provisions a **development** VM (UTM on macOS). It is **not** the production VM path — Sentinel uses Firecracker with ephemeral rootfs. The two could converge (constructor output → Sentinel base image), but today they're independent.
- No automation beyond the single script. Future: `utmctl` scripting, multiple VM profiles.

**Cryptum (Display Plane):**
- Smithay POC phases 0–4 archived. Six VNC/RDP attempts failed on headless Alpine VM (no GPU). See `gbe-cryptum/docs/vnc-migration-notes.md`.
- **Pivoted to ttyd + native client (2026-03-14)**. `ttyd-connect` (~150 lines Rust) connects to ttyd on the VM via WebSocket, renders in native macOS terminal. Interactive shell, resize propagation, clean exit.
- **Three-plane architecture**: control (envoy JSON), data (envoy binary frames), display (ttyd WebSocket). ttyd is outside envoy — dumb PTY transport.
- **Next**: wire `ttyd` to wrap `gbe-client` (envoy TUI renderer), enabling remote display of envoy tool streams.
- **Future**: Metarch — multi-stream terminal compositor consuming N ttyd streams as panes.

**Harness (Python agentic loop):**
- Docs and code are well-aligned. Agent loop, tool registry, and stop conditions all match documentation.
- Clean, well-tested codebase: 80 tests across 3 files covering agent loop, tools, and path handling.
- **`paths.py` is partially orphaned** — provides XDG config + API key loading, but `cli.py` doesn't use it (relies on Anthropic SDK's implicit env var handling). The harness constructor calls `get_api_key()` but CLI never exercises that path.
- **Relation to LlmOperative**: Harness is a full agent loop (multi-turn, tool-calling). LlmOperative is a single prompt→response executor (no tools, no iteration). Harness is the planned Python implementation of the operative trait — it would wrap the agent loop inside `Operative.execute()`.
- No streaming, no parallel tool execution, no session persistence. These are documented limitations, not bugs.
