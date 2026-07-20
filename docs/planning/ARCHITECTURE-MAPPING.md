# LibrarianOS Core/Node Architecture Mapping

**Status:** Evolved — Architecture established, not discovery  
**Original Date:** 2026-07-15  
**Revised Date:** 2026-07-19  
**Preceding ADR:** AIR-Q (established qualification boundaries)  
**Superseded By:** ADR-PLATFORM-001-CORE-NODE-AUTHORITY-ARCHITECTURE.md  
**Reference:** ARCHITECTURAL-BOUNDARY-MAP.md  
**Workspace:** G:\Models  

---

## 1. Purpose

This document maps the existing Librarian architecture against the Core/Node conceptual model. It was originally a **discovery and mapping artifact** — it has been updated to reflect the evolved state.

The original question was: *"What already exists across both Mac (conceptual authority) and Windows (execution reality) that maps to a distributed Core/Node architecture?"*

**The answer has been found and formalized in ADR-PLATFORM-001.**

### Evolution Summary

| Date | Status | Key Finding |
|------|--------|-------------|
| 2026-07-15 | Discovery Phase | Core and Node co-located in same crate |
| 2026-07-15 | ADR-NODE-001 Accepted | Model B selected, crate separation planned |
| 2026-07-15 | Crate Separation Complete | librarian-contracts, librarian-core, librarian-node exist |
| 2026-07-19 | Platform Architecture | ADR-PLATFORM-001 formalizes full platform |

### Critical Finding: The Discovery Inversion (Resolved)

**Before discovery, the assumption was:**
- Core is conceptual (Mac-side, unimplemented)
- Node is implemented (Windows-side, running code)
- The gap is "implement Core"

**After discovery, the truth was:**
- **Core already exists in code** — the `canonical/` module (19 submodules)
- **Node already exists in code** — the `residency/`, `process/`, `evidence/`, `db/`, `runtime_state/`, `operator/` modules
- **They are co-located in the same binary** — `rust-router` contains both Core and Node
- **The gap is not "implement Core" — the gap is enforce the boundary**

**After implementation, the truth is:**
- **Crate separation is complete** — `librarian-contracts`, `librarian-core`, `librarian-node` exist
- **Compiler enforcement is active** — Forbidden dependencies = build failure
- **The boundary is enforced** — Not just a convention, but a build-time invariant
- **The platform architecture is established** — ADR-PLATFORM-001 formalizes the full model

---

## 2. Three Architectural Models

The discovery evidence supported three possible models for enforcing Core/Node separation. **Model B has been selected and implemented.**

### Model A — Current State: Logical Separation Only (No Enforcement)

```
                 rust-router
                      |
        +-------------+-------------+
        |                           |
   canonical/                  runtime/
   (Core authority)           (Node execution)

   Enforced by: nothing
   Dependency direction: unrestricted
```

**Status:** Legacy / temporary — was the starting point, no longer acceptable

### Model B — Workspace Separation (Compile-Time Enforcement) — **TARGET**

```
librarianos/
│
├── librarian-contracts/          (shared packet types, no logic)
│   ├── QualificationRequest
│   ├── EvidencePacket
│   ├── ResidencyStatus
│   └── validation schemas
│
├── librarian-core/               (canonical authority)
│   ├── canonical-db
│   ├── qualification
│   ├── governance / ledger
│   ├── provenance
│   ├── capability policy
│   └── bridge client
│
└── librarian-node/               (execution runtime)
    ├── residency supervisor
    ├── process management
    ├── evidence collection
    ├── operational DB
    ├── hardware adapters
    └── operator surface

Dependency direction:

    librarian-contracts
          ^     ^
          |     |
   librarian-core  librarian-node

   Forbidden: core → node (compile error)
   Forbidden: node → core (compile error)
```

**Status:** Implemented and enforced — `librarian-contracts` (56 tests), `librarian-core` (580+ tests), `librarian-node` (85+ tests)

### Model C — Distributed Services (Network Enforcement)

```
Mac                              Windows
 |                                |
 | librarian-core service         | librarian-node service
 | (HTTP/gRPC/MCP server)         | (HTTP/gRPC/MCP server)
 |                                |
 |  canonical-db                  |  llama.cpp / GPU
 |  qualification engine          |  residency supervisor
 |  governance ledger             |  evidence collection
 |                                |
 +--------+  network boundary  +--+
          |                    |
          QualificationRequest
          EvidencePacket
          ResidencyStatus
```

**Status:** Future evolution — when multi-node operation is required

### 2.1 Authority Boundary vs. Transport Boundary

The most important architectural insight from discovery:

```
MCP is not the authority boundary.
```

Current reality:

```
Agent/Human
     |
     v
    MCP                          ← transport layer
     |
     v
Librarian Core                   ← authority layer
     |
     | QualificationRequest / EvidencePacket
     v
Librarian Node                   ← execution layer
```

