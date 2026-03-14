# GBE POC: Distributed Task Lifecycle

Proves GBE components working together across macOS ↔ ark VM boundary.

## What It Does

A periodic system monitoring task runs on ark (Alpine VM). Sentinel
manages the task from macOS, publishing lifecycle events. All streams
are discoverable and monitorable in real time via envoy TUI clients.
Stop signal propagates cleanly through the system.

## Architecture

```
macOS                              ark VM (192.168.64.2)
─────                              ─────────────────────
router (envoy)                     poc-operative (Rust)
poc-sentinel (Rust)                  runs every 30s
  ├─ starts adapter ── ssh ──────►   emits system stats
  ├─ lifecycle stream (envoy)        exits on SIGTERM
  ├─ progress stream (envoy)
  └─ monitors operative
gbe-client (TUI)
  subscribes to any stream
```

## Three Streams

| Stream | Type | Source | Content |
|--------|------|--------|---------|
| sentinel-lifecycle | sentinel events | sentinel → envoy | host-level: started, claimed, task_ended |
| task-progress | task events | sentinel → envoy | heartbeats, stop_received, operative_exited |
| task-output | operative stdout | adapter → envoy | system stats from ark |

All events also persist to `/tmp/nexus/` files (nexus stand-in).

## Running

```sh
# Terminal 1: start everything
cd ~/projects/gbe && ./poc/run-poc.sh

# Terminal 2+: connect to streams (use tool IDs from discovery output)
./gbe-envoy/target/debug/gbe-client --router /tmp/gbe-router.sock --target <TOOL_ID>

# Stop
./poc/stop-poc.sh
```

## GBE Role Mapping

| POC Component | GBE Role | What It Would Be |
|---------------|----------|-------------------|
| poc-sentinel | Sentinel | Per-host VM lifecycle manager |
| poc-operative | Operative | ShellOperative::execute() inside Firecracker VM |
| run-poc.sh | Oracle + Overseer | DAG dispatch + stream discovery |
| stop-poc.sh | Oracle cancel | CancelTask via nexus |
| SSH boundary | vsock | Sentinel ↔ VM communication |
| /tmp/nexus/ files | Nexus (Redis Streams) | Event bus + state store |
| envoy router | Envoy | Tool composition substrate |
| gbe-client TUI | Overseer display | Surface for stream monitoring |

## Known Issues

1. **Lifecycle stream: no replay** — events before subscriber connects
   are lost. Real nexus (Redis Streams) supports replay via XREAD from
   past ID. Fix: nexus-as-files with FIFOs.

2. **EnvoyStream lazy accept** — sentinel's directly-registered streams
   only accept client connections during publish(). Clients connecting
   between publishes get "connection refused". Fix: background accept thread.

3. **Stop event race** — last few events during shutdown may not reach
   subscribers before the stream closes. Fix: graceful drain before exit.

4. **Discovery shows self** — `--list` includes the discovery client's
   own tool ID. Fix: filter by own tool_id in client.

5. **Task output has no metadata** — adapter stream shows empty
   capabilities/metadata. Fix: pass metadata from sentinel to adapter
   via CLI args or env.

6. **Client needs status area** — task output TUI should show stream
   state (connected/disconnected) and metadata (task name, host, etc).

## Next Steps

- Implement nexus-as-files transport (FIFOs + append logs)
- Refactor sentinel to use Transport trait instead of raw file writes
- Add metadata to adapter CLI for labeled task-output streams
- Add status/state display area to gbe-client TUI
- Explore multi-stream client (Metarch) consuming all three streams
