# ADR-PLATFORM-002 — Platform Lifecycle

**Status:** Accepted  
**Date:** 2026-07-19  
**Preceded By:** ADR-PLATFORM-001-CORE-NODE-AUTHORITY-ARCHITECTURE.md  
**Workspace:** G:\Models  
**Purpose:** Define the lifecycle of a Librarian installation, distinguishing component-local states from platform relationship states

---

## 1. Context

ADR-PLATFORM-001 established the platform architecture: Core / Node / MCP / Shared Contracts with an Installability layer. That ADR answers *what the platform is*. This ADR answers *how the platform lives*.

**Critical distinction:** The lifecycle mixes two different kinds of transitions:
- **Component-local states** — The component is self-sufficient, not yet interacting with the wider platform
- **Platform relationship states** — The component is interacting with other components, discovering, being admitted, serving

This distinction matters because:
- A standalone Node can legitimately stop at READY
- A development Core might stop at READY without advertising MCP
- An offline installation might never progress beyond READY until later
- Not every component participates in every platform relationship

**The lifecycle is the contract between:**
- Installers (how do I get from nothing to running?)
- First-run experience (what happens on first boot?)
- Upgrades (how do I move between versions?)
- Migrations (how do I move between machines?)
- Disaster recovery (how do I recover from failure?)
- Backups (what do I need to save?)
- Portability (how do I move an installation?)

---

## 2. Platform Invariants

Before defining the lifecycle, we establish platform-wide invariants that hold across all states:

### Invariant 1: Evidence is Append-Only

**State may change; evidence does not.**

Lifecycle evidence is never modified or deleted. Every state transition generates evidence. Recovery evidence is recorded alongside normal evidence. This invariant reinforces the custody model built across both Core and Node.

### Invariant 2: Rollback is First-Class

Every lifecycle phase must support rollback to the previous state. Rollback is not an error path — it is a designed operation.

### Invariant 3: Installability Precedes Activation

The installer prepares the platform. It does not start the platform. Installation, initialization, qualification, and identity creation are separate from Core activation, MCP exposure, and node admission.

### Invariant 4: Migration is Cross-Machine

Migration is not a lifecycle state. It is a protocol that spans two lifecycles: retirement on the old machine and installation on the new machine.

---

## 3. Two-Phase Lifecycle

Every component follows a two-phase lifecycle:

### Phase 1: Component-Local Lifecycle

The component becomes self-sufficient without any platform relationships.

```
ABSENT
  │
  │ [1] Install binaries
  ▼
INSTALL
  │
  │ [2] Create databases, apply migrations
  ▼
INITIALIZE
  │
  │ [3] Measure hardware, profile models (Node only)
  ▼
QUALIFY (Node only)
  │
  │ [4] Generate identity, create keypair
  ▼
IDENTITY
  │
  │ [5] Populate database with capabilities
  ▼
READY
```

**At READY, the component is self-sufficient.** It can:
- Execute work locally (Node)
- Accept proposals locally (Core, if activated)
- Generate evidence
- Operate offline indefinitely

**A standalone Node can legitimately stop here.**

### Phase 2: Platform Relationship Lifecycle

The component interacts with the wider platform.

```
READY
  │
  │ [6] Become visible to platform
  ▼
DISCOVERED
  │
  │ [7] Evaluate trust, validate capabilities
  ▼
CANDIDATE
  │
  │ [8] Complete registration handshake
  ▼
ADMITTED
  │
  │ [9] Begin serving
  ▼
OPERATIONAL
```

**Not all components reach OPERATIONAL.** A standalone Node at READY is still a valid installation.

---

## 4. Node Lifecycle

The Node follows the generic lifecycle with Node-specific states:

### Phase 1: Component-Local

```
ABSENT
  │
  │ install binaries
  ▼
INSTALL
  │
  │ create operational DB, apply migrations
  ▼
INITIALIZE
  │
  │ measure hardware, profile models
  ▼
QUALIFY
  │
  │ generate node identity, create keypair
  ▼
IDENTITY
  │
  │ populate runtime DB with capabilities
  ▼
READY
```

**At READY, the Node is:**
- Fully installed
- Hardware qualified
- Identity established
- Capabilities registered
- Ready for admission to a Core