Not:

```
Agent → MCP → Node (WRONG — MCP does not grant authority)
```

Why this matters:

- MCP exposes capabilities (tools)
- A tool being callable does not mean it has authority
- Authority comes from: Core state + Owner decision + Evidence validation
- The packet contracts (QualificationRequest, EvidencePacket) are the **real** authority boundary
- MCP is only the transport that carries those packets

This aligns with the AIR-Q principle: *Qualification is contextual, not model-based.* Similarly, authority is contextual, not transport-based.

---

## 3. Architectural Model Under Investigation

The target conceptual separation under investigation:

```
                 LibrarianOS
              Control Architecture
                     |
                    Core
                     |
        ------------------------------
        |                            |
     MCP Boundary              Authority Layer
        |
        |
   -----------------
   |               |
 Nodes          Clients
   |
 Runtime / Tools / Models
```

### 2.1 Key Questions This Discovery Addresses

1. **Librarian Core** — What currently acts as Core authority?
2. **Runtime Nodes** — What existing components are node-like?
3. **MCP Role** — How is MCP (or equivalent) used as transport/contract boundary?
4. **State Synchronization** — How does state move between environments?
5. **Offline/Reconciliation** — Does the architecture support disconnected operation?
6. **Model Location Independence** — Can qualification be environment-independent?

---

## 3. Discovery Method

This mapping is based on inspection of the workspace at `G:\Models` and referenced external projects:

| Source | Location | Content |
|--------|----------|---------|
| Sprint plan | `G:\Models\LOCAL-MODEL-ORCHESTRATION-SPRINT-PLAN.md` | Architecture decisions, gap analysis, sprint chain |
| Custody ledger spec | `win-custody-ledger\docs\governance\custody-ledger-spec.md` | Node role contract, authority separation |
| Intake boundary | `win-custody-ledger\docs\governance\result-artifact-intake-boundary.md` | Evidence handoff envelope, certifications |
| Evidence packet | `win-custody-ledger\docs\governance\*` | Export pipeline, manifest, bundle, transfer receipt |
| Pipeline orchestration | `win-custody-ledger\docs\governance\mac-inspection-export-pipeline-orchestration.md` | Cross-node evidence flow |
| Custody ledger lib | `win-custody-ledger\lib\*.ps1` | 11 PowerShell modules |
| Custody ledger schemas | `win-custody-ledger\schema\*.json` | 10 JSON schemas |
| Custody ledger tests | `win-custody-ledger\tests\*.ps1` | 14 test suites, 376+ tests |
| Hardware | Machine identity DESKTOP-ISNJ51B, RX 570, 4GB VRAM |
| Model files | `minicpm5\` — MiniCPM5 Q4/Q8 GGUF |

**External (referenced but not in this workspace):**
| Component | Path | Content |
|-----------|------|---------|
| Rust router crate | `G:\openwork\librarian-runtime-node\rust-router\` | Axum HTTP server, process supervision |
| Runtime binaries | `G:\openwork\librarian-runtime-node\runtime\llama.cpp\` | llama-server.exe |
| PowerShell ops | `G:\openwork\librarian-runtime-node\scripts\` | ~25 test scripts, operations |
| Config | `G:\openwork\librarian-runtime-node\config\` | Model profiles, hardware measurement |

---

## 4. Current Architecture Map

### 4.1 Deployed Environment Topology

```
                    Mac Side (Conceptual/Planning)
                    ==============================
                    [Librarian Authority]
                         |
                    Canonical State:
                      - Sprint definitions
                      - Packet plans
                      - Context items
                      - Provenance
                      - Owner decisions
                      - Validation state
                      - Model capability policy
                         |
                    (Conceptual boundary — not yet implemented)
                         |
                    ======== Evidence Handoff ========
                         |
                         v
                    Windows Side (Implemented)
                    ==========================
                    [Runtime Node] -----> [Evidence Packet] -----> [Mac Inspection]
                         |                                                ^
                    [Custody Ledger] --- advisory evidence only ---------|
                         |
                    [llama-server.exe]
                         |
                    [GPU: RX 570 / 4GB]
                         |
                    [Model files on disk]
