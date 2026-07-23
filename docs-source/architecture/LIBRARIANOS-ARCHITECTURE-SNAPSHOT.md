# LibrarianOS Architecture Snapshot

**Date:** 2026-07-15  
**Status:** Post crate-separation baseline  
**Repository:** `G:\openwork\librarian-runtime-node\`  
**Preceding ADR:** AIR-Q, ADR-NODE-001  
**Mac Core:** Frozen (not yet activated)

## 0. Operational Model

LibrarianOS is a **distributed coordination architecture**, not a client/server system. Core and Nodes cooperate through contracts while maintaining different authority domains.

### Core — Canonical Coordination Domain

Core is the canonical coordination domain. It has three responsibilities:

1. **Own canonical state** — project truth, planning, product memory, governance
2. **Coordinate work** — scheduling, packet dispatch, node awareness, evidence intake
3. **Preserve Owner authority** — decisions, approval policy, capability classification

Core is **not** "the authority." Core is the domain that preserves and exercises authority on behalf of the Owner.

Core does **not** execute models, run tools, or benchmark GPUs.

**Core → Node information flow (context):**
- Project context
- Planning state
- Current sprint
- Product documentation
- Coding standards
- Capability policies
- Qualification requirements
- Work packets

Core answers: *"Here is the context you are authorized to execute within."*

### Node — Execution Environment

A Node is an **execution environment that advertises capabilities and returns evidence.** It is not "a worker" — it is a capability surface.

Nodes may be:
- GPU node (local NVIDIA/AMD)
- CPU node (inference without GPU)
- Browser node (WASM runtime)
- Cloud node (remote GPU instance)
- Phone node (on-device inference)
- Embedded device
- Simulation node

All use the same contract types.

**Node → Core information flow (evidence):**
- Execution status
- Residency state
- Runtime health
- Benchmark results
- Evidence packets
- Receipts
- Runtime metrics
- Proposed outputs

Nodes answer: *"Here is what actually happened."*

### Deployment Independence

Core and Node are logical roles, not physical machines. A single workstation can host both:

```
MacBook
   Core
      ├── Dashboard
      ├── Product DB
      ├── Planner
      └── Owner UI

   Node
      ├── Local LLM
      ├── OCR
      ├── Tools
      └── Runtime
```

Nothing crosses a network. The contracts are identical.

A distributed deployment simply moves the Node:

```
Mac                                     Windows
   Core                                      Node
      ├── Dashboard                           ├── GPU LLM
      ├── Product DB                          ├── Benchmark
      └── Owner UI                            └── Evidence
```

No architectural change required. This validates that the architecture separates deployment topology from system design.

### Capability Publication

A Node is a **runtime role**, not a machine. Nodes publish capability manifests that describe what they can do. Core plans against advertised capabilities, not hard-coded assumptions.

**Capability Manifest (node advertisement):**
```
Node: win-bigpickle-rx570

Capabilities:
  Models:      MiniCPM5, Phi-4-mini
  Tools:       OCR, Python, Git
  Hardware:    RX570, 4GB VRAM
  Concurrency: 1 model at a time
  Available:   true
```

**Three distinct concerns:**
- **Core** decides what work should be done (planning, scheduling)
- **Node** advertises what work it can do (capability manifest)
- **Owner** decides what work is allowed (policy, approval)

Each is independently authoritative in its domain. No single component owns all three.

### Crate Model

| Crate | Responsibility | Owns |
|-------|---------------|------|
| `librarian-contracts` | **Communication** | Packets, schemas, serialization, protocol contracts |
| `librarian-core` | **Coordination** | Canonical state, planning, governance, Owner workflow, scheduling |
| `librarian-node` | **Execution** | Models, tools, runtimes, hardware, evidence generation |

None of these owns the other. They exchange well-defined information:

```
Core  ──── Work / Context ────→  Node
Node  ──── Evidence / Status ──→  Core
```

---

## 1. Architecture State

### Dependency Graph

```
                 librarian-contracts
                (neutral boundary)
                    56 tests
                       ▲
                       |
          ┌────────────┴────────────┐
          │                         │
   librarian-core             librarian-node
   authority domain           execution domain
   580 tests                  85+ tests