**The Node does NOT require:**
- Owner identity (Core owns identity)
- Core activation (Node is execution only)
- MCP exposure (Node is not agent-facing)

### Phase 2: Platform Relationship

```
READY
  │
  │ become visible to Core
  ▼
DISCOVERED
  │
  │ Core evaluates trust, validates capabilities
  ▼
CANDIDATE
  │
  │ complete registration handshake with Core
  ▼
ADMITTED
  │
  │ begin executing work packets
  ▼
OPERATIONAL
```

### Node Lifecycle Complete

```
ABSENT → INSTALL → INITIALIZE → QUALIFY → IDENTITY → READY
                                                          │
                                                    DISCOVERED
                                                          │
                                                      CANDIDATE
                                                          │
                                                      ADMITTED
                                                          │
                                                      OPERATIONAL
```

---

## 5. Core Lifecycle

The Core follows the generic lifecycle with Core-specific states:

### Phase 1: Component-Local

```
ABSENT
  │
  │ install binaries
  ▼
INSTALL
  │
  │ create canonical DB, apply migrations
  ▼
INITIALIZE
  │
  │ (no hardware qualification for Core)
  ▼
IDENTITY
  │
  │ generate owner identity, create keypair
  ▼
IDENTITY
  │
  │ activate Core authority
  ▼
CORE ACTIVE
  │
  │ expose MCP tools
  ▼
MCP AVAILABLE
  │
  │ Core is self-sufficient
  ▼
READY
```

**At READY, the Core is:**
- Fully installed
- Owner identity established
- Core authority active
- MCP tools available
- Ready to discover and admit nodes

**The Core does NOT require:**
- Any Nodes (Core is self-sufficient)
- Hardware qualification (Core does not execute models)

### Phase 2: Platform Relationship

```
READY
  │
  │ begin discovering nodes
  ▼
DISCOVERING NODES
  │
  │ evaluate node trust, validate capabilities
  ▼
ADMITTING NODES
  │
  │ complete registration handshakes
  ▼
OPERATIONAL
```

### Core Lifecycle Complete

```
ABSENT → INSTALL → INITIALIZE → IDENTITY → CORE ACTIVE → MCP AVAILABLE → READY
                                                                              │
                                                                      DISCOVERING NODES
                                                                              │
                                                                      ADMITTING NODES
                                                                              │
                                                                          OPERATIONAL
```

---

## 6. Platform Lifecycle (Single-Node)

When Core and Node are on the same machine (single-node deployment):

```
ABSENT
  │
  │ install binaries
  ▼
INSTALL
  │
  │ create databases, apply migrations
  ▼
INITIALIZE
  │
  │ measure hardware, profile models
  ▼
QUALIFY
  │
  │ generate identities (owner + node)
  ▼
IDENTITY
  │
  │ activate Core, expose MCP, register capabilities
  ▼
READY
  │
  │ Core discovers local Node
  ▼
DISCOVERED
  │
  │ Core evaluates local Node
  ▼
CANDIDATE
  │
  │ Core admits local Node
  ▼
ADMITTED
  │
  │ begin serving
  ▼
OPERATIONAL
```

---

## 7. State Definitions

### 7.1 ABSENT

**Definition:** No Librarian software is installed on the machine.

**Characteristics:**
- No binaries, no databases, no configuration
- No node identity, no trust state
- No evidence, no receipts
- Machine is unaware of Librarian

**Transitions:**
- → INSTALL (begin installation)

### 7.2 INSTALL

**Definition:** Platform binaries are placed on the machine.

**What happens:**
- Rust binaries placed in installation directory
- Runtime dependencies placed
- PowerShell operational scripts placed
- Configuration templates placed
- Installation manifest created (binary hashes, versions, platform)

**Acceptance gates:**
- [ ] All binaries present and executable
- [ ] Installation manifest generated
- [ ] Binary hashes match expected values
- [ ] Platform requirements verified (OS, GPU, Vulkan)
- [ ] Rollback possible (installation can be undone)

**Transitions:**
- → INITIALIZE (create databases and configuration)
- → ABSENT (rollback: remove everything)

### 7.3 INITIALIZE

**Definition:** Databases are created, configuration is populated, operational state is established.

**What happens:**
- Databases created (canonical for Core, operational for Node)
- Schema migrations applied
- Configuration files populated from templates
- First lifecycle evidence is recorded

