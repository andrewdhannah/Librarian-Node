# LibrarianOS Node Roadmap

**Date:** 2026-07-15  
**Status:** Foundation complete — Node productization phase  
**Repository:** `G:\openwork\librarian-runtime-node\`  
**Preceding ADRs:** AIR-Q, ADR-NODE-001  
**Architecture Snapshot:** `docs/architecture/LIBRARIANOS-ARCHITECTURE-SNAPSHOT.md`

---

## Summary

### The Shift

The architecture is not missing a Core implementation. The crate separation revealed that Core already existed in code — it was co-located with Node inside a monolith. The enforcement work (G-CONTRACTS, G-CORE, G-NODE, WORKSPACE-CLOSURE) extracted the boundary that already existed conceptually.

The current phase is no longer about proving an architecture model. It is about **productizing the separated domains** — turning `librarian-node` from an internal runtime into a deployable, identity-aware, session-enforcing execution environment that can later plug into Core through well-defined contracts.

### Guiding Principle

> **Prepare the Node to be governed by Core without making the Node depend on Core.**

Windows is the first Node implementation target. All new code is platform-agnostic behind adapter traits. Mac and Linux become implementation targets, not architectural experiments.

### Design Principle: Installer vs. Bootstrap

The Node has two distinct setup layers:

| Layer | Responsibility | Handles |
|-------|---------------|---------|
| **Installer** (`LibrarianNodeSetup.exe`) | Place known-good base | Binaries, directories, permissions, service/user setup, initial config, dependencies |
| **Bootstrap** (capability within `librarian-node`) | Adapt to machine | Hardware scan, runtime selection, model sizing, backend choice, qualification validation |

**Rule of thumb:** The installer places files. The bootstrap makes the Node capable.

A traditional installer cannot reason about GPU backends, VRAM constraints, or model sizing. The bootstrap agent can — and its decisions produce qualification evidence that makes the Node's capabilities known rather than assumed. The near-term roadmap does not require the bootstrap on day one; the first installer places a known-good runtime. The bootstrap capability becomes valuable as more hardware targets, runtimes, and edge cases appear.

---

## Current State

```
┌──────────────────────────────────────────────────────────────┐
│                        Completed                             │
├──────────────────────────────────────────────────────────────┤
│ AIR-Q                    Qualification boundary             │
│ MQR                      Evidence flow                       │
│ ADR-NODE-001             Authority separation decision       │
│ Epics:                                                       │
│   G-CONTRACTS            Neutral contract layer enforced     │
│   G-CORE                 Canonical authority extracted       │
│   G-NODE                 Runtime execution extracted         │
│   WORKSPACE-CLOSURE      Monolith removed                    │
│ Architecture Snapshot    Baseline established                │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│                        Repository Today                      │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  librarian-contracts/  (56 tests, neutral)                   │
│    packets/  QualificationRequest, EvidencePacket, etc.      │
│    bridge/   BridgeClient                                    │
│                                                              │
│  librarian-core/ (580+ tests, authority domain)              │
│    db/  qualification/  capability_evidence/  ledger/        │
│    provenance/  release/  routing/  capability/              │
│    comparative/  review/  lifecycle/  registry/              │
│                                                              │
│  librarian-node/ (85+ tests, execution domain)               │
│    server/  process/  residency/  evidence/                  │
│    db/  runtime_state/  operator/  config/  refusal/         │
│                                                              │
│  Node identity?        No — machine is "Big Pickle"          │
│  Capability manifest?  No — capabilities are implicit         │
│  Session protocol?     No — agents operate freely            │
│  Evidence lifecycle?   No — evidence is files, not states    │
│  Installer?            No — must build from source           │
│  CLI?                  No — must use cargo run               │
│  SDK?                  No — no extension surface              │
└──────────────────────────────────────────────────────────────┘
```

---

## Architecture Model

```
                    Owner
                      │
              (decision authority)
                      │
                      ▼
              ┌─────────────────┐
              │ Librarian Core  │
              │ (coordination)  │
              │                 │
              │ Canonical state │
              │ Planning        │
              │ Governance      │
              │ Scheduling      │
              │ Dashboard       │
              └─────────────────┘
                 ▲           ▲
         Context │           │ Evidence
                 │           │
         ┌───────┘           └───────┐
         ▼                           ▼
  ┌──────────────┐           ┌──────────────┐
  │  Node A      │           │  Node B      │
  │  Windows GPU │           │  Mac Studio  │
  │  llama.cpp   │           │  Metal       │
  │  Tools       │           │  Tools       │
  │  Evidence    │           │  Evidence    │
  └──────────────┘           └──────────────┘