```

### 4.2 Node Identity

| Property | Current State | Evidence |
|----------|---------------|----------|
| Machine name | DESKTOP-ISNJ51B | Sprint plan §2.5 |
| Hostname | "Big Pickle" | Sprint plan §2.5 |
| Runtime node ID | Not formalized | No node_id in runtime config |
| Hardware ID | Informal (i5-3570K, RX 570) | Sprint plan §2.5 |
| Runtime identity | Not formalized | No registry/registration |
| Model identity | Filename-based | `minicpm5\MiniCPM5-1B-Q4_K_M.gguf` |

**Finding:** The Windows machine has a de facto identity but no formal node identity, registration, or discoverable runtime identity.

### 4.3 Core Capabilities (Conceptual — Mac Side)

| Capability | Exists? | Location | Evidence | Limitation |
|------------|---------|----------|----------|------------|
| Project state | **Conceptual only** | Mac side (not in this workspace) | Referenced in sprint plan §1 | No implementation |
| Planning context | **Conceptual only** | Mac side | Sprint plan §1 | No implementation |
| Governance rules | **Partially** | win-custody-ledger | custody-ledger-spec.md | Windows-local rules only; no canonical rule store |
| Owner decisions | **Conceptual only** | Mac side | Sprint plan §6, Decision #4 | Referenced as authority domain |
| Receipts | **Windows only** | win-custody-ledger | Evidence packet, intake boundary | Advisory-only; no canonical receipt store |
| Evidence history | **Windows only** | win-custody-ledger | Ledger chain, lifecycle evidence | Local-only; not aggregated |
| Artifact authority | **No** | — | — | Windows explicitly disclaims canonical authority |
| Reconciliation | **No** | — | — | Not implemented |

### 4.4 Node Capabilities (Implemented — Windows Side)

| Capability | Exists? | Location | Evidence |
|------------|---------|----------|----------|
| Model execution | **Yes** | Rust router → llama-server.exe | Sprint plan §2.1, `process.rs` |
| GPU residency | **Partial** | BackendState (Stopped/Starting/Healthy/Degraded/Failed) | Sprint plan §2.1 |
| Process supervision | **Yes** | `process.rs` — BackendProcess | Sprint plan §2.1 |
| Health endpoints | **Yes** | HTTP: `/backend/status`, `/backend/health` | Sprint plan §2.1 |
| Model profiles | **Yes** | `config/model-profiles.json` | Sprint plan §2.2 |
| Hardware profiles | **Yes** | `config/measured_hardware_profiles.json` | Sprint plan §2.2 |
| Runtime state | **Partial** | BackendState enum | Sprint plan §3, G3 |
| Evidence recording | **Yes** | `evidence.rs` — EvidenceWriter | Sprint plan §2.1 |
| Custody ledger | **Yes** | win-custody-ledger | 376+ passing tests |
| Evidence export | **Yes** | Evidence packet → intake boundary → manifest index → export bundle | Pipeline orchestration |
| Transfer receipt | **Yes** | Transfer-attempt receipt | LOCAL-ONLY advisory |
| Authority boundary | **Yes** | 24+ forbidden fields enforced | Schema-level, module-level |

### 4.5 MCP Role — Current Status

| Aspect | Current State | Evidence |
|--------|---------------|----------|
| MCP servers | **Draft templates only** | `mcp/templates/mcp-server.example.json` — macOS/Windows examples |
| MCP bridge scripts | **Draft** | `scripts/mcp-bridge.ps1` — stdio→HTTP bridge |
| MCP clients | **None in rust-router** | Bridge uses raw HTTP, not MCP protocol |
| Tools exposed | HTTP REST API (axum) | Not MCP protocol |
| Transport | HTTP (localhost:9120-9124) | Custom, not MCP |
| Authentication | **None** | Localhost-only |
| Permissions | Implicit (localhost access) | No auth layer |

**Finding:** The current architecture **does not use MCP** for the Core/Node boundary. All internal communication is via:
- HTTP/REST (local model server, evidence bridge)
- Exported evidence files (cross-machine)
- Advisory evidence packets (file-based handoff)

MCP bridge scripts and template configs exist as a **proposed future transport** for agent↔authority communication. The planning doc `WIN-MULTINODE-MCP-DOCUMENT-CUSTODY-NOTES.md` defines the MCP tool contract direction (proposal-based, not generic file write), but no MCP tools are implemented.

MCP is discussed in the custody ledger spec as **forbidden** for Windows nodes:
> *"Must not reference MCP write/apply tools (mcp_tool: 'write', mcp_tool: 'apply')"* — custody-ledger-spec.md §3.3

### 4.6 Current Distributed Execution Paths

#### Path A: Local Model Inference (within Windows node)
```
Agent/User
    |
    v
HTTP POST /v1/chat/completions
    |
    v
rust-router (axum)
    |
    v
llama-server.exe (child process)
    |
    v
Model loaded on GPU
    |
    v
Inference → Response
    |
    v
Evidence written (evidence.rs)
```

#### Path B: Evidence Export (Windows → Mac)
```
Local evidence
    |
    v
Evidence packet (advisory, LOCAL-ONLY)
    |
    v
Intake boundary record (certifies no authority claim)
    |
    v
Artifact manifest (hash-indexed)
    |
    v
Export bundle (portable directory + checksums)
    |
    v
Transfer receipt (attempt record, not delivery proof)
    |
    v
[Physical handoff to Mac]
    |
    v
