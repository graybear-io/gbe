# Ecumene Roles Architecture

**Date**: 2026-02-18
**Status**: Draft
**Context**: Defining GBE roles as trait contracts with separate implementations.

---

## Principle

Each GBE role is a **trait** — a contract defining what the role does, not how.
Implementations live in separate crates. The ecumene is the set of contracts;
any conforming implementation can participate.

```text
gbe-{role}/          # trait crate — defines the contract
gbe-{role}-{impl}/   # implementation crate — one way to fulfill it
```

---

## Roles (Forerunner Theme)

Sourced from [naming-themes.md](./naming-themes.md).

| Role | Contract | Description |
|------|----------|-------------|
| **Nexus** | Transport + state storage | Message backbone, KV store |
| **Oracle** | DAG walking + task dispatch | Walks job DAGs, emits tasks to bus, tracks completions |
| **Operative** | Task execution + reporting | Subscribes to task types, executes work, reports outcomes |
| **Envoy** | Tool composition substrate | Router, adapter, buffer, client — composable tool plumbing |
| **Sentinel** | VM lifecycle management | Per-host boundary enforcement |
| **Watcher** | Sweep + archive | Retention enforcement, anomaly detection |
| **Beacon** | Health signals | Periodic heartbeat broadcast |
| **Custodian** | Artifact management | Images, kernels, manifests |
| **Architect** | Provisioning | Stands up hosts, configures infrastructure |
| **Overseer** | Human interface | Observer with intervention authority |

---

## Oracle and Operative: The Job Execution Contract

### Oracle (trait)

The oracle owns job lifecycle. It receives job definitions, walks the DAG,
and emits tasks as their dependencies resolve. It does **not** execute tasks.

The oracle only reads `depends_on` from task definitions. All other fields
(params, timeout, retries) pass through opaquely — they are the operative's
concern.

```rust
#[async_trait]
pub trait Oracle: Send + Sync {
    /// Submit a job definition. Returns a job ID.
    async fn submit(&self, def: JobDefinition) -> Result<JobId, OracleError>;

    /// Drive all active jobs forward. Called in a loop or on event.
    async fn tick(&self) -> Result<(), OracleError>;

    /// Handle a task completion/failure report.
    async fn task_reported(&self, task_id: TaskId, outcome: TaskOutcome)
        -> Result<(), OracleError>;
}
```

**Publishes**: `gbe.jobs.{type}.created`, `gbe.jobs.{type}.completed`, etc.
**Emits tasks**: `gbe.tasks.{task_type}.queue`
**Listens**: `gbe.tasks.{task_type}.terminal`

### Operative (trait)

The operative executes tasks. It subscribes to one or more task types,
claims work from the queue, decides how to run it, and reports outcomes.

The operative is **stateless** — everything it needs comes from the bus
event plus (optionally) a KV fetch using the definition ref. It interprets
`params` according to the task types it handles.

```rust
#[async_trait]
pub trait Operative: Send + Sync {
    /// Task types this operative handles.
    fn handles(&self) -> &[TaskType];

    /// Execute a single task. The operative decides *how*.
    async fn execute(&self, task: &TaskDefinition) -> Result<TaskOutcome, OperativeError>;
}
```

**Listens**: `gbe.tasks.{task_type}.queue` (for types it handles)
**Reports**: `gbe.tasks.{task_type}.progress`, `gbe.tasks.{task_type}.terminal`

### Task Outcome

What the operative reports back:

```rust
pub enum TaskOutcome {
    Completed {
        output: Vec<String>,
        result_ref: Option<String>,
    },
    Failed {
        exit_code: i32,
        error: String,
    },
}
```

---

## Communication

Oracle and operative communicate **only through the bus**. They never call
each other directly.

```text
Oracle                         Bus                        Operative
  │                             │                            │
  ├─ submit(job) ──────────────►│                            │
  │                             │                            │
  ├─ tick() ───► task ready ───►│ gbe.tasks.X.queue ────────►│
  │                             │                            ├─ claim
  │                             │                            ├─ execute
  │                             │ gbe.tasks.X.terminal ◄────┤ report
  │◄── task_reported() ────────│                            │
  ├─ tick() ───► next task ───►│                            │
  │  ...                        │                            │
  ├─ all done ─────────────────►│ gbe.jobs.Y.completed       │
```

This means:

- They can run in-process (same binary, in-memory transport) or
  as separate processes (over nexus transport)
- Multiple operatives can compete on the same task type (scaling)
- One operative can handle multiple task types (generalist)
- The oracle doesn't know or care what operatives exist