```

**Key principles:**
- Core and Nodes cooperate through contracts, not control
- Information flows both ways; authority does not
- A Node is a runtime role, not a machine (co-location is valid)
- Compute location is not authority location
- Every agent, regardless of transport, follows the same session protocol

---

## Epic Sequence

```
Phase 1 ─── Node Bootstrap + Runtime Initialization
               │
               ▼
Phase 2 ─── Node Identity + Capability Foundation
               │
               ▼
Phase 3 ─── Node Session Protocol
               │
               ▼
Phase 4 ─── Node Evidence Reconciliation
               │
               ▼
Phase 5 ─── Node Packaging + CLI
               │
               ▼
Phase 6 ─── Node SDK
               │
               ▼
Phase 7 ─── Agent Bootstrap + Owner Consent Model
               │
               ▼
Phase 8 ─── Core Runtime Activation (future)
               │
               ▼
Phase 9 ─── MCP/Agent Gateway (future)
```

---

## Phase 1: Node Bootstrap + Runtime Initialization

**Epic:** Not yet defined  
**Status:** Architectural concept — ready for formalization on signal

### Problem

Node setup is currently manual: build from source, configure paths, place model files, run the binary. There is no governed initialization workflow, no hardware scan, no capability assessment, and no evidence trail for the setup process.

### What Changes

Instead of an installer that guesses hardware and hopes the runtime works, the Node is initialized through a governed bootstrap workflow:

```
Node bootstrap
    │
    ▼
START_SESSION
    │
    ▼
Core provides Setup Policy Packet (or local fallback)
    │
    ▼
Agent performs:
  - hardware scan (GPU, VRAM, CPU, RAM, disk)
  - environment scan (OS, drivers, existing runtimes, permissions)
  - capability assessment (what can this machine provide?)
    │
    ▼
Agent proposes setup plan with Owner visibility
    │
    ▼
Owner approves (or declines) per permission class
    │
    ▼
Runtime installation (llama.cpp, model registry, config)
    │
    ▼
Qualification tests
    │
    ▼
Setup evidence generated
    │
    ▼