Mac inspects, ingests, accepts (separate process)
```

#### Path C: Work Packet (future — Mac → Windows)
```
Mac assembles packet (sprint plan §4, Phase B)
    |                   
    v                  
Windows receives       
    |                   
Executes via supervisor 
    |                   
Returns evidence        
    v                   
Mac reconciles canonical state
```

**Finding:** Path C is defined as a bridge contract (`QualificationRequest` packet + `BridgeClient` HTTP transport) but the actual Mac→Windows dispatch API is not yet exposed as a runtime HTTP endpoint. The contract types exist; the operational pipeline does not.

---

## 5. Existing Core Capabilities — Detailed Map

### 5.1 What Currently Acts as Core Authority

**CRITICAL UPDATE (2026-07-15):** The initial mapping underestimated the Windows side. The `canonical/` module at `rust-router/src/canonical/` is a **fully implemented Mac-side canonical authority** with 19 submodules, co-located in the Windows crate.

| Domain | Current Implementation | Core Gap |
|--------|----------------------|----------|
| Sprint definitions | `canonical/ledger/` — Sprint ledger with authorization/receipt models | No Owner decision ingestion |
| Packet plans | `canonical/packets/` — QualificationRequest packet type defined | No packet assembly/dispatch UI |
| Context items | `canonical/qualification/` — Task packs, fixture references | No context management UI |
| Governance rules | `canonical/ledger/validation.rs` — Transition validation | Rules co-located with Windows |
| Owner decisions | `canonical/capability/decisions.rs` — Decision models | No owner decision table in DB |
| Receipts | `canonical/release/` — Trust packages, provenance, manifest | No canonical receipt aggregation |
| Evidence history | `canonical/packets/evidence_packet.rs` — EvidencePacket with lifecycle chain | No aggregated evidence view |
| Artifact authority | `canonical/packets/` — assert_no_capability_data() enforcement | No artifact acceptance workflow |
| Model identity | `canonical/db.rs` — model_identity_record with qualification scope | No identity enrollment UI |
| Qualification | `canonical/qualification/` — Runner, stages, validator engine, batch, custom executor | No owner decision integration |
| Capability evidence | `canonical/capability_evidence/` — 13 modules, benchmark adapters | No capability manifest generation |
| Comparative analysis | `canonical/comparative/` — Analyzer, audit, finding, roster | No automated threshold |

### 5.2 Authority Separation Model

The custody ledger establishes a three-role model:

```
Windows Node:  governed-worker-node     | canonical_authority = false
Mac/Librarian: canonical-authority      | canonical_authority = true
Owner:         decision-authority       | (separate governed process)
```

**Contract enforcement:**
- Module-level (`Add-LedgerEntry`): rejects forbidden patterns before entry
- Validator-level (`Test-ForSplitBrain`, `Test-ForStaleAuthority`, `Test-MacLedgerMutation`): post-hoc detection
- Schema-level: 24+ forbidden root-level authority fields

---

## 6. Existing Node Capabilities — Detailed Map

### 6.1 Windows Runtime Node Identity

**Current:** No formal node identity exists beyond the machine hostname.

What exists:
- `node_id` schema field in custody ledger (`win-custody-*` pattern)
- Machine hostname: DESKTOP-ISNJ51B
- Nickname: "Big Pickle"

What is missing:
- No node ID persistence or registration
- No runtime identity service
- No hardware identity beyond manual config
- No model identity beyond filenames (no SHA-256 manifest for model files)
- No node capability advertisement

### 6.2 Runtime Management

| Function | Status | Module |
|----------|--------|--------|
| llama.cpp integration | **Complete** | llama-server.exe + Rust process supervision |
| Model loading | **Implemented** | `POST /backend/select` → spawn llama-server |
| Model unloading | **Implemented** | `POST /backend/stop` → terminate process |
| Residency tracking | **Partial** | BackendState enum (no formal state machine) |
| VRAM monitoring | **Planned** (Sprint 2) | Not yet implemented |
| Process tracking | **Implemented** | BackendProcess (PID, health polling) |
| State reporting | **Implemented** | `GET /backend/status`, `/backend/health` |
| Health polling | **Implemented** | `server.rs` periodic health check |

### 6.3 Qualification/Evidence Capabilities

| Capability | Status | Details |
|------------|--------|---------|
| Qualification harnesses | **Partial** | Script-based (`scripts/test-*.ps1`) |
| Evidence exporters | **Complete** | EvidenceWriter, Evidence Packet Exporter |
| Receipts | **Complete** | Custody ledger entries, transfer receipts |
| Runtime measurements | **Planned** (Sprint 2) | VRAM, tokens/sec |
| Validation outputs | **Partial** | Test scripts, validation logs |

### 6.4 What Is Not Yet a Node

| Component | Current Role | Missing for Node Classification |
|-----------|-------------|--------------------------------|
| Agent Bridge | Not implemented | No bridge exists yet |
| Workbench | Not present | No workbench in this workspace |
| OpenWork app | Not in this workspace | External agent host |
| Python router (legacy) | Being replaced | Deprecated |
| Other machines | No other nodes exist | Single machine only |

---

## 7. MCP Usage — Current Map

### 7.1 What MCP Is Today

The custody ledger spec references MCP only as a **forbidden mechanism**:

> *"Must not reference MCP write/apply tools (mcp_tool: 'write', mcp_tool: 'apply')"* — custody-ledger-spec.md §3.3

The sprint plan does not mention MCP at all.

### 7.2 What MCP Is Not Today

- Not used as Core/Node boundary
- Not used for authority transfer
- Not used for evidence transport
- Not used for node discovery
- Not used for capability advertisement

### 7.3 Conceptual MCP Role (from user's framing)

The user's architectural model defines MCP as:

| Role | Is | Is Not |
|------|----|--------|
| MCP | transport layer | authority |
| MCP | capability contract | governance |
| MCP | communication boundary | evidence store |
| MCP | — | project truth |

---

## 8. State Synchronization Model

### 8.1 Current Capabilities

| Mechanism | Exists? | Notes |
|-----------|---------|-------|
| Shared files | **Partial** | Export bundles are file-based |
| Artifact movement | **Partial** | Export bundle → physical handoff |
| Project DB sync | **No** | No DB-to-DB sync |
| Evidence sync | **Partial** | One-way (Windows → Mac via exports) |
| Conflict detection | **No** | No mechanism |
| Version reconciliation | **No** | No mechanism |

### 8.2 Current Flow

```
Windows generates evidence
    |
    v
