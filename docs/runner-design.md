# gbe-operative Design

**Date**: 2026-02-16
**Status**: Superseded by [ecumene-roles.md](./ecumene-roles.md)
**Context**: First production use of GBE — orchestrate DAG-based jobs over existing Python tooling.

> **Note**: This document describes the original single-binary runner design.
> The architecture has evolved to separate the **Oracle** (DAG walking) and
> **Operative** (task execution) roles. See [ecumene-roles.md](./ecumene-roles.md)
> for the current design. `gbe-runner` has been renamed to `gbe-operative`.

---

## Overview

`gbe-operative` is a CLI binary that orchestrates jobs. A job is a DAG of tasks
defined in YAML. Each task wraps an existing Python CLI tool via a gbe-envoy
adapter. State is tracked in gbe-nexus's transport and KV store.

Triggered by cron:

```
gbe-operative --org org_acme --date 2026-02-16 --job jobs/daily-report.yaml
```

The runner is **not part of GBE** — it is a consumer of GBE's infrastructure.

---

## Architecture

```text
cron ──► gbe-operative
           │
           ├── starts gbe-router (per-job, temporary)
           ├── loads YAML → JobDefinition → validates DAG
           ├── creates job + task records in StateStore
           │
           ├── for each ready task (parallel via JoinSet):
           │     ├── spawns gbe-adapter -- python -m tool <args>
           │     ├── connects to router, discovers adapter ToolId
           │     ├── subscribes to data stream, reads output
           │     ├── waits for adapter exit → gets exit code
           │     └── updates task state, publishes events
           │
           ├── after each completion: unblock dependents, dispatch ready
           └── all done → update job state → exit
```

### Key Design Decisions

1. **Per-job router** — runner starts/stops its own gbe-router. No shared
   daemon. Simple lifecycle, no coordination.

2. **Envoy adapters for execution** — every task runs as a gbe-adapter
   subprocess wrapping a Python command. Output streams via the envoy protocol.
   This gives us structured output capture and future composability.

3. **Trait-based executor** — `TaskExecutor` trait allows swapping
   `EnvoyExecutor` for `MockExecutor` in tests without touching orchestration.

4. **In-memory backends for testing** — `nexus-memory` transport and
   in-memory state store for unit/integration tests. Redis for production.

---

## Repo Structure

New sibling repo: `gbe/gbe-operative/` (separate git). Single crate.

```text
gbe-operative/
  Cargo.toml
  src/
    main.rs            # CLI entry, arg parsing, wiring
    config.rs          # RunnerConfig (paths, timeouts)
    error.rs           # RunnerError enum
    router_client.rs   # Async gbe-protocol client over Unix socket
    executor.rs        # TaskExecutor trait + EnvoyExecutor
    orchestrator.rs    # DAG loop with tokio JoinSet
    state.rs           # StateStore + Transport integration
  tests/
    orchestrator_test.rs
  fixtures/
    daily-report.yaml
```

### Dependencies

| Crate | Source | Purpose |
|-------|--------|---------|
| `gbe-jobs-domain` | `../gbe-nexus/crates/jobs-domain` | Job/task schemas, state machines, keys |
| `gbe-nexus` | `../gbe-nexus/crates/nexus` | Transport trait |
| `gbe-nexus-memory` | `../gbe-nexus/crates/nexus-memory` | In-memory transport for tests |
| `gbe-state-store` | `../gbe-nexus/crates/state-store` | StateStore trait |
| `gbe-protocol` | `../gbe-envoy/protocol` | ControlMessage, DataFrame, ToolInfo |
| `tokio` | crates.io | Async runtime |
| `clap` | crates.io | CLI arg parsing |
| `serde_yaml` | crates.io | YAML job definition loading |

---

## CLI Interface

```text
gbe-operative --org <ORG_ID> --date <DATE> --job <PATH>
           [--backend memory|redis]
           [--router-bin <PATH>]
           [--adapter-bin <PATH>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--org` | required | Organization ID (e.g., `org_acme`) |
| `--date` | required | Target date (e.g., `2026-02-16`) |
| `--job` | required | Path to YAML job definition |
| `--backend` | `memory` | Transport/state backend |
| `--router-bin` | `gbe-router` | Path to router binary |
| `--adapter-bin` | `gbe-adapter` | Path to adapter binary |

Exit codes: 0 = all tasks succeeded, 1 = task failure, 2 = runner error.

---

## Job Definition Format

YAML files consumed by the runner. Schema defined in `gbe-jobs-domain::definition`.

```yaml
v: 1
name: "Daily Usage Report"
job_type: "daily-usage-report"
tasks:
  - name: "fetch-data"
    task_type: "data-fetch"
    params:
      source: "billing-api"
      format: "csv"
    timeout_secs: 120

  - name: "transform"
    task_type: "data-transform"
    depends_on: ["fetch-data"]
    params:
      template: "usage-summary"

  - name: "send-report"
    task_type: "email-send"
    depends_on: ["transform"]
    params:
      template: "daily-usage"
      recipient_list_ref: "s3://config/recipients.json"
```