Node becomes REGISTERED
```

### Permission Model

The agent operates under a bounded permission model, not blanket authority:

| Class | Description | Examples | Owner interaction |
|-------|-------------|----------|-------------------|
| **Class 1** Pre-approved | Node-internal operations with no external impact | Create directories, write config files, generate identity, run diagnostics, store evidence | Silently executed; logged to audit trail |
| **Class 2** Owner confirmation | Operations that affect machine state or resources | Install runtime, download packages, modify PATH, configure services | Requires Owner approval before execution |
| **Class 3** Forbidden | Operations outside the node's governance scope | Modify OS security policies, delete user data, alter unrelated applications | Blocked by the session contract; elevated action required |

The agent may optionally run in verbose mode, showing each action with:
- what happened
- why it happened
- what changed
- what authority allowed it
- what evidence was produced

### Bootstrap Module

New module: `librarian-node/src/bootstrap/`

| Component | Description |
|-----------|-------------|
| `hardware_scan.rs` | GPU, VRAM, CPU, RAM, disk detection |
| `environment_scan.rs` | OS, drivers, existing runtimes, permissions |
| `runtime_install.rs` | Managed runtime download and installation |
| `setup_plan.rs` | Proposal generation and Owner approval flow |
| `bootstrap_evidence.rs` | Evidence generation for setup actions |

### Key Principles

- The bootstrap process itself produces evidence (setup is not outside the governance model)
- Setup policy can come from Core or a local fallback — the Node is not dependent on Core being available
- The same bootstrap flow works for Windows, Mac, and Linux — only the platform adapter changes
- After bootstrap, the Node has identity, capabilities, runtime, and qualification evidence

### Prerequisite

None — this is the first Node-side work. It creates the substrate that identity, session, and evidence epics build upon.

---

## Phase 2: Node Identity + Capability Foundation

**Epic:** `EPIC-NODE-IDENTITY-AND-CAPABILITY-FOUNDATION`  
**Status:** Ready — epic definition written  
**Effort:** New module + endpoints + contract types. No new external dependencies.

### What Is Built

| Component | Location | Description |
|-----------|----------|-------------|
| Node Identity Service | `librarian-node/src/node/identity.rs` | Persistent UUID, metadata, survival across restarts |
| Capability Manifest | `librarian-node/src/node/capabilities.rs` | Auto-detected capabilities at startup |
| Node State Machine | `librarian-node/src/node/state.rs` | UNREGISTERED → REGISTERED → ... (future states reserved) |
| Node API Endpoints | `librarian-node/src/server.rs` | GET /node/identity, /node/status, /node/capabilities |
| Contract Types | `librarian-contracts/src/node/` | NodeIdentity, NodeStatus, CapabilityManifest, NodeState |
| Platform Adapter | `librarian-node/src/platform/` | HardwareDetector trait + Windows impl |

### Key Deliverables

```
GET /node/identity → { node_id, display_name, platform, runtime_version, ... }
GET /node/status   → { identity, state, uptime_seconds }
GET /node/capabilities → { node_id, capabilities: [{ type, runtime, models }] }
```

### Dependencies

- None on Core
- None on MCP
- Contract types added to contracts crate (must remain neutral)

### Gates (12)

Identity persistence, endpoint responses, capability detection, contract neutrality, state machine validation, platform adapter compilation, Windows GPU detection, all existing tests pass.

---

## Phase 3: Node Session Protocol

**Epic:** Not yet defined (ready for formalization on signal)  
**Prerequisite:** Phase 2 (Identity) complete

### Problem

Currently agents and tools can execute operations without any session context. There is no way to associate work with a specific initialization state, agent identity, or authorization grant.

### What Changes

Every agent operation belongs to a known session:

```
Agent
    │
    ▼
START_SESSION
    │
    ▼
Node validates:
  - who is connecting?
  - what session?
  - what packet?
  - what permissions?
    │
    ▼
SESSION_ACTIVE
    │
    ▼
Authorized operations only
    │
    ▼
SESSION_CLOSE → Evidence receipt
```

**Invariant:** No session = No Librarian operations.

The transport does not matter — MCP, HTTP, CLI, local process — the same session contract applies.

### What Would Be Built

| Component | Description |
|-----------|-------------|
| Session contract types | → `librarian-contracts/src/session/` |
| Session lifecycle | INIT → REQUESTED → AUTHORIZED → ACTIVE → CLOSED |
| Session ID generation | Per-connection UUID |
| Operation gating | Middleware checks active session before execution |
| Session receipts | Evidence of what was done in the session |
| START_SESSION endpoint | POST /session/start |
| SESSION_CLOSE endpoint | POST /session/close |
| GET /session/{id} | Session status and receipt |

### Contract Types (→ `librarian-contracts`)

```rust
pub struct SessionStartRequest {
    pub agent_id: Option<String>,
    pub node_id: String,
    pub requested_capabilities: Vec<String>,
}

pub struct SessionContract {
    pub session_id: String,
    pub node_id: String,
    pub started_at: String,
    pub authorized_operations: Vec<String>,
    pub constraints: SessionConstraints,
}

pub struct SessionReceipt {
    pub session_id: String,
    pub operations_executed: u32,
    pub evidence_ids: Vec<String>,
    pub closed_at: String,
}
```

### Key Architectural Point

The session protocol is not MCP. It is the underlying contract.

MCP becomes a transport that carries session messages. The same session protocol works over:
- HTTP (direct)
- MCP (agent access)
- CLI (operator)
- Local function call (embedded)

### Prerequisite

Phase 2 (node identity) must be complete — sessions need a known node identity to anchor to.

---

## Phase 4: Node Evidence Reconciliation

**Epic:** Not yet defined  
**Prerequisite:** Phase 2 complete (sessions optional but beneficial)

### Problem

Currently evidence is written to the DB and exported as files. There is no lifecycle: evidence goes from "generated" to "exported" with no intermediate states, no acknowledgment, no reconciliation.

### What Changes

Evidence gains a tracked lifecycle:

```
Execution
    │
    ▼