**Acceptance gates:**
- [ ] Databases created and migrations applied
- [ ] Configuration files valid
- [ ] Database health check passes
- [ ] Initialization evidence recorded

**Transitions:**
- → QUALIFY (Node: begin hardware qualification)
- → IDENTITY (Core: begin identity creation)
- → INSTALL (rollback: remove databases and config)

### 7.4 QUALIFY

**Definition:** Hardware and runtime are measured, profiled, and qualified. **Node-only state.**

**What happens:**
- llama.cpp binary inventoried (SHA-256, version, build metadata)
- Vulkan device discovery verified
- Each model loaded, profiled, and unloaded
- VRAM measurements recorded
- Token throughput benchmarks recorded
- Hardware profiles written to database
- Runtime profiles written to database
- Qualification evidence recorded

**Acceptance gates:**
- [ ] llama.cpp binary hash and version captured
- [ ] Vulkan device discovered
- [ ] Each model loads, produces inference, unloads cleanly
- [ ] VRAM measurements recorded in DB
- [ ] Token throughput measured at multiple context sizes
- [ ] Clean unload + VRAM release verified
- [ ] Sequential model switching test passes
- [ ] All measurements written to runtime_profiles and hardware_profiles
- [ ] Qualification evidence recorded

**Transitions:**
- → IDENTITY (create node identity)
- → INITIALIZE (rollback: clear qualification data)

### 7.5 IDENTITY

**Definition:** Identity is created and the installation is claimed.

**For Node:** Node identity is created.
**For Core:** Owner identity is created.

**What happens (Node):**
- Node identity record created
- Node keypair generated
- Node ID assigned (e.g., "win-bigpickle-rx570-001")
- Trust state set to "registered"
- Identity evidence recorded

**What happens (Core):**
- Owner identity record created
- Owner keypair generated
- Installation bound to owner
- Trust anchor established
- Identity evidence recorded

**Acceptance gates:**
- [ ] Identity record created
- [ ] Keypair generated
- [ ] ID assigned and unique
- [ ] Trust state established
- [ ] Identity evidence recorded

**Transitions:**
- → READY (component is self-sufficient)
- → QUALIFY (rollback: clear identity)

### 7.6 READY

**Definition:** Component is self-sufficient and ready for platform relationships.

**Characteristics:**
- Fully installed
- Identity established
- Capabilities registered (Node)
- Core authority active (Core, if activated)
- MCP tools available (Core, if activated)
- Can operate offline indefinitely
- Can execute work locally (Node)
- Can accept proposals locally (Core)

**This is a valid terminal state for standalone operation.**

**Acceptance gates:**
- [ ] Component installed and configured
- [ ] Identity established
- [ ] Capabilities registered (Node)
- [ ] Core authority active (Core)
- [ ] MCP tools available (Core)
- [ ] Component self-sufficient

**Transitions:**
- → DISCOVERED (become visible to platform)
- → RECOVER (fault repair)
- → RETIRE (graceful removal)

### 7.7 DISCOVERED

**Definition:** Component is visible to the platform and can be evaluated.

**What happens (Node):**
- Node becomes visible to Core
- Node manifest is exportable
- Node capabilities are advertised
- Node trust state is evaluated

**What happens (Core):**
- Core begins discovering nodes
- Node manifests are received
- Node capabilities are evaluated

**Acceptance gates:**
- [ ] Component visible to platform
- [ ] Component manifest exportable
- [ ] Component capabilities advertised
- [ ] Component trust state evaluated

**Transitions:**
- → CANDIDATE (trust evaluation in progress)
- → READY (rollback: become invisible)

### 7.8 CANDIDATE

**Definition:** Component is being evaluated for admission.

**What happens:**
- Trust evaluation continues
- Capabilities are validated
- Identity is verified
- Authority grant is prepared

**Acceptance gates:**
- [ ] Trust evaluation complete
- [ ] Capabilities validated
- [ ] Identity verified
- [ ] Authority grant prepared

**Transitions:**
- → ADMITTED (trust established, admission complete)
- → DISCOVERED (rollback: revoke evaluation)

### 7.9 ADMITTED

**Definition:** Component is trusted and admitted to the platform.