```

### Enforced at Compile Time

| Dependency | Allowed? | Mechanism |
|-----------|----------|-----------|
| `librarian-core` → `librarian-contracts` | ✅ | Explicit `Cargo.toml` dependency |
| `librarian-node` → `librarian-contracts` | ✅ | Explicit `Cargo.toml` dependency |
| `librarian-core` → `librarian-node` | ❌ | Not listed → compile error |
| `librarian-node` → `librarian-core` | ❌ | Not listed → compile error |
| `librarian-contracts` → either | ❌ | Contracts have no authority or runtime dependencies |

---

## 2. Crate Boundaries

### 2.1 `librarian-contracts` — Neutral Contract Layer

**Purpose:** Shared packet types, validation primitives, and bridge communication types that cross the authority boundary. Owned by neither Core nor Node.

**Dependencies:** serde, serde_json, anyhow, chrono, uuid, sha2, reqwest

**Contains:**
- `QualificationRequest` — Mac→Windows work packet
- `EvidencePacket` — Windows→Mac evidence envelope
- `ResidencyStatusResponse` — Windows→Mac state query response
- `BridgeClient` — HTTP client for Core→Node communication
- Common packet types: `PacketModelIdentity`, `PacketExecutionIdentity`, `PacketLeaseLifecycle`, `PacketExecutionMetrics`, `PacketLifecycleEvent`, `PacketReleaseVerification`

**Does NOT contain:**
- Database logic
- Qualification logic
- Governance logic
- Runtime/process logic
- HTTP server
- Node state

### 2.2 `librarian-core` — Canonical Authority

**Purpose:** The authority domain. Owns canonical truth, qualification policy, governance, and decisions.

**Dependencies:** `librarian-contracts`, rusqlite, serde, serde_json, anyhow, chrono, uuid, sha2, reqwest

**Contains:**

| Module | Responsibility |
|--------|---------------|
| `db` | CanonicalDatabase — model identity, system profiles, task packs, validator packs |
| `qualification` | Runner, stages (smoke, primitive probes), validator engine, batch, custom executor |
| `capability_evidence` | Benchmark runners, adapters (lm_eval, code_needle, adversarial), quantization differential |
| `ledger` | Sprint governance — authorization, receipts, state transitions |
| `provenance` | Model provenance builder |
| `release` | Trust packages, manifest generation |
| `routing` | Canonical routing projections, execution profiles |
| `capability` | Capability manifest, decisions |
| `comparative` | Comparative analysis — analyzer, audit, finding, roster |
| `review` | Review construction |
| `lifecycle` | Lifecycle transitions, history |
| `observability` | Observability service |
| `pipeline` | Pipeline integration tests |
| `registry` | Registry store |
| `models` | Core data models |
| `bridge` | Bridge client (re-exported via contracts) |

**Does NOT contain:**
- Process management
- GPU residency management
- HTTP server endpoints
- Evidence collection (Node responsibility)
- Windows-specific code

### 2.3 `librarian-node` — Runtime Execution

**Purpose:** The execution domain. Owns model execution, residency, evidence generation, and operational state.

**Dependencies:** `librarian-contracts`, axum, tokio, tower, tower-http, serde, serde_json, reqwest, tracing, tracing-subscriber, clap, chrono, uuid, rusqlite, sha2, anyhow

**Contains:**

| Module | Responsibility |
|--------|---------------|
| `server` | Axum HTTP server, route definitions, AppState |
| `process` | BackendProcess — child process lifecycle, llama-server.exe supervision |
| `residency` | 8-state residency supervisor (Unloaded→Loading→Ready→Running→Draining→Unloading→VerifyingRelease→Unloaded→Failed) |
| `evidence` | EvidenceWriter, evidence export, residency status construction |
| `db` | RuntimeDatabase — Windows operational DB (6 tables) |
| `runtime_state` | ModelLease, RuntimeRun, lifecycle evidence models |
| `operator` | OperatorService — dashboard models, event store |
| `config` | RouterConfig, ProfileManager, Profile types |
| `refusal` | Request refusal logic |
| `models` | LocalModel, RuntimeProfile, HardwareProfile |

**Does NOT contain:**
- Canonical database
- Qualification logic
- Governance or ledger
- Capability policy
- Provenance

---

## 3. Architectural Progression

```
AIR-Q                    Qualification boundaries established
  ↓