Packages as advisory evidence packet
    |
    v
Creates intake boundary (certifies no authority claim)
    |
    v
Creates export bundle (portable directory)
    |
    v
[Physical transfer — USB, email, network share]
    |
    v
Mac inspects
    |
    v
Mac accepts (separate process, not implemented)
```

### 8.3 Gaps

1. **No sync protocol** — Evidence moves via files, not a protocol
2. **No node identity** — Nodes cannot discover each other
3. **No reconciliation** — No mechanism to reconcile divergence
4. **No conflict detection** — Cannot detect or resolve conflicts
5. **No bidirectional sync** — Windows→Mac only; no Mac→Windows state push
6. **No packet interface** — Mac cannot send work packets programmatically

---

## 9. Offline / Conditional Operation

### 9.1 Connected Mode (Conceptual)

```
Node → Core → Authority
     ↑           |
     └── evidence ┘
```

Not yet implemented. Requires packet interface and evidence ingestion.

### 9.2 Disconnected Mode (Partially Implemented)

```
Node
 |
 +-- cached rules (config files only)
 +-- local execution (complete — llama-server.exe)
 +-- pending evidence (complete — custody ledger)
 +-- local state (partial — no formal offline mode)
```

The Windows node can operate independently today:
- Model loading and inference work offline
- Custody ledger records locally
- Evidence is queued in the ledger for later export
- No formal "offline mode" flag

### 9.3 Reconnection Mode (Not Implemented)

```
Node
 |
 v
Reconciliation → Trusted State Restored
```

**Missing:**
- No reconnection protocol
- No reconciliation mechanism
- No conflict resolution
- No version vector or causality tracking
- No "last synced" checkpoint

---

## 10. Model Location Independence

### 10.1 Current Qualification Environment

The current environment is **tightly coupled**:

```
Specified by:
    Model filename       (minicpm5\MiniCPM5-1B-Q4_K_M.gguf)
    Architecture         (MiniCPM architecture)
    Quantization         (Q4_K_M)
    Runtime binary       (llama-server.exe, build-specific)
    GPU hardware         (RX 570, 4GB VRAM)
    Driver               (Vulkan driver version)
    OS                   (Windows 10 Pro Workstations)
    RAM                  (16GB+)
```

### 10.2 Not "Model Running on Librarian Hardware"

The current qualification is device-specific, not location-independent. AIR-Q's concept of "qualification portability" has not been implemented.

### 10.3 What Would Be Required for Location Independence

```
Qualification =
    Model
    + Runtime (versioned, hashed)
    + Node (identified, measured)
    + Context (workload, parameters)
    + Tools (versioned)
    + Governance Boundary (rules, policies)