Evidence Generated
    │
    ▼
Pending (local, awaiting submission)
    │
    ▼
Submitted (sent to Core)
    │
    ▼
Acknowledged (Core received)
    │
    ▼
Accepted or Rejected (Core decision)
```

The Node does not decide truth. It tracks:
- "generated"
- "submitted"
- "acknowledged"

### What Would Be Built

| Component | Description |
|-----------|-------------|
| Evidence state machine | PENDING → SUBMITTED → ACKNOWLEDGED → ACCEPTED/REJECTED |
| Evidence queue | Persisted, survives restart |
| Evidence status endpoints | GET /evidence/queue, GET /evidence/{id}/status |
| Submit endpoint | POST /evidence/submit (locally marks as submitted) |
| Acknowledge endpoint | POST /evidence/ack (Core calls this) |
| Reconciliation state | Track what evidence Core has vs. what Node has |

### Filesystem Layout

```
data/evidence/
    ├── pending/        ← newly generated, not yet submitted
    ├── submitted/      ← marked as submitted
    ├── acknowledged/   ← Core confirmed receipt
    └── rejected/       ← Core returned with reason
```

### Key Distinction

The evidence queue lives on the Node. Core does not push into the Node's storage. The Node presents evidence; Core accepts or rejects.

---

## Phase 5: Node Packaging + CLI

**Epic:** Not yet defined  
**Prerequisite:** Phases 1-4 provide the feature surface the CLI wraps

### What Changes

From "must build from source and run via cargo" to:

```
LibrarianNodeSetup.exe
    │
    ▼
Installs to C:\Program Files\LibrarianNode\
    │
    ▼
First start runs bootstrap (Phase 1), generates identity (Phase 2)
    │
    ▼
Node CLI available for operator interaction
```

### CLI Design

```
librarian-node init          # Bootstrap + identity generation
librarian-node status        # Show node state + identity
librarian-node capabilities  # Show capability manifest
librarian-node models        # List installed models
librarian-node sessions      # List active/completed sessions
librarian-node evidence      # Show evidence queue
librarian-node register      # (future) register with Core
```

### Install Layout

```
C:\Program Files\LibrarianNode\
    ├── bin\
    │   ├── librarian-node.exe
    │   └── librarian-cli.exe
    ├── config\
    │   └── node.toml
    ├── runtime\
    │   └── (llama.cpp, etc.)
    └── ...

%APPDATA%\LibrarianNode\
    ├── identity.json
    ├── node.db
    ├── evidence\
    └── logs\
```

### Platform Neutrality

The CLI concepts are platform-agnostic. A Mac or Linux node would have the same commands and same config layout, just different file paths.

---

## Phase 6: Node SDK

**Epic:** Not yet defined  
**Prerequisite:** Phases 1-5 stabilize the internal APIs the SDK wraps

### What Changes

From "modify librarian-node source to extend" to:

```python
from librarian_node import NodeExtension