---

## TaskDefinition Is the Contract

The `TaskDefinition` (from `gbe-jobs-domain`) is the full contract between
the job author and the operative. No additional assignment or envelope type
is needed.

```yaml
- name: "fetch-data"
  task_type: "data-fetch"
  params:
    command: "curl -s https://api.example.com/usage"
    output_dest: "s3://bucket/fetch-output.csv"
  timeout_secs: 120
```

- **Oracle reads**: `name`, `depends_on` — to walk the DAG
- **Oracle ignores**: `params`, `timeout_secs`, `max_retries` — passes through
- **Operative reads**: everything — interprets `params` for its task types

The `params` field is `HashMap<String, String>`. What keys exist and what
they mean is a convention between the job author and the operative that
handles that `task_type`. The oracle and the domain schema are agnostic.

For large artifacts, use the claim-check pattern: store the artifact
externally, pass a reference in params (`result_ref`, `output_dest`, etc.).
The operative writes there; downstream tasks read from there.

### Bus Message

What goes on the queue is lean — `TaskQueued` carries IDs and a reference:

- `task_id`, `job_id`, `org_id`, `task_type`, `params`, `retry_count`

Or the oracle publishes just IDs + a `definition_ref` (KV key), and the
operative fetches the full definition. Either way, the operative is
stateless — it gets everything from the event.

---

## Operative Specialization

| Implementation | Handles | How it executes |
|---|---|---|
| **ShellOperative** | Configurable task types | Reads `command` from params, spawns adapter + shell |
| **PythonOperative** | Python task types | Maps task_type to `python -m ...` convention |
| **LambdaOperative** | (future) | Invokes cloud functions |
| **ContainerOperative** | (future) | Runs in container |

The `ShellOperative` is the first implementation. It reads `command` from
params, wraps it via envoy's adapter, and captures output.

---

## Current Code Mapping

### What exists and where it maps

| Current code | Role | Notes |
|---|---|---|
| `gbe-operative/orchestrator.rs` | Oracle + Operative (conflated) | Needs splitting |
| `gbe-operative/executor.rs` TaskExecutor | Proto-Operative | Close to Operative trait |
| `gbe-operative/executor.rs` EnvoyExecutor | PythonOperative (draft) | Hardcodes Python convention |
| `gbe-operative/state.rs` StateManager | Oracle concern | Job/task state tracking |
| `gbe-nexus/` Transport + StateStore | Nexus (trait-based) | Aligned |
| `gbe-nexus/jobs-domain/` | Shared domain types | Used by both oracle and operative |
| `gbe-envoy/` router, adapter, etc. | Envoy | Aligned — adapter is the dumb pipe |
| `gbe-watcher/` | Watcher | Aligned |

### What needs to change

1. **Create `gbe-oracle`** — extract DAG walking + task dispatch from orchestrator
2. **~~Rename `gbe-runner` → `gbe-operative`~~** — done
3. **Split orchestrator** — oracle logic (DAG, state) vs operative logic (execution)
4. **Remove hardcoded Python convention** — operative reads `command` from params
5. **Decouple `TaskContext`** — remove `router_socket` from shared contract
6. **Update `runner-design.md`** — superseded by this document

---

## Crate Layout

```text
gbe/
├── Cargo.toml                   # workspace root (all crates)
├── docs/
├── poc/                         # frozen POC (separate workspace)
└── crates/
    ├── nexus/                   # Transport trait + envelope
    ├── nexus-memory/            # in-memory transport (edge/testing)
    ├── nexus-bridge/            # edge → core forwarding
    ├── nexus-redis/             # Redis Streams transport
    ├── state-store/             # StateStore trait
    ├── state-store-redis/       # Redis state store
    ├── jobs-domain/             # shared job/task schemas
    ├── oracle/                  # DAG walker, task dispatch
    ├── operative/               # task execution (shell, HTTP, LLM, molecule)
    ├── sentinel/                # VM lifecycle, boundary enforcement
    ├── watcher/                 # stuck job detection, archiver
    ├── watcher-tui/             # TUI monitor
    ├── protocol/                # envoy wire protocol
    ├── router/                  # envoy tool router
    ├── adapter/                 # envoy stdio adapter
    ├── buffer/                  # ring/rope buffer
    ├── client/                  # envoy stream TUI
    ├── proxy/                   # envoy proxy
    └── cryptum/                 # ttyd-connect native client
```

---

## Open Questions

- **Beacon**: Is this a trait or a pattern any role can implement?
- **Overseer**: Human CLI/UI — trait or concrete tool?
