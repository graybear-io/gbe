# Job Pipeline Components

The job execution pipeline: DAG submission → task routing → VM isolation → execution → monitoring.

---

## Nexus — The Backbone

**Repo**: gbe-nexus (Rust workspace: 6 crates)

Three abstractions:
- **Transport** trait — publish/subscribe over Redis Streams (NATS planned)
- **StateStore** trait — KV with field-level ops, CAS, scan
- **Jobs Domain** — TaskDefinition, JobDefinition, state machines, lifecycle events

Three-stream pattern per task type: `queue` (workers), `progress` (orchestrator), `terminal` (monitors). Envelope is transport-owned (message_id, subject, timestamp, trace_id). Payload is domain-owned, wrapped in `DomainPayload<T>` with schema version and dedup ID.

---

## Oracle — The Coordinator

**Repo**: gbe-oracle (Rust, single crate)

`SimpleOracle` is a **sync state machine** — two HashSets (`dispatched`, `completed`) and a filter. `ready_tasks()` returns tasks whose dependencies are all completed and that haven't been dispatched yet. Failure is terminal: one task fails, no more dispatch. `OracleDriver` wraps it with bus event emission. 16 tests cover linear, diamond, and wide fan-out DAGs.

---

## Operative — The Executor

**Repo**: gbe-operative (Rust, single crate)

Trait: `fn handles() -> &[TaskType]` + `async fn execute(task) -> TaskOutcome`. Six implementations:
- **ShellOperative** — `sh -c <command>`, stdout capture, JSON auto-detect
- **HttpOperative** — structured HTTP calls via reqwest
- **LlmOperative** — OpenAI-compatible chat completions (works with local ollama)
- **MoleculeOperative** — runs a sub-DAG as a single task (recursive composition)
- **CompositeOperative** — router: dispatches to child operatives by task type
- **MockOperative** — testing

Driver resolves `input_from` references between tasks (dot-notation into upstream output JSON), enabling typed data flow through the DAG.

---

## Sentinel — The Boundary

**Repo**: gbe-sentinel (Rust workspace)

Boots ephemeral Firecracker VMs per task. Communicates with guest operative over vsock (JSON-lines protocol). Three-phase network security evolution: NAT → proxy → zero-trust tool proxy. Slot-based capacity model. Core structures and contracts defined; run loop and VM management are stubs.

---

## Watcher — The Watchdog

**Repo**: gbe-watcher (Rust workspace: 2 crates)

Two subsystems: **Watcher** (stuck job detection, stream trimming with distributed Redis lock) and **Archiver** (batch consumption from streams, gzip JSONL to cold storage, ack-after-write). TUI monitor for real-time event observation (ratatui). 15+ integration tests.