node = NodeExtension()
node.register_capability(
    name="image-generation",
    handler=my_generator
)
```

Or Rust:

```rust
impl NodeCapability for MyCustomTool {
    fn execute(&self, request: TaskRequest) -> EvidencePacket {
        // ...
    }
}
```

### Extension Points

| Extension Type | What It Allows |
|----------------|----------------|
| ModelAdapter | New inference backend (e.g., ONNX, TensorRT, custom hardware) |
| ToolCapability | New tool or automation (e.g., database query, file processing) |
| HardwareProvider | New hardware detection (e.g., NPU, FPGA, custom accelerator) |
| EvidenceEmitter | New evidence type or export format |

### Important

The SDK should not require modifying Core. Third-party extensions register with the Node; the Node reports new capabilities in its manifest. Core discovers capabilities through the manifest, not through SDK hooks.

---

## Phase 7: Agent Bootstrap + Owner Consent Model

**Epic:** Not yet defined  
**Prerequisite:** Phase 3 (Session Protocol) complete

### What This Solves

An agent operating through MCP should not receive unbounded authority. The question is not "what tools does MCP expose?" but "how does an agent safely receive authority to perform operations on behalf of an Owner?"

### What Would Be Built

| Component | Description |
|-----------|-------------|
| Agent identity model | agent_id, agent_type, session binding |
| Operation classification | Class 1 (pre-approved), Class 2 (owner confirm), Class 3 (forbidden) |
| Owner approval surface | Confirmation requests for Class 2 operations |
| Audit trail | Every operation recorded with permission class and authorization |
| Bootstrap authorization | First-run setup uses this model to get owner consent |

### Permission Model

| Class | Description | Examples | Owner interaction |
|-------|-------------|----------|-------------------|
| **Class 1** Pre-approved | Node-internal operations, no external impact | Create directories, write config, generate identity, run diagnostics | Silently executed; logged |
| **Class 2** Owner confirmation | Affects machine state or resources | Install runtime, download packages, modify PATH, configure services | Requires Owner approval |
| **Class 3** Forbidden | Outside governance scope | Modify OS security, delete user data, alter unrelated apps | Blocked by session contract |

### Relationship to MCP

MCP is not the consent model. MCP is the transport. The consent model lives in the session protocol and the Core authority layer. MCP exposes the results of consent decisions, not the decision itself.

---

## Phase 9: Core Runtime Activation (Future)

**Preceded by:** ADR-CORE-001  
**Not yet started.** This is the point where Mac-side work begins.

### What It Requires

| Component | Description |
|-----------|-------------|
| ADR-CORE-001 | Decision on Core runtime model (embedded, local service, distributed) |
| Core runtime | `librarian-core` becomes a running service with API |
| Canonical DB ownership | Core owns the canonical DB deployment |
| Owner workflow | Interface for Owner decisions |
| Node registration | Core registers and tracks nodes |
| Session acceptance | Core validates and authorizes sessions |

### Models to Decide

**Model A — Embedded Core:** Core is a library consumed by an application. No separate service.
**Model B — Local Core Service:** Core runs as a daemon on the same machine as the Owner.
**Model C — Distributed Authority Service:** Core is a network service coordinating multiple Nodes.

### Trigger Conditions

Core activation begins when one of these is true:
- Core needs to run as an independent service
- Canonical DB moves off the development machine
- Owner decision workflow becomes operational
- Multi-node coordination is required
- External agent access (MCP) needs an authority anchor

---

## Phase 7: MCP / Agent Gateway (Future)

**Preceded by:** Phase 6 (Core Runtime)  
**Not yet started.**

### Correct Architecture

```
Agent
    │
    ▼
MCP  ← transport only
    │
    ▼
Core ← authority boundary
    │
    ▼
Session Protocol ← session boundary
    │
    ▼
Node ← execution boundary
```

MCP is not the authority boundary. It is a transport mechanism. The session protocol is the control boundary. The contracts are the data boundary.

### What It Requires

| Component | Description |
|-----------|-------------|
| MCP server | Exposes Core capabilities to agents |
| Session binding | MCP tool calls bind to a START_SESSION context |
| Tool contracts | Proposal/evidence/receipt pattern, not generic file write |
| Agent identity | Agents authenticate through Core, not directly to Node |

---

## Dependency Graph

```
Phase  1: Node Bootstrap
    │
    ▼
Phase  2: Identity + Capability
    │
    ├──► Phase 3: Session Protocol
    │       │
    │       └──► Phase 4: Evidence Reconciliation
    │
    ├──► Phase 5: Packaging + CLI
    │
    └──► Phase 6: SDK
              │
              └──► Phase 7: Agent Bootstrap + Consent Model

All of the above are prerequisites for:

    Phase 8: Core Runtime Activation (Mac side begins)
        │
        └──► Phase 9: MCP / Agent Gateway