```

---

## 11. Missing Abstractions — Gap Summary

| Abstraction | Status | Priority | Notes |
|-------------|--------|----------|-------|
| Node identity | **Missing** | High | No node ID, registration, or discovery |
| Node capability advertisement | **Missing** | High | Cannot discover what a node offers |
| Core authority service | **Missing** | High | Mac side not implemented |
| Packet protocol | **Missing** | High | No formal work packet interface |
| Evidence ingestion | **Missing** | High | Mac has no evidence receiver |
| Sync protocol | **Missing** | Medium | File-based handoff only |
| Node reconciliation | **Missing** | Medium | No reconnection mechanism |
| Offline mode | **Missing** | Medium | Node works offline but not explicitly |
| MCP boundary | **Missing** | Medium | Not used for Core/Node communication |
| Agent Bridge | **Missing** | Medium | Referenced but not implemented |
| Workbench | **Missing** | Low | Not present |
| Qualification portability | **Missing** | Low | Tied to specific hardware |
| VRAM monitoring | **Planned** | Sprint 2 | Scheduled |
| Residency state machine | **Planned** | Sprint 3 | Scheduled |
| Model switching | **Planned** | Sprint 3 | Scheduled |

---

## 12. Classification Table

| Component | Current Role | Core | Node | Notes |
|-----------|-------------|------|------|-------|
| `canonical/` module (19 submodules) | Mac-side canonical authority | **Yes** | No | Co-located in Windows crate |
| `canonical/packets/` | Bridge contracts | **Yes** | No | QualificationRequest, EvidencePacket, ResidencyStatus |
| `canonical/bridge/` | Mac→Windows HTTP client | **Yes** | No | BridgeClient for evidence retrieval |
| `canonical/db.rs` | Canonical DB (identity, task/validator packs) | **Yes** | No | Full CRUD + 22 tests |
| `canonical/ledger/` | Sprint governance ledger | **Yes** | No | Authorization, receipts, validation |
| `canonical/qualification/` | Qualification engine | **Yes** | No | Runner, stages, validator engine, batch |
| `canonical/capability_evidence/` | Capability benchmark runners | **Yes** | No | 13 modules, 5+ benchmark adapters |
| `canonical/release/` | Trust packages + provenance | **Yes** | No | Release management |
| `canonical/comparative/` | Comparative analysis | **Yes** | No | Analyzer, audit, finding, roster |
| `canonical/routing/` | Canonical routing | **Yes** | No | Projections, execution profiles |
| `canonical/review/` | Review/builder | **Yes** | No | Review construction |
| `canonical/provenance/` | Model provenance | **Yes** | No | Builder + models |
| `canonical/registry/` | Registry store | **Yes** | No | Registry operations |
| `canonical/capability/` | Capability manifest + decisions | **Yes** | No | Manifest builder |
| `residency/` module | GPU residency supervisor | No | **Yes** | 8-state state machine |
| `process.rs` | Child process lifecycle | No | **Yes** | BackendProcess |
| `server.rs` | HTTP router + endpoints | No | **Yes** | Axum server |
| `evidence/` module | Evidence recording | No | **Yes** | EvidenceWriter, residency status |
| `db/` module | Windows operational DB | No | **Yes** | 6 runtime tables |
| `runtime_state/` | Lease/run lifecycle | No | **Yes** | ModelLease, RuntimeRun |
| `operator/` module | Dashboard surface | No | **Yes** | Advisory UI models |
| llama-server.exe | Model inference binary | No | **Yes** | — |
| win-custody-ledger (PowerShell) | Custody governance | No | **Yes** | 376+ tests |
| Python router (legacy) | Being replaced | No | No | Deprecated |
| mcp-bridge.ps1 | stdio bridge | — | — | Draft |
| OpenWork | Agent host | — | — | External |

---

### 12.1 Critical Finding: Monolith Architecture

The `rust-router` crate is a **monolith** containing both Core and Node logic:

```
rust-router binary
    │
    ├── canonical/  (Core — Mac-side authority)
    │   ├── packets/        — QualificationRequest, EvidencePacket, ResidencyStatus
    │   ├── bridge/         — Mac→Windows HTTP client
    │   ├── db.rs           — Canonical DB (model_identity, task_pack, validator_pack)
    │   ├── ledger/         — Sprint governance
    │   ├── qualification/  — Runner, stages, validator engine
    │   ├── capability_evidence/ — Benchmark adapters
    │   ├── release/        — Trust packages, provenance
    │   ├── comparative/    — Comparative analysis
    │   ├── routing/        — Canonical routing
    │   ├── provenance/     — Model provenance
    │   └── capability/     — Manifest, decisions
    │
    └── (Node — Windows runtime)
        ├── residency/      — 8-state state machine
        ├── process.rs      — BackendProcess lifecycle
        ├── server.rs       — HTTP endpoints
        ├── evidence/       — Evidence recording + export
        ├── db/             — Windows operational DB (6 tables)
        ├── runtime_state/  — Lease, run, lifecycle models
        └── operator/       — Dashboard surface

