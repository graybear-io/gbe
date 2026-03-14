# Implementation Status

Updated: 2026-03-14

---

## Progress

| Component | Contracts | Implementation | Tests |
|-----------|-----------|----------------|-------|
| Nexus | Done | Redis backends working | Yes |
| Oracle | Done | SimpleOracle + Driver | 16 |
| Operative | Done | Shell, HTTP, LLM, Molecule | Yes |
| Sentinel | Done | Stubs (SlotTracker, config, protocol) | Partial |
| Watcher | Done | Sweep + archiver working | 15+ |
| Envoy | Done | Router, adapter, buffer, client, proxy | Yes |
| Cryptum | Done | ttyd-connect working, Metarch planned | 10 |
| Ark | Done | Provisioning done | N/A |
| Harness | Done | Agent loop working | 80 |
| Overseer | Not started | — | — |

---

## Gaps

### Sentinel: last-mile stubs
Architecture doc is detailed and mature, but `Sentinel::run()`, VM spawning, and vsock listener are stubs. Contracts (config, protocol types, slot tracker, claim logic) are solid.

### Oracle-Sentinel integration
Docs describe oracle emitting to bus and sentinel claiming from bus. Currently, `gbe-operative/src/driver.rs` runs the oracle+operative loop **in-process** (no bus, no sentinel). The bus-mediated path is designed but not wired end-to-end.

### Envoy-job pipeline separation
Envoy's protocol and data model don't intersect with Nexus subjects or job domain types. These are two parallel systems that may converge at the Overseer layer.

### NATS transport
Described as a planned alternative throughout docs. No implementation exists yet.

### Archiver S3 backend
`ArchiveWriter` trait exists with filesystem implementation. S3 writer documented but not built.

### Overseer
Referenced in naming docs and system overview. No code or detailed design.

### Harness-Operative convergence
Harness is a full agent loop (multi-turn, tool-calling). LlmOperative is single prompt→response. Harness is the planned Python operative impl but the integration isn't wired.

---

## Aligned (intent matches code)

- **Nexus**: three-stream pattern, envelope/payload separation, CAS claims
- **Oracle**: sync state machine with fail-stop semantics
- **Operative**: pluggable executors with input resolution
- **Watcher**: sweep/archive with distributed lock
- **Envoy**: adapter-centric, dual-channel protocol
- **Ark**: docs match constructor exactly
- **Cryptum**: ttyd pivot fully documented in migration notes