**What happens (Node):**
- Registration handshake completes
- Authority grant received
- Node is ready to execute work
- Admission evidence recorded

**What happens (Core):**
- Node registration handshake completes
- Node identity verified
- Node capabilities validated
- Trust state set to "verified"
- Authority grant issued
- Admission evidence recorded

**Acceptance gates:**
- [ ] Registration handshake completes
- [ ] Identity verified
- [ ] Capabilities validated
- [ ] Trust state set to "verified"
- [ ] Authority grant issued/received
- [ ] Admission evidence recorded

**Transitions:**
- → OPERATIONAL (begin serving)
- → DISCOVERED (rollback: revoke admission)

### 7.10 OPERATIONAL

**Definition:** Component is running, serving requests, and processing work.

**Characteristics:**
- Core is accepting proposals
- MCP tools are available to agents
- Nodes are executing work packets
- Evidence is being generated and collected
- Receipts are being recorded
- Health monitoring is active
- All subsystems are operational

**This is the steady state.** Platform can remain here indefinitely.

**Acceptance gates:**
- [ ] Core accepting proposals
- [ ] MCP tools available
- [ ] Nodes executing work
- [ ] Evidence pipeline flowing
- [ ] Health monitoring active
- [ ] All subsystems healthy

**Transitions:**
- → UPGRADE (version change)
- → RECOVER (fault repair)
- → RETIRE (graceful removal)

### 7.11 UPGRADE

**Definition:** Component version changes.

**What happens:**
- Current state is captured (backup)
- New binaries are installed
- Database migrations are applied
- Configuration is migrated
- Health check passes
- Upgrade evidence is recorded
- Rollback is possible

**Acceptance gates:**
- [ ] Current state backed up
- [ ] New binaries installed
- [ ] Database migrations applied
- [ ] Configuration migrated
- [ ] Health check passes
- [ ] Upgrade evidence recorded
- [ ] Rollback tested

**Transitions:**
- → OPERATIONAL (upgrade complete)
- → RECOVER (rollback on failure)

### 7.12 RECOVER

**Definition:** Fault is repaired, component is restored.

**What happens:**
- Failure is diagnosed
- Recovery plan is created
- State is restored from backup (if needed)
- Databases are repaired (if needed)
- Health check passes
- Recovery evidence is recorded

**Acceptance gates:**
- [ ] Failure diagnosed
- [ ] Recovery plan created
- [ ] State restored (if needed)
- [ ] Databases repaired (if needed)
- [ ] Health check passes
- [ ] Recovery evidence recorded

**Transitions:**
- → OPERATIONAL (recovery complete)
- → RETIRE (if recovery fails)

### 7.13 RETIRE

**Definition:** Component is gracefully removed.

**What happens:**
- Active work is completed or cancelled
- Evidence is exported and archived
- Databases are backed up
- Configuration is archived
- Binaries are removed
- Installation directory is cleaned up
- Retirement evidence is recorded

**Acceptance gates:**
- [ ] Active work completed or cancelled
- [ ] Evidence exported and archived
- [ ] Databases backed up
- [ ] Configuration archived
- [ ] Binaries removed
- [ ] Installation directory cleaned
- [ ] Retirement evidence recorded

**Transitions:**
- → ABSENT (component removed)

---

## 8. Lifecycle Constraints

### 8.1 State Ordering

States must be visited in order for initial installation. The exact sequence depends on component type (Node vs Core).

**Node:**
```
ABSENT → INSTALL → INITIALIZE → QUALIFY → IDENTITY → READY
```

**Core:**
```
ABSENT → INSTALL → INITIALIZE → IDENTITY → CORE ACTIVE → MCP AVAILABLE → READY
```

**Violation is a platform error.** You cannot activate Core before creating identity. You cannot admit nodes before discovering them.

### 8.2 Backward Transitions

Backward transitions are allowed only as rollback within a phase:

**Component-Local:**
```
INITIALIZE → INSTALL    (rollback: remove databases)
QUALIFY → INITIALIZE    (rollback: clear qualification)
IDENTITY → QUALIFY      (rollback: clear identity)
READY → IDENTITY        (rollback: clear capabilities)
```