Enforced by:     Nothing (same binary)
Detectable by:   Compile-time dependency analysis
Recommended:     Extract canonical/ into separate crate
```

**This means there is currently no compile-time enforcement of the Core→Node authority direction.** The bridge client and packet contracts define the boundary at the type level, but a bug in `canonical/` could accidentally depend on `process.rs` or `residency/` without any compiler error.

---

## 13. Answers to Discovery Questions

### 13.1 What is Librarian Core Today?

**Librarian Core is implemented as the `canonical/` module in the `rust-router` crate.** This was a critical discovery during the Windows node inventory.

The `canonical/` module contains:
- **CanonicalDatabase** — Full SQLite-backed record management for model identity, system profiles, task packs, and validator packs
- **Packet contracts** — Sealed `QualificationRequest` (Mac→Windows) and `EvidencePacket` (Windows→Mac) with versioning, SHA-256 hashing, and `assert_no_capability_data()` enforcement
- **Bridge client** — HTTP client for Mac→Windows evidence retrieval (`get_evidence_run`, `get_evidence_lifecycle`, `get_residency_status`)
- **Qualification engine** — Runner, stages (Smoke, PrimitiveProbes), validator engine, batch execution, custom executor
- **Capability evidence framework** — 13 modules with benchmark adapters (lm_eval, code_needle, adversarial fixtures)
- **Release management** — Trust packages, provenance builder, manifest generator
- **Sprint ledger** — Governance receipts, authorization, state transitions
- **Comparative analysis** — Analyzer, audit, finding, roster
- **Provenance** — Model provenance builder
- **Capability manifest** — Model capability declarations and decisions

**Key architectural finding:** Core logic is **co-located in the same crate** as Node logic. The `rust-router` binary contains both `canonical/` (Core) and `residency/`, `process/`, `server/`, `db/`, `evidence/` (Node). There is no physical separation between Core and Node at the build level.

### 13.2 What is a Librarian Node Today?

A Librarian Node is the **Windows Runtime Node** (`win-custody-*`), which is:
- A single machine (DESKTOP-ISNJ51B / "Big Pickle")
- Running the `rust-router` binary (axum HTTP server + llama.cpp supervision)
- Running the `residency` supervisor (8-state machine: Unloaded→Loading→Ready→Running→Draining→Unloading→VerifyingRelease→Unloaded→Failed)
- Running a PowerShell custody ledger with 376+ passing tests
- Running ~50 operational scripts for model management, qualification, and harness
- Capable of local model execution, evidence recording, evidence export, and startup reconciliation
- Explicitly prohibited from claiming canonical authority
- Lacking formal node identity, registration, or state reporting

### 13.3 What is Not Yet a Node?

- **Mac/Librarian**: Not a node yet — conceptual authority only
- **Agent Bridge**: Does not exist
- **Workbench**: Does not exist
- **Client devices**: No concept of "clients" exists
- **Other machines**: Single-node deployment only
- **MCP gateway**: No MCP infrastructure exists

### 13.4 What Authority Remains Centralized?

All canonical authority remains centralized:
- Sprint definition
- Packet planning
- Owner decisions
- Seal and acceptance
- Canonical evidence history
- Project truth

The Windows node is explicitly a **governed worker** with zero canonical authority.

### 13.5 What Evidence Flows Between Nodes?

Evidence flows **one direction only**: Windows → Mac

The evidence packet contains:
- Custody ledger entries (chain-of-custody)
- Intake boundary record (authority certifications)
- Artifact manifest (hash-indexed file references)
- Export bundle (portable directory + checksums)
- Transfer receipt (attempt record)

No evidence flows Mac → Windows (no packet dispatch implemented).

### 13.6 What Decisions Require Core Availability?

Based on governance documents and architecture:

| Decision | Requires Core? | Current Handling |
|----------|---------------|-----------------|
| Model execution | **No** | Windows executes locally |
| Model selection | **No** | Local profile-based routing |
| Evidence recording | **No** | Windows-ledger local |
| Evidence export | **No** | Windows creates advisory packet |
| Work acceptance | **Yes** | Windows cannot accept work |
| Sprint definition | **Yes** | Core-only |
| Packet planning | **Yes** | Core-only |
| Seal/approval | **Yes** | Core-only |
| Canonical receipt | **Yes** | Core-only |
| Owner decision | **Yes** | Owner-only |
| Governance update | **Yes** | Core-only (currently Windows-local rules) |
| Model capability policy | **Yes** | Core-only |

---

## 16. Recommended ADR Scope

**The ADR question has been refined by discovery and resolved by implementation.**

The original framing was: *"What architecture allows AIR-Q-qualified systems to operate across distributed execution environments while preserving Librarian authority?"*

The refined framing after discovery was: **"Does LibrarianOS enforce Core/Node authority separation through architectural boundaries, or only through contracts and conventions?"**

**The answer is: Yes, through compile-time enforcement (Model B).**

### 16.1 Decisions Captured

| # | Decision | Status | Evidence |
|---|----------|--------|----------|
| 1 | Core/Node as architectural invariant | **Formalized** | ADR-PLATFORM-001 |
| 2 | Boundary enforcement | **Implemented** | Crate graph (Model B) |
| 3 | Packet contracts as authority boundary | **Implemented** | librarian-contracts |
| 4 | Core → Node dependency direction | **Enforced** | Compile-time (0 forbidden imports) |
| 5 | MCP role | **Defined** | Agent→Core transport, not Core→Node |
| 6 | Node identity | **Implemented** | Registration, trust state, capability advertisement |
| 7 | Offline behavior | **Formalized** | Pending evidence queue + reconciliation |
| 8 | Deployment model | **Implemented** | Crate separation; network separation deferred |

### 16.2 Dependency-Direction Audit (Completed)

**Objective:** Determine whether the current Rust implementation already respects Core/Node separation logically, and identify extraction risks.

**Results:**
1. **Dependency direction** — Map completed: 0 forbidden imports
2. **Authority leakage** — None found: Core→Node: 0, Node→Core: 0
3. **Extraction feasibility** — Purely mechanical, no behavioral code changes required
4. **Final classification** — Extraction complete; all three crates exist

### 16.3 In Scope for ADR-PLATFORM-001 (Supersedes ADR-NODE-001)

1. **Platform architecture** — Core / Node / MCP / Shared Contracts
2. **Installability layer** — First-class architectural concern
3. **Boundary enforcement model** — Model B implemented
4. **Packet contract layer** — librarian-contracts exists
5. **Authority direction** — Core commands, Node executes, Node reports
6. **Node identity model** — Implemented: node ID, registration, capability advertisement
7. **Node lifecycle** — Implemented: Registration → Connection → Authorized → Executing → Evidence → Reconnecting
8. **Offline behavior** — Formalized: pending evidence queue + reconciliation
9. **MCP role** — Defined: Agent ↔ Core transport only

### 16.4 Out of Scope for ADR-PLATFORM-001

1. Implementation details (schema, API design, migration scripts)
2. Security/cryptography design (separate ADR)
3. UI/Workbench architecture (separate ADR)
4. Multi-node operational deployment (future)
5. Distributed platform (future)

---

## 15. Evidence Files Examined

| File | Relevance |
|------|-----------|
| `G:\Models\LOCAL-MODEL-ORCHESTRATION-SPRINT-PLAN.md` | Primary architecture reference |
| `G:\Models\win-custody-ledger\docs\governance\custody-ledger-spec.md` | Node role contract |
| `G:\Models\win-custody-ledger\docs\governance\result-artifact-intake-boundary.md` | Evidence handoff protocol |
| `G:\Models\win-custody-ledger\docs\governance\mac-inspection-export-pipeline-orchestration.md` | Cross-node evidence flow |
| `G:\Models\win-custody-ledger\docs\governance\mac-inspection-export-transfer-receipt.md` | Transfer attempt record |
| `G:\Models\win-custody-ledger\docs\governance\result-artifact-manager.md` | Artifact lifecycle |
| `G:\Models\win-custody-ledger\docs\planning\WIN-WORK-PACKAGE-RESULT-ARTIFACTS-1.md` | Result artifact planning |
| `G:\Models\win-custody-ledger\docs\planning\WIN-EVIDENCE-PACKET-EXPORT-1.md` | Evidence packet design |
| `G:\Models\win-custody-ledger\docs\planning\WIN-MAC-INSPECTION-EXPORT-PACKAGING-1.md` | Export bundle format |
| `G:\Models\win-custody-ledger\docs\planning\WIN-HARNESS-CUSTODY-LEDGER-1.md` | Harness foundation |
| `G:\Models\.opencode\openwork.json` | Workspace configuration |

---

## 16. Completion Status

| Question | Answer Status |
|----------|---------------|
| What is Librarian Core today? | **Answered and implemented** — `librarian-core` crate with 580+ tests, full CanonicalDatabase, qualification engine, capability evidence framework, sprint ledger. Separated from Node logic. |
| What is a Librarian Node today? | **Answered and implemented** — `librarian-node` crate with 85+ tests, 8-state residency supervisor, full process lifecycle, evidence pipeline, custody ledger (376+ tests), ~50 ops scripts. Formal node identity implemented. |
| What is not yet a Node? | Mac side (Core authority activation pending), Owner/decision UI, agent workstations |
| What authority remains centralized? | All canonical authority — implemented in `librarian-core` with compiler-enforced separation |
| What evidence flows between nodes? | **Answered** — Bidirectional sealed packets: QualificationRequest (Core→Node) + EvidencePacket (Node→Core) + ResidencyStatus query/response. All via HTTP/REST, not MCP. |
| What decisions require Core availability? | **Answered** — Owner decisions, capability classification, router projection, sprint sealing. Node is fully autonomous for execution, evidence recording, and evidence export. |

**Status:** Architecture established, crate separation complete, platform architecture formalized in ADR-PLATFORM-001.

**Next:** Begin platform installability and Core authority activation.
