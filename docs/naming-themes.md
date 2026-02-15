# Naming Themes for GBE Components

Thematic naming for system components. Each theme maps the same set of roles
to a different sci-fi aesthetic. The active theme affects binary names, log
prefixes, config sections, and CLI output.

---

## Collective Name: Ecumene

**GBE = Gray Bear Ecumene** — a self-referring acronym. The Ecumene is the
collection of all roles below. From Greek "the whole inhabited world", adopted
by Forerunner lore as the name of the civilization itself.

The roles are organs of the Ecumene. Sentinel guards, Watcher sweeps, Oracle
routes — together they form the governing body.

---

## Active: Forerunner (Ancient AI Hierarchy)

Inspired by autonomous AI constructs guarding vast infrastructure.

| Role | Component | Description |
|---|---|---|
| **Nexus** | Bus / transport layer | Central nervous system, message backbone |
| **Sentinel** | Per-host VM lifecycle manager | Guards the host, enforces boundaries |
| **Watcher** | Sweeper / archiver | Passive monitor, detects anomalies, cleans up |
| **Oracle** | Task router / scheduler | Makes routing decisions, distributes work |
| **Overseer** | Human command interface | Observer with intervention authority |
| **Operative** | Guest agent inside VM | Executes the mission, reports back |
| **Beacon** | Heartbeat / health signals | Periodic proof-of-life broadcast |
| **Custodian** | Image / artifact manager | Maintains rootfs images, kernels, manifests |
| **Architect** | System provisioner / deployer | Stands up new hosts, configures infrastructure |
| **Envoy** | Cross-cluster bridge | Relays between isolated network zones |

---

## Theme: Colony Ship (Station Crew)

A self-sustaining station where every role keeps the ship running.
Expanse / Alien aesthetic.

| Role | Component | Description |
|---|---|---|
| **Relay** | Bus / transport layer | Comms backbone between decks |
| **Sentinel** | Per-host VM lifecycle manager | Station security, compartment control |
| **Salvager** | Sweeper / archiver | Reclaims resources, scraps dead processes |
| **Marshal** | Task router / scheduler | Assigns crew to jobs |
| **Overseer** | Human command interface | Bridge officer with override authority |
| **Drone** | Guest agent inside VM | Expendable worker unit |
| **Pulse** | Heartbeat / health signals | Life-sign monitor |
| **Quartermaster** | Image / artifact manager | Manages supplies and provisions |
| **Shipwright** | System provisioner / deployer | Builds and repairs infrastructure |
| **Courier** | Cross-cluster bridge | Runs messages between ships |

---

## Theme: Defense Grid (Autonomous Network)

A distributed defense network protecting a perimeter.
Horizon Zero Dawn / Mass Effect aesthetic.

| Role | Component | Description |
|---|---|---|
| **Lattice** | Bus / transport layer | The mesh connecting all nodes |
| **Sentinel** | Per-host VM lifecycle manager | Perimeter enforcer |
| **Scrubber** | Sweeper / archiver | Cleans corrupted/stale state |
| **Arbiter** | Task router / scheduler | Judges priority, allocates resources |
| **Overseer** | Human command interface | Command authority with kill switch |
| **Spectre** | Guest agent inside VM | Autonomous field agent |
| **Ping** | Heartbeat / health signals | Radar sweep, proof-of-life |
| **Armorer** | Image / artifact manager | Prepares loadouts (VM images) |
| **Foundry** | System provisioner / deployer | Fabricates new nodes |
| **Gate** | Cross-cluster bridge | Controlled passage between zones |

---

## Implementation Notes

### Theme as Config

```toml
[display]
theme = "forerunner"   # "colony_ship", "defense_grid", or custom
```

### Where Theming Applies

- Binary names: `gbe-sentinel`, `gbe-watcher` (always use Forerunner for actual crate names)
- Log prefixes: `[sentinel:host-01]`, `[watcher]`
- CLI output: status displays, health reports
- Config file section names (optional, can alias)

### Where Theming Does NOT Apply

- Crate names (always Forerunner — `gbe-sentinel`, `gbe-watcher`)
- Bus subjects (always `gbe.tasks.*`, `gbe.events.*`)
- State store keys (always `gbe:state:*`)
- API contracts

Crate names are the canonical identity. Theming is a presentation layer only.