**Platform Relationship:**
```
DISCOVERED → READY      (rollback: become invisible)
CANDIDATE → DISCOVERED  (rollback: revoke evaluation)
ADMITTED → CANDIDATE    (rollback: revoke admission)
```

**Rollback is a first-class operation.** Every phase must support rollback to the previous state.

### 8.3 Operational Transitions

From OPERATIONAL, the component can transition to:

```
OPERATIONAL → UPGRADE    (version change)
OPERATIONAL → RECOVER    (fault repair)
OPERATIONAL → RETIRE     (graceful removal)
```

**These are the only forward transitions from OPERATIONAL.**

### 8.4 Recovery Transitions

From any state, the component can transition to:

```
any state → RECOVER      (fault repair)
any state → RETIRE       (graceful removal)
```

**Recovery and retirement are always available.**

---

## 9. Migration Protocol

Migration is not a lifecycle state. It is a cross-machine protocol that spans two lifecycles.

### 9.1 Migration Sequence

```
OLD MACHINE                              NEW MACHINE
    │                                        │
    │ OPERATIONAL                            │ ABSENT
    │                                        │
    │ ── Export Durable State ──►            │
    │    (databases, config,                 │
    │     evidence, identity)                │
    │                                        │
    │ ── Verify Export ──►                   │
    │    (checksums, completeness)           │
    │                                        │
    │ ── Transfer ──►                        │
    │    (physical or network)               │
    │                                        │
    │                                        │ ── Import Durable State ──►
    │                                        │    (databases, config,
    │                                        │     evidence, identity)
    │                                        │
    │                                        │ ── Re-qualify Hardware ──►
    │                                        │    (new machine has different
    │                                        │     hardware)
    │                                        │
    │                                        │ ── Resume ──►
    │                                        │    (OPERATIONAL on new machine)
    │                                        │
    │ RETIRE                                 │
    │ (graceful removal of old installation) │
```

### 9.2 What Transfers

| Item | Transfers? | Notes |
|------|-----------|-------|
| Databases | Yes | Canonical + operational |
| Evidence | Yes | All lifecycle evidence |
| Configuration | Yes | May need adjustment for new hardware |
| Identity | Yes | Node/Owner identity + keypair |
| Trust state | Yes | Trust anchor preserved |
| Binaries | No | Reinstall on new machine |
| Runtime state | No | Reconstruct on new machine |
| Model files | No | Re-download on new machine |

### 9.3 Migration Constraints

- Hardware qualification must be re-run on new machine
- Identity may need to be re-established (if hardware-based)
- Evidence must be preserved across migration
- Old installation must be retired after migration completes

---

## 10. Lifecycle Evidence

Every state transition generates evidence:

### 10.1 Component-Local Evidence

| Transition | Evidence Type | Contents |
|------------|--------------|----------|
| → INSTALL | `installation_started` | Binary paths, platform, timestamp |
| INSTALL → | `installation_completed` | Binary hashes, manifest, timestamp |
| → INITIALIZE | `initialization_started` | DB paths, config paths, timestamp |
| INITIALIZE → | `initialization_completed` | DB versions, migration count, timestamp |
| → QUALIFY | `qualification_started` | Model list, hardware profile, timestamp |
| QUALIFY → | `qualification_completed` | Measurements, profiles, benchmarks, timestamp |
| → IDENTITY | `identity_created` | ID, keypair reference, timestamp |
| → READY | `component_ready` | Capabilities, Core status, timestamp |

### 10.2 Platform Relationship Evidence

| Transition | Evidence Type | Contents |
|------------|--------------|----------|
| → DISCOVERED | `discovery_started` | Discovery method, timestamp |
| → CANDIDATE | `evaluation_started` | Evaluation criteria, timestamp |
| → ADMITTED | `admission_completed` | Trust state, authority grant, timestamp |
| → OPERATIONAL | `operational_started` | Subsystem status, timestamp |

### 10.3 Operational Evidence

| Transition | Evidence Type | Contents |
|------------|--------------|----------|
| → UPGRADE | `upgrade_started` | Old version, new version, timestamp |
| UPGRADE → | `upgrade_completed` | New version, migration count, timestamp |
| → RECOVER | `recovery_started` | Failure type, recovery plan, timestamp |
| RECOVER → | `recovery_completed` | Repairs made, timestamp |
| → RETIRE | `retirement_started` | Active work count, timestamp |
| RETIRE → | `retirement_completed` | Archives, cleanup, timestamp |