MQR                      Evidence flow and custody protocol
  ↓
ADR-NODE-001             Authority separation decision accepted
  ↓
G-CONTRACTS              Neutral contract layer enforced
  ↓
G-CORE                   Canonical authority extracted
  ↓
G-NODE                   Runtime execution extracted
  ↓
WORKSPACE-CLOSURE        Monolith removed, architecture enforced
                         ↓
                    Current baseline
```

---

## 4. Validated Properties

| Property | Evidence |
|----------|----------|
| Contracts isolated from authority/runtime | `librarian-contracts` has no DB, process, or HTTP deps |
| Core builds independently | `cargo build -p librarian-core --release` |
| Node builds independently | `cargo build -p librarian-node --release` |
| Core cannot depend on Node | `librarian-core/Cargo.toml` does not list `librarian-node` |
| Node cannot depend on Core | `librarian-node/Cargo.toml` does not list `librarian-core` |
| Packet serialization preserved | Round-trip tests pass in contracts crate |
| Packet validation preserved | Validate tests pass in contracts crate |
| Qualification unchanged | 580 core tests pass |
| Runtime unchanged | 85+ node tests pass |
| No schema changes | All DB migrations and protocol fields unchanged |
| No feature additions | Zero new types, endpoints, or dependencies added during extraction |
| Monolith removed | `rust-router/` crate directory deleted |

---

## 5. Migration Exception Log

| Exception | Reason | Owner | Removal Condition |
|-----------|--------|-------|-------------------|
| None | Every file moved mechanically | — | — |

---

## 6. Future Activation Triggers

The following events would trigger Mac Core activation or further architectural work. None are active.

| Trigger | Description | Prerequisites |
|---------|-------------|---------------|
| **Core runtime activation** | `librarian-core` becomes a running authority service/process | ADR-NODE-001 already accepted; crate exists |
| **Canonical state migration** | Canonical DB and authority records move to Core-owned deployment | Core crate is stable; deployment target identified |
| **Owner workflow activation** | Owner decisions become operational through Core | Core qualification pipeline is complete; decision models exist |
| **Multi-node operation** | Multiple runtime nodes require registration, reconciliation, routing | Core crate exists; contracts define boundary |
| **External agent boundary** | MCP or other transport becomes necessary for agent↔Core interaction | Contracts define MCP tool proposals; no implementation yet |

**Current state:** Windows owns execution. Core owns authority logic. Contracts define the boundary. Mac remains the future deployment target.

---

## 7. Key Documents

| Document | Location | Relevance |
|----------|----------|-----------|
| ADR-NODE-001 | `G:\Models\docs\architecture\decision-records\ADR-NODE-001-DISTRIBUTED-LIBRARIAN-AUTHORITY-MODEL.md` | Authority separation decision |
| Architecture mapping | `G:\Models\docs\planning\LIBRARIANOS-CORE-NODE-ARCHITECTURE-MAPPING.md` | Discovery findings |
| Epic definition | `docs/planning/EPIC-CORE-NODE-CRATE-SEPARATION.md` | Execution plan |
| Epic report | `docs/planning/EPIC-CORE-NODE-CRATE-SEPARATION-REPORT.md` | Completion evidence |
| Audit report | `docs/planning/CORE-NODE-DEPENDENCY-BOUNDARY-AUDIT-REPORT.md` | Pre-extraction dependency analysis |
| This snapshot | `docs/architecture/LIBRARIANOS-ARCHITECTURE-SNAPSHOT.md` | Post-extraction baseline |