Tasks with no `depends_on` are DAG roots and run immediately. Tasks with
dependencies wait until all named dependencies complete.

---

## Execution Flow

### Task Execution (one task via envoy)

The `EnvoyExecutor` runs a single task:

1. **Snapshot tools** — `router_client.query_tools()` → save existing ToolIds
2. **Build command** — map `task_type` + `params` to CLI args:
   `python -m gbe_tools.{task_type} --param1 val1 --date {date} --org {org}`
3. **Spawn adapter** — `gbe-adapter --router <sock> -- <command>`
4. **Discover ToolId** — poll `query_tools()` (100ms interval, 5s timeout)
   until a new ToolId appears
5. **Subscribe** — send `Subscribe { target }` to router, receive
   `SubscribeAck { data_connect_address }`
6. **Read output** — connect to data socket, read DataFrames until EOF
7. **Get exit code** — `child.wait()` on adapter subprocess
8. **Return result** — `TaskResult { exit_code, output }`

### DAG Orchestration

The `Orchestrator` manages the full job:

```text
completed = {}
dispatched = {}

1. Seed JoinSet with root tasks (no dependencies)
2. Loop:
   a. Await next task completion from JoinSet
   b. If success:
      - Add to completed set
      - Scan all tasks: if all deps in completed and not dispatched → dispatch
   c. If failure:
      - Fail-fast: cancel remaining, mark job failed
3. When JoinSet empty → job complete
```

Independent tasks run concurrently via `tokio::task::JoinSet`.

### State Tracking

Uses `gbe-jobs-domain::keys` for KV key patterns and `gbe-jobs-domain::payloads`
for stream events.

| Event | KV Update | Stream Event |
|-------|-----------|--------------|
| Job start | Write job record (`state: running`) | `JobCreated` → `gbe.jobs.{type}.created` |
| Task dispatch | CAS task `blocked→pending` | `TaskQueued` → `gbe.tasks.{type}.queue` |
| Task progress | Update `current_step` | `TaskProgress` → `gbe.tasks.{type}.progress` |
| Task complete | CAS task `running→completed`, increment `completed_count` | `TaskCompleted` → `gbe.tasks.{type}.terminal` |
| Task fail | CAS task `running→failed` | `TaskFailed` → `gbe.tasks.{type}.terminal` |
| Job complete | CAS job `running→completed` | `JobCompleted` → `gbe.jobs.{type}.completed` |
| Job fail | CAS job `running→failed` | `JobFailed` → `gbe.jobs.{type}.failed` |

---

## Modules

### `executor.rs`

```rust
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    async fn execute(
        &self,
        task: &TaskDefinition,
        ctx: &TaskContext,
    ) -> Result<TaskResult, RunnerError>;
}

pub struct TaskResult {
    pub exit_code: i32,
    pub output: Vec<String>,
}

pub struct TaskContext {
    pub org_id: OrgId,
    pub date: String,
    pub job_id: JobId,
    pub task_id: TaskId,
}
```

`EnvoyExecutor` is the production implementation. `MockExecutor` returns
configurable results for tests.

### `router_client.rs`

Async version of `gbe-envoy/adapter/src/router_connection.rs`. Same
newline-delimited JSON protocol over `tokio::net::UnixStream`.

### `orchestrator.rs`

Owns the DAG loop. Takes `Arc<dyn TaskExecutor>` and `Arc<dyn StateManager>`.
Pure orchestration logic, no I/O details.

### `state.rs`

Wraps `StateStore` + `Transport` behind a `StateManager` that exposes
domain-level operations: `create_job()`, `dispatch_task()`,
`complete_task()`, `fail_task()`, `complete_job()`, `fail_job()`.

---

## Prerequisite: gbe-envoy Adapter Exit Code

**File**: `gbe-envoy/adapter/src/main.rs:123`

The adapter currently always exits 0. It must propagate the wrapped command's
exit code so the runner can detect task failures:

```rust
// Replace Ok(()) at end of main() with:
std::process::exit(exit_status.code().unwrap_or(1));
```

---

## Testing Strategy

| Level | What | Backend | Envoy? |
|-------|------|---------|--------|
| Unit | Orchestrator DAG ordering, fail-fast | Mock executor | No |
| Unit | State transitions, KV writes | In-memory store | No |
| Integration | Full job from YAML | Mock executor + memory backends | No |
| E2E | Real job with echo commands | Memory backends + real router/adapter | Yes |

E2E tests marked `#[ignore]` — require built envoy binaries.

---

## Open Questions

- **Command mapping**: How exactly does `task_type` + `params` map to a Python
  CLI invocation? Hardcoded mapping? Convention-based (`python -m gbe_tools.{task_type}`)?
  Config-driven? → Start with convention, add config if needed.

- **Large output**: Should the runner store task output somewhere (claim-check
  to S3) or just log it? → Log for MVP, claim-check later.

- **Retry**: Runner handles retries itself (re-dispatch failed tasks) or
  relies on watcher? → Watcher handles stuck tasks. Runner does immediate
  retry for transient failures (configurable `max_retries` per task).