**Evidence is append-only.** Lifecycle evidence is never modified or deleted. This is a platform-wide invariant.

---

## 11. Lifecycle and Installability

### 11.1 Installer Responsibilities

The installer must:
1. Place binaries (INSTALL)
2. Create databases and config (INITIALIZE)
3. Measure hardware (QUALIFY, Node only)
4. Create identity (IDENTITY)
5. Support rollback at each step

The installer must NOT:
- Activate Core (that is a separate step)
- Admit nodes (that is a separate step)
- Start serving (that is a separate step)

**The installer prepares the platform. It does not start the platform.**

### 11.2 First-Run Experience

First run is the sequence: INSTALL → INITIALIZE → QUALIFY → IDENTITY → READY → DISCOVERED → CANDIDATE → ADMITTED → OPERATIONAL

**First run can be interactive or automated.** The platform must support both:
- Interactive: User is prompted for identity, node selection, etc.
- Automated: Configuration file provides all answers

### 11.3 Upgrade Experience

Upgrade is the sequence: OPERATIONAL → UPGRADE → OPERATIONAL

**Upgrade must:**
1. Capture current state (backup)
2. Install new binaries
3. Run database migrations
4. Migrate configuration
5. Verify health
6. Resume operation

**Upgrade must NOT:**
- Lose evidence
- Lose receipts
- Lose node trust state
- Break backward compatibility within a major version

### 11.4 Backup

**What to backup:**
- Canonical database
- Operational database
- Configuration files
- Evidence archives
- Owner identity
- Node trust state

**What NOT to backup:**
- Binaries (can be reinstalled)
- Runtime state (can be reconstructed)
- Model files (can be re-downloaded)

**Backup is a point-in-time snapshot of the platform's durable state.**

### 11.5 Disaster Recovery

**Recovery is a first-class operation.** Every state must support recovery to OPERATIONAL.

**Recovery paths:**
- Database corruption → Restore from backup, replay evidence
- Binary corruption → Reinstall, re-qualify
- Configuration loss → Restore from backup, re-qualify
- Identity loss → Restore from backup (identity is critical)
- Node trust loss → Re-admit nodes

**Recovery evidence is recorded.** Recovery is not silent.

---

## 12. Lifecycle and Multi-Node

### 12.1 Single-Node Lifecycle

The lifecycle above describes a single-node installation. Core and Node are on the same machine.

### 12.2 Multi-Node Lifecycle

When Core and Node are on different machines:

**Core machine lifecycle:**
```
ABSENT → INSTALL → INITIALIZE → IDENTITY → CORE ACTIVE → MCP AVAILABLE → READY
                                                                              │
                                                                      DISCOVERING NODES
                                                                              │
                                                                      ADMITTING NODES
                                                                              │
                                                                          OPERATIONAL
```

**Node machine lifecycle:**
```
ABSENT → INSTALL → INITIALIZE → QUALIFY → IDENTITY → READY
                                                          │
                                                    DISCOVERED
                                                          │
                                                      CANDIDATE
                                                          │
                                                      ADMITTED
                                                          │
                                                      OPERATIONAL
```

**Node does not need:**
- Owner identity (Core owns identity)
- Core activation (Node is execution only)
- MCP exposure (Node is not agent-facing)

**Node needs:**
- Hardware qualification (must be measured)
- Registration with Core (must be admitted)
- Evidence export (must return evidence to Core)

### 12.3 Cross-Machine Evidence Flow

```
Node OPERATIONAL → evidence generated → evidence exported → evidence transferred
                                                                    ↓
Core OPERATIONAL ← evidence ingested ← evidence validated ← evidence received
```

**Evidence flows one direction: Node → Core.**

---

## 13. Decisions

### Decision 1: Lifecycle is the Governing Document

The platform lifecycle defined in this ADR is the single source of truth for:
- Installer sequencing
- First-run experience
- Upgrade paths
- Migration procedures
- Disaster recovery
- Backup requirements
- Portability mechanisms

All installability epics must implement lifecycle states in order.

### Decision 2: Component-Local vs Platform Relationship

The lifecycle distinguishes two phases:
- **Component-local:** The component becomes self-sufficient without platform relationships
- **Platform relationship:** The component interacts with other components