```

Phases 1-5 require no Core activation. Phases 6-7 require Core to be a running service.

---

## What Remains Frozen

| Component | Status | Activation Trigger |
|-----------|--------|-------------------|
| Mac implementation | Frozen | Phase 8 (Core Runtime Activation) |
| MCP server | Frozen | Phase 9 |
| Agent consent UI | Frozen | Phase 7 |
| Cloud model integration | Frozen | Explicit capability need |
| Multi-node orchestration | Frozen | Explicit capability need |
| Owner workflow UI | Frozen | Phase 8 |
| Dashboard | Frozen | Phase 8 |

---

## Platform Strategy

```
librarian-node/
    ├── session/         ← portable
    ├── evidence/        ← portable
    ├── capability/      ← portable
    ├── identity/        ← portable
    ├── state/           ← portable
    └── platform/
        ├── mod.rs       ← HardwareDetector trait
        ├── windows.rs   ← first implementation
        ├── linux.rs     ← future
        └── macos.rs     ← future
```

Windows is the first adapter. The trait ensures Mac/Linux can be added later without changing node core logic.

---

## Key Documents

| Document | Location | Status |
|----------|----------|--------|
| Architecture Snapshot | `docs/architecture/LIBRARIANOS-ARCHITECTURE-SNAPSHOT.md` | ✅ Complete |
| Crate Separation Epic | `docs/planning/EPIC-CORE-NODE-CRATE-SEPARATION.md` | ✅ Complete |
| Crate Separation Report | `docs/planning/EPIC-CORE-NODE-CRATE-SEPARATION-REPORT.md` | ✅ Complete |
| Node Roadmap | `docs/planning/LIBRARIANOS-NODE-ROADMAP.md` | ✅ Current |
| Node Bootstrap Epic | — | ⏳ Not yet defined |
| Node Identity Epic | `docs/planning/EPIC-NODE-IDENTITY-AND-CAPABILITY-FOUNDATION.md` | ✅ Defined |
| Node Session Protocol Epic | — | ⏳ Not yet defined |
| Node Evidence Reconciliation Epic | — | ⏳ Not yet defined |
| Node Packaging + CLI Epic | — | ⏳ Not yet defined |
| Node SDK Epic | — | ⏳ Not yet defined |
| Agent Bootstrap + Consent Epic | — | ⏳ Future |
| ADR-CORE-001 | — | ⏳ Future |

---

## Resolved Decisions

All architectural questions from the roadmap have been resolved prior to execution. The remaining decisions are implementation choices for each epic, not architecture choices.

| # | Question | Decision | Rationale |
|---|----------|----------|-----------|
| 1 | Display name default? | Hostname, allow override | First install zero-touch; hostname not stable enough for permanent identity |
| 2 | Public key in identity? | Reserve field, defer security ADR | Identity and authentication are separate problems |
| 3 | Session identity model? | `node_id` + `agent_id` + `session_id` | node_id = where execution happens; agent_id = who requested; session_id = this context |
| 4 | Session expiration? | Lifecycle-first (CREATED→ACTIVE→CLOSED); EXPIRED as recovery only | Avoid arbitrary policy in transport layer |
| 5 | Evidence queue storage? | Existing runtime DB | Evidence queue is Node operational state; separate store = premature complexity |
| 6 | Superseded evidence? | Yes — GENERATED→SUBMITTED→ACKNOWLEDGED→ACCEPTED, or SUPERSEDED | Qualification is not always monotonic |
| 7 | Bundle llama.cpp? | Yes — managed runtime | Qualification requires reproducibility; Node must know exact runtime producing evidence |
| 8 | Windows service or user process? | User process first; service later | Better GPU compatibility; simpler development |
| 9 | SDK language? | Rust first; Python FFI bindings later | Core contracts are Rust; safety-critical boundary originates in Rust |
| 10 | SDK repository? | Same repo initially | Contract changes need atomic updates; prevents SDK drift |
| 11 | Core operational model? | ADR-CORE-001 (deferred) | Purpose of ADR-CORE-001 is to decide this |
| 12 | Core database? | SQLite first | Matches local-first architecture; same technology family as Node |

### Remaining MCP Question

The MCP questions (gateway location, authentication mapping) are intentionally deferred. They depend on Core runtime activation (Phase 6) and have no bearing on Node-side implementation.

**Result:** EPIC-NODE-IDENTITY-AND-CAPABILITY-FOUNDATION can proceed without architectural ambiguity.
