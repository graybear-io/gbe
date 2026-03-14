# Job Pipeline Components

The job execution pipeline: DAG submission → task routing → VM isolation → execution → monitoring.

---

## Nexus — The Backbone

Crates: `nexus`, `nexus-memory`, `nexus-bridge`, `nexus-redis`, `state-store`, `state-store-redis`, `jobs-domain`

Three abstractions:
- **Transport** trait — publish/subscribe over Redis Streams (NATS planned)
- **StateStore** trait — KV with field-level ops, CAS, scan
- **Jobs Domain** — TaskDefinition, JobDefinition, state machines, lifecycle events

Three-stream pattern per task type: `queue` (workers), `progress` (orchestrator), `terminal` (monitors). Envelope is transport-owned (message_id, subject, timestamp, trace_id). Payload is domain-owned, wrapped in `DomainPayload<T>` with schema version and dedup ID.

### Two-Tier Model

Nexus operates in two tiers:

- **Core nexus** (Redis/NATS) — system-wide backbone. Oracle, Watcher, Overseer connect here.
- **Edge nexus** (MemoryTransport or future durable local store) — runs on sentinel hosts or inside VMs. Operatives publish here via vsock relay. No network dependency.

Sentinel is the only component that spans both tiers.

### Nexus Bridge (`nexus-bridge` crate)

Subscribes to subjects on a source transport and republishes to one or more sink transports. Tracks a cursor (last forwarded message ID) per subject for resumability.

```
edge MemoryTransport ──[bridge]──→ core Redis transport
                       └─────────→ envoy streams (display)
```

The bridge uses consumer group "bridge" with `StartPosition::Earliest` so no events are missed. Subject routing is configurable — not all edge subjects need to cross the boundary.

---

## Oracle — The Coordinator

Crate: `crates/oracle`

`SimpleOracle` is a **sync state machine** — two HashSets (`dispatched`, `completed`) and a filter. `ready_tasks()` returns tasks whose dependencies are all completed and that haven't been dispatched yet. Failure is terminal: one task fails, no more dispatch. `OracleDriver` wraps it with bus event emission. 16 tests cover linear, diamond, and wide fan-out DAGs.

---

## Operative — The Executor

Crate: `crates/operative`

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

Crate: `crates/sentinel`

Boots ephemeral Firecracker VMs per task. Communicates with guest operative over vsock (JSON-lines protocol). Three-phase network security evolution: NAT → proxy → zero-trust tool proxy. Slot-based capacity model. Run loop implemented: subscribes to task queues, claims via CAS, emits lifecycle events through edge transport, bridges to core via nexus-bridge. VM provisioning and vsock listener are stubs.

Sentinel is the **nexus bridge**: it owns an edge transport locally, receives operative events over vsock, publishes them to the edge transport, and bridges them to core nexus. The operative never touches the bus directly — its world is sentinel-sized. This enables operation across disparate networks where VMs may have zero network access and sentinels may sit behind firewalls with only port 22 available.

---

## Watcher — The Watchdog

Crates: `crates/watcher`, `crates/watcher-tui`

Two subsystems: **Watcher** (stuck job detection, stream trimming with distributed Redis lock) and **Archiver** (batch consumption from streams, gzip JSONL to cold storage, ack-after-write). TUI monitor for real-time event observation (ratatui). 15+ integration tests.