This distinction preserves standalone operation and allows components to stop at READY.

### Decision 3: Evidence is Append-Only (Platform Invariant)

**State may change; evidence does not.** Lifecycle evidence is never modified or deleted. Every state transition generates evidence. Recovery evidence is recorded alongside normal evidence. This invariant reinforces the custody model built across both Core and Node.

### Decision 4: Rollback is First-Class (Platform Invariant)

Every lifecycle phase must support rollback to the previous state. Rollback is not an error path — it is a designed operation.

### Decision 5: Installability Precedes Activation (Platform Invariant)

The installer prepares the platform. It does not start the platform. Installation, initialization, qualification, and identity creation are separate from Core activation, MCP exposure, and node admission.

### Decision 6: Migration is Cross-Machine Protocol

Migration is not a lifecycle state. It is a protocol that spans two lifecycles: retirement on the old machine and installation on the new machine.

### Decision 7: Backup is Durable State Only

Backup captures databases, configuration, evidence, and identity. It does not capture binaries, runtime state, or model files. These can be reconstructed.

---

## 14. Consequences

### Positive

1. **Single governing document** — Every installability epic references this ADR
2. **Consistent sequencing** — All subsystems follow the same lifecycle
3. **Rollback by design** — Not an afterthought, but a requirement
4. **Evidence completeness** — Every transition is recorded
5. **Migration clarity** — Cross-machine operations are well-defined
6. **Multi-node support** — Lifecycle scales from single-node to distributed
7. **Standalone operation** — Components can stop at READY
8. **Platform invariants** — Evidence, rollback, installability, migration are consistent

### Negative

1. **Lifecycle adds states** — More states means more transitions to implement
2. **Evidence overhead** — Every transition generates evidence
3. **Rollback complexity** — Every phase must support rollback

### Risks

1. **Lifecycle rigidity** — If the lifecycle is wrong, every subsystem is wrong. Mitigation: lifecycle is a living document, updated as implementation reveals issues.
2. **Evidence storage growth** — Append-only evidence grows without bound. Mitigation: evidence archival policy (future ADR).

---

## 15. Compliance

| Entity | Must | Must Not |
|--------|------|----------|
| Installer | Follow lifecycle states in order | Skip states or activate Core prematurely |
| First-run | Support interactive and automated modes | Require manual intervention for standard flows |
| Upgrade | Backup before, migrate during, verify after | Lose evidence or break backward compatibility |
| Migration | Export durable state, re-qualify hardware | Assume hardware is identical |
| Backup | Capture databases, config, evidence, identity | Capture binaries or runtime state |
| Recovery | Record recovery evidence, restore to OPERATIONAL | Silently recover without evidence |
| All transitions | Generate lifecycle evidence | Skip evidence generation |
| All components | Respect platform invariants | Violate evidence, rollback, or installability invariants |

---

## 16. Decision Summary

| Question | Decision |
|----------|----------|
| What governs installability? | **Platform lifecycle (this ADR)** |
| Is rollback a first-class operation? | **Yes. Every phase supports rollback.** |
| Is evidence append-only? | **Yes. Platform-wide invariant.** |
| Does installer activate Core? | **No. Installer prepares; activation is separate.** |
| Is migration a lifecycle state? | **No. Cross-machine protocol spanning two lifecycles.** |
| What is backed up? | **Databases, config, evidence, identity. Not binaries or runtime state.** |
| Can components operate standalone? | **Yes. Components can stop at READY.** |
| What are the two lifecycle phases? | **Component-local (self-sufficient) and Platform relationship (interacting)** |

---

## 17. References

1. ADR-PLATFORM-001-CORE-NODE-AUTHORITY-ARCHITECTURE.md — Platform architecture
2. ADR-NODE-001-DISTRIBUTED-LIBRARIAN-AUTHORITY-MODEL.md — Original Core/Node ADR
3. ARCHITECTURAL-BOUNDARY-MAP.md — Module-to-crate mapping
4. LOCAL-MODEL-ORCHESTRATION-SPRINT-PLAN.md — Sprint definitions
5. custody-ledger-spec.md — Evidence and receipt specifications
6. result-artifact-intake-boundary.md — Evidence handoff protocol
