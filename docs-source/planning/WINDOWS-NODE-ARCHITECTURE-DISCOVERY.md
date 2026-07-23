# Windows Node Architecture Discovery

**Status:** Implementation discovery — completed  
**Date:** 2026-07-15  
**Root:** `G:\openwork\librarian-runtime-node\`  
**Purpose:** Answer "What already exists on this node that maps to LibrarianOS, AIR-Q, MQR, MCP, and distributed execution?"

---

## 1. Workspace Inventory

### 1.1 Primary Repository

| Path | Purpose | Current State | Relevance |
|------|---------|---------------|-----------|
| `rust-router/` | Rust crate — HTTP server + process supervision + canonical DB | **Complete** | Core runtime node |
| `rust-router/src/canonical/` | Mac-side canonical authority implementation | **Complete** | Canonical authority logic |
| `rust-router/src/residency/` | 8-state GPU residency supervisor | **Complete** | Sprint 3 deliverable |
| `rust-router/src/operator/` | Operator dashboard models and event store | **Complete** | UI surface |
| `rust-router/src/evidence/` | Evidence export and residency status | **Complete** | Evidence pipeline |
| `rust-router/src/db/` | Windows operational DB (6 tables) | **Complete** | Runtime DB |
| `rust-router/src/runtime_state/` | Lease, run, lifecycle evidence models | **Complete** | State persistence |
| `router/` | Legacy Python router | **Being replaced** | Deprecated |
| `runtime/llama.cpp/` | llama-server.exe binary | **Deployed** | Model execution |
| `scripts/` | PowerShell operational scripts (~50 scripts) | **Complete** | Ops, tests, harness |
| `config/` | Model profiles, hardware measurements, runtime config | **Live** | Configuration |
| `mcp/` | MCP template configurations | **Draft** | MCP bridge setup |
| `docs/` | Architecture, design, planning, sprint docs | **Comprehensive** | 23 planning docs |
| `receipts/` | Action/sprint receipts | **Active** | Evidence trail |
| `reports/` | Sprint closeout reports | **Complete** | Sprint documentation |

### 1.2 External Projects

| Path | Purpose | Current State | Relevance |
|------|---------|---------------|-----------|
| `G:\Models\win-custody-ledger\` | PowerShell custody governance | **Complete** | 376+ tests |
| `G:\Models\minicpm5\` | Model files (MiniCPM5 Q4/Q8) | **Downloaded** | Test models |
| `G:\Models\blobs\` | SHA-256 hashed evidence blobs | **Active** | Integrity storage |

---

## 2. Windows Runtime Node Analysis — Complete

### 2.1 Identity

| Aspect | Status | Location |
|--------|--------|----------|
| **Node ID** | **Not formalized** — no persistent node identity | Schema field `node_id` pattern `win-custody-*` exists in win-custody-ledger |
| **Machine identity** | **Informal** — hostname DESKTOP-ISNJ51B, nickname "Big Pickle" | Sprint plan §2.5 |
| **Runtime identity** | **Not formalized** — `authority: advisory_only` logged at startup | `main.rs` line 158 |
| **Hardware identity** | **Manual** — hardware profile JSON files | `config/measured_hardware_profiles.json` |
| **Model identity** | **Partial** — filename-based, SHA-256 not stored | `local_models` table planned in DB schema, SHA-256 not yet populated |

**Finding:** The node has de facto identity but no formal node registration, discovery, or persistent runtime identity.

### 2.2 Runtime Management

| Capability | Status | Implementation |
|------------|--------|----------------|
| llama.cpp integration | **Complete** | `process.rs` — BackendProcess, child process lifecycle |
| Model loading | **Complete** | HTTP `POST /backend/select` → spawn llama-server |
| Model unloading | **Complete** | HTTP `POST /backend/stop` → BackendProcess::stop() |
| Residency management | **Complete** | `residency/supervisor.rs` — 8-state state machine |
| VRAM monitoring | **Complete** | Baseline + tolerance verification in release verification |
| Process tracking | **Complete** | BackendState: Stopped, Starting, Healthy, Degraded, Failed |
| Health endpoint | **Complete** | `GET /backend/health`, `GET /backend/status` |
| Graceful shutdown | **Complete** | `shutdown_signal()` — Ctrl+C handler, backend cleanup |

**Residency State Machine (8 states):**
```
Unloaded → Loading → Ready → Running → Draining → Unloading → VerifyingRelease → Unloaded
                                                                         ↓
                                                                    Failed (any transition)
```

### 2.3 Qualification Evidence

| Capability | Status | Implementation |
|------------|--------|----------------|
| EvidenceWriter | **Complete** | `evidence/mod.rs` — writes runtime evidence JSON |
| Lifecycle evidence | **Complete** | `runtime_state/lifecycle_evidence.rs` — 15 event types |
| Evidence export | **Complete** | `evidence/export.rs` — evidence packet export |
| Evidence packet | **Complete** | `canonical/packets/evidence_packet.rs` — sealed packet type |
| Residency status query | **Complete** | `evidence/residency_status.rs` — snapshot endpoint |
| Evidence bridge (HTTP) | **Complete** | `canonical/bridge/client.rs` — Mac-side HTTP client |
| Runtime qualification scripts | **Complete** | ~25 test scripts under `scripts/` |

**Answer:** Yes — the Windows node can currently prove "This model ran successfully in this environment under these conditions" through:
1. EvidenceWriter logging lifecycle events
2. Job lease + runtime run records in DB
3. EvidencePacket export with SHA-256 hashes
4. Release verification (PID exit + GPU memory proof)

---

## 3. MQR Implementation — Complete

### 3.1 Existing Canonical Database Tables

| Table | Status | Type |
|-------|--------|------|
| `model_identity_record` | **Complete** | Identity + qualification scope + roles |
| `system_profile` | **Complete** | Hardware system description |
| `task_pack` | **Complete** | Qualification task fixtures |
| `validator_pack` | **Complete** | Qualification validation rules |
| `schema_migrations` | **Complete** | Version tracking |

### 3.2 Planned But Not Yet Implemented

| Table | Planned Sprint |
|-------|---------------|
| `qualification_request` | Future |
| `qualification_run` | Future |
| `qualification_stage_log` | Future |
| `capability_manifest` | Future |
| `owner_decision` | Future |
| `execution_profile` | Future |
| `router_projection` | Future |
| `routing_log` | Future |

(See `canonical/db.rs` test `test_f1_15_no_qualification_tables_yet`)

### 3.3 Schemas, APIs, Adapters

| Component | Status | Location |
|-----------|--------|----------|
| QualificationRequest packet | **Complete** | `canonical/packets/qualification_request.rs` |
| EvidencePacket packet | **Complete** | `canonical/packets/evidence_packet.rs` |
| ResidencyStatus packet | **Complete** | `canonical/packets/residency_status.rs` |
| Common packet types | **Complete** | `canonical/packets/common.rs` |
| Bridge client (Mac→Windows HTTP) | **Complete** | `canonical/bridge/client.rs` |
| Qualification runner | **Complete** | `canonical/qualification/runner.rs` |
| Qualification stages | **Complete** | Smoke + PrimitiveProbes |
| Validation engine | **Complete** | `canonical/qualification/validator_engine.rs` |
| Batch qualification | **Complete** | `canonical/qualification/batch.rs` |
| Custom executor | **Complete** | `canonical/qualification/custom_executor.rs` |
| Capability evidence runners | **Complete** | `canonical/capability_evidence/` — 13 modules |
| Comparative analysis | **Complete** | `canonical/comparative/` — analyzer, audit, finding |
| Review/builder | **Complete** | `canonical/review/` |
| Release management | **Complete** | `canonical/release/` — trust packages, provenance, manifest |
| Canonical ledger | **Complete** | `canonical/ledger/` — store, models, validation |

### 3.4 Answer: Where does qualification currently happen?

**B: Model + Runtime + Hardware + Workload + Evidence + Decision**

The system is built for the AIR-Q model. The qualification pipeline is:
```
QualificationRequest (Mac→Windows)
    ↓
Model Identity Binding (model_id + sha256 + filename)
    ↓
Runtime Profile (runtime_profile_id + executable hash)
    ↓
Hardware Profile (hw_profile_id)
    ↓
Task Execution (task_description, max_tokens, temperature)
    ↓
Evidence Collection (lifecycle events, metrics, release verification)
    ↓
EvidencePacket (Windows→Mac)
    ↓
Mac Intake + Validation
    ↓
Qualification Decision (not yet implemented — planned)
```

---

## 4. Agent Bridge Analysis — Found

### 4.1 Bridge Implementation

The `canonical/bridge/` module implements a **Mac-side HTTP client** that communicates with the Windows runtime node.

**Current role:** Protocol adapter / transport gateway

**APIs (Mac side → Windows side):**

| Method | Endpoint | Purpose |
|--------|----------|---------|
| `get_evidence_run()` | `GET /evidence/runs/{run_id}` | Retrieve EvidencePacket for a run |
| `get_evidence_lifecycle()` | `GET /evidence/lifecycle?lease_id=` | Retrieve lifecycle events |
| `get_residency_status()` | `GET /residency/status?model_id=` | Query current residency state |

**Bridge client features:**
- Classified error types: Transport, Timeout, HttpStatus, Deserialization, Validation, IdentityMismatch
- Configurable timeout (default 30s)
- Packet integrity validation on response
- Identity mismatch detection (sha256 verification)

**What it is NOT:**
- Not an authority boundary (Mac already has canonical authority)
- Not a node abstraction (it's a specific HTTP client)
- Not a trust boundary (trust established through packet validation)

### 4.2 MCP Bridge Script

`scripts/mcp-bridge.ps1` — stdio bridge for OpenWork MCP connection:
- Reads JSON-RPC from stdin
- POSTs to `LIBRARIAN_MCP_URL` (default `http://127.0.0.1:3456/mcp`)
- Windows-native equivalent of macOS `mcp-bridge.sh`

**MCP template configs** exist at `mcp/templates/mcp-server.example.json` with examples for macOS (Swift) and Windows (PowerShell) bridge setups.

---

## 5. Project Context and Artifact Flow

### 5.1 Current Source of Truth

| Domain | Source of Truth | Location |
|--------|----------------|----------|
| **Model inventory** | Windows DB (`local_models` table) | `rust-router/src/db/` |
| **Runtime profiles** | Windows config + DB | `config/model-profiles.json` + DB |
| **Hardware profiles** | Windows config + DB | `config/measured_hardware_profiles.json` + DB |
| **Residency state** | Windows supervisor (in-memory + DB) | `residency/supervisor.rs` |
| **Lifecycle evidence** | Windows DB (`lifecycle_evidence` table) | `runtime_state/lifecycle_evidence.rs` |
| **Qualification requests** | Mac → Windows packet | `canonical/packets/qualification_request.rs` |
| **Qualification evidence** | Windows → Mac EvidencePacket | `canonical/packets/evidence_packet.rs` |
| **Canonical identity** | Mac DB (`model_identity_record`) | `canonical/db.rs` |
| **Task/validator packs** | Mac DB (`task_pack`, `validator_pack`) | `canonical/db.rs` |
| **Sprint ledger** | Mac ledger (`canonical/ledger/`) | `canonical/ledger/store.rs` |
| **Custody chain** | Windows custody ledger | `G:\Models\win-custody-ledger\` |
| **Planning docs** | Windows docs + sprint plans | `docs/planning/` (23 files) |

### 5.2 Current Evidence Flow

```
Windows execution
    ↓
Lifecycle evidence appended to DB
    ↓
EvidenceWriter logs runtime events
    ↓
EvidencePacket assembled (canonical::packets)
    ↓
Exported via evidence/export.rs
    ↓
Mac-side BridgeClient retrieves via HTTP GET /evidence/runs/{run_id}
    ↓
Packet validated (type, version, hashes, lifecycle ordering)
    ↓
assert_no_capability_data() called on packet
    ↓
Evidence ingested into canonical DB
    ↓
Qualification stages process evidence (runner, validator_engine)
    ↓
(Owner decision — planned, not yet implemented)
```

### 5.3 Missing Pieces

- **Owner decision ingestion** — no `owner_decision` table or UI
- **Capability manifest generation** — no capability classification
- **Router projection** — no routing decision engine
- **RAG/vector context** — not present
- **Cross-machine sync** — currently HTTP-based polling, no pub/sub

---

## 6. Current MCP Status

### 6.1 Inventory

| Component | Status | Details |
|-----------|--------|---------|
| MCP servers | **None running** | Template configs exist in `mcp/templates/` |
| MCP clients | **None in rust-router** | Bridge uses raw HTTP, not MCP protocol |
| Tools exposed via MCP | **None** | No MCP tool implementation |
| Transport mechanisms | **HTTP/REST** | All internal communication through axum HTTP server |
| Authentication | **None** | Localhost-only, no auth layer |
| MCP bridge script | **Draft** | `scripts/mcp-bridge.ps1` created |
| MCP template config | **Draft** | `mcp/templates/mcp-server.example.json` |

### 6.2 Relationship to Architecture

MCP is **not** currently used as the Core/Node boundary. The existing architecture uses:
- **HTTP/REST** for local model inference (`/v1/chat/completions`)
- **HTTP/REST** for bridge communication (`/evidence/runs/`, `/residency/status`)
- **Advisory evidence files** for cross-machine handoff
- **MCP proposed** as future transport for agent↔authority communication

The `canonical` module has explicit `assert_no_capability_data()` enforcement — MCP tools must not carry capability authority. Transport route is not authority route.

---

## 7. Core vs Node Classification

| Component | Current Role | Core | Node | Notes |
|-----------|-------------|------|------|-------|
| `rust-router` crate | Combined Core + Node | **Partial** | **Partial** | Contains both Windows runtime AND Canonical DB logic |
| `canonical/` module | Canonical authority logic | **Yes** | No | Mac-side DB, packets, bridge, ledger |
| `canonical/packets/` | Bridge contracts | **Yes** | No | QualificationRequest, EvidencePacket, ResidencyStatus |
| `canonical/bridge/` | Mac→Windows HTTP client | **Yes** | No | BridgeClient for evidence retrieval |
| `canonical/db.rs` | Canonical DB (identity, system, task/validator packs) | **Yes** | No | Mac-side data models |
| `canonical/ledger/` | Sprint ledger | **Yes** | No | Governance receipts, authorization |
| `canonical/qualification/` | Qualification engine | **Yes** | No | Runners, stages, validation |
| `canonical/capability_evidence/` | Capability benchmark runners | **Yes** | No | Benchmark adapters |
| `residency/` module | GPU residency supervisor | No | **Yes** | State machine + lease enforcement |
| `process.rs` | Process lifecycle | No | **Yes** | BackendProcess, child process mgmt |
| `server.rs` | HTTP router + endpoints | No | **Yes** | Axum HTTP server |
| `evidence/` module | Evidence recording | No | **Yes** | EvidenceWriter, residency status |
| `db/` module | Windows operational DB | No | **Yes** | 6 runtime tables |
| `runtime_state/` | Lease/run lifecycle | No | **Yes** | ModelLease, RuntimeRun |
| `operator/` module | Dashboard surface | No | **Yes** | Advisory UI models |
| `router/` (Python, legacy) | Original router | No | — | Being replaced |
| `scripts/` | Ops + tests | No | **Yes** | ~50 PowerShell scripts |
| `win-custody-ledger/` | Governance enforcement | No | **Yes** | 376+ tests |
| `mcp/` templates | MCP bridge configs | — | — | Draft / not active |

**Key insight:** The `rust-router` crate is a **monolith** containing both Core authority logic (`canonical/`) and Node execution logic (`residency/`, `process/`, `server/`, `db/`). They are not yet physically separated.

---

## 8. Offline / Reconciliation Capability

### 8.1 Disconnected Operation

| Capability | Status | Details |
|------------|--------|---------|
| Node works offline | **Yes** | Models load and infer with no network |
| Local execution | **Complete** | llama-server.exe is fully local |
| Local evidence generation | **Complete** | EvidenceWriter writes locally |
| Local ledger | **Complete** | DB persists lease/run/lifecycle records |
| Startup reconciliation | **Complete** | `residency::reconciliation::reconcile_startup()` — handles stale leases, orphan processes |

### 8.2 Reconnection

| Capability | Status | Details |
|------------|--------|---------|
| Pending evidence queue | **Partial** | Evidence is in DB, but no formal "pending sync" flag |
| Cache mechanism | **None** | No outbox pattern |
| Sync mechanism | **None** | Mac polls Windows via BridgeClient, no push |
| Conflict handling | **None** | No version vectors or causality tracking |
| Reconnection protocol | **None** | No formal reconnection handshake |

### 8.3 Startup Reconciliation (Implemented)

`canonical/connection.rs` — The supervisor's `reconcile_startup()`:
- Detects stale leases (leases with state 'active' at startup when no process exists)
- Detects orphan processes (llama-server PIDs not tracked by any lease)
- Records interrupted runs with descriptive summaries
- Returns a `ReconciliationReport` with counts

---

## 9. MCP Bridge Architecture (Draft)

The MCP bridge architecture follows from planning doc `WIN-MULTINODE-MCP-DOCUMENT-CUSTODY-NOTES.md`:

```
Agent → [MCP stdio] → mcp-bridge.ps1 → [HTTP] → Librarian Server → [HTTP] → Windows Runtime Node
                                                                                |
                                                                        evidence endpoints
                                                                        residency endpoints
```

### MCP Tool Contract (Proposed)

The planning doc proposes exposing these MCP tools (not yet implemented):
- `project_proposal_submit` — Propose a change
- `project_evidence_submit` — Return evidence
- `project_receipt_submit` — Return action receipts

Not exposed: `file_write`, `file_overwrite` on canonical paths.

---

## 10. Files Examined

| Path | Lines | Relevance |
|------|-------|-----------|
| `rust-router/src/lib.rs` | 11 | Module structure |
| `rust-router/src/main.rs` | 237 | Entry point, startup, supervisor init |
| `rust-router/src/canonical/mod.rs` | 19 | 19 submodules |
| `rust-router/src/canonical/db.rs` | 1060 | Canonical DB CRUD |
| `rust-router/src/canonical/migrations.rs` | 209 | Schema migrations |
| `rust-router/src/canonical/models/` | — | Data models (directory) |
| `rust-router/src/canonical/packets/common.rs` | 164 | Shared packet types |
| `rust-router/src/canonical/packets/evidence_packet.rs` | 423 | EvidencePacket contract |
| `rust-router/src/canonical/packets/qualification_request.rs` | 402 | QualificationRequest contract |
| `rust-router/src/canonical/packets/residency_status.rs` | 228 | Residency query/response |
| `rust-router/src/canonical/bridge/client.rs` | 413 | Mac-side HTTP bridge client |
| `rust-router/src/canonical/bridge/mod.rs` | 20 | Bridge module docs |
| `rust-router/src/canonical/connection.rs` | 82 | DB connection config |
| `rust-router/src/canonical/ledger/mod.rs` | 16 | Sprint ledger |
| `rust-router/src/canonical/qualification/stages/mod.rs` | 11 | Qualification stages |
| `rust-router/src/residency/mod.rs` | 39 | Supervisor architecture |
| `rust-router/src/residency/state.rs` | 317 | 8-state machine |
| `rust-router/src/residency/supervisor.rs` | 1324 | Supervisor implementation |
| `rust-router/src/operator/mod.rs` | 13 | Operator surface |
| `rust-router/src/evidence/mod.rs` | — | Evidence recording |
| `rust-router/Cargo.toml` | 42 | Dependencies |
| `docs/architecture/RUNTIME-NODE-ARCHITECTURE.md` | 159 | Architecture doc |
| `docs/planning/MODEL-QUALIFICATION-ROUTER-AUTHORITY-MAP.md` | 101 | Ownership matrix |
| `docs/planning/WIN-MAC-MODEL-QUALIFICATION-EVIDENCE-BRIDGE.md` | 179 | Bridge contract |
| `docs/planning/WIN-MULTINODE-MCP-DOCUMENT-CUSTODY-NOTES.md` | 270 | MCP custody design |
| `mcp/templates/mcp-server.example.json` | 102 | MCP bridge templates |
| `scripts/mcp-bridge.ps1` | 52 | stdio bridge |
| `G:\Models\docs\planning\LIBRARIANOS-CORE-NODE-ARCHITECTURE-MAPPING.md` | ~600 | Architecture mapping |

---

## 11. Major Discoveries

1. **The `canonical/` module is a fully implemented Mac-side authority** — 19 submodules covering packets, bridge, qualification, capability evidence, release, review, routing, ledger, registry, provenance, comparative analysis, lifecycle, pipeline, and observability. The Mac-side CanonicalDatabase (SQLite) with full CRUD for ModelIdentityRecord, SystemProfile, TaskPack, and ValidatorPack exists.

2. **The bridge is bidirectional and contract-defined** — `QualificationRequest` (Mac→Windows) and `EvidencePacket` (Windows→Mac) are sealed packet types with versioning, SHA-256 hashing, and `assert_no_capability_data()` enforcement. There is no capability authority data in bridge packets.

3. **The residency supervisor is complete** — 8-state machine (Unloaded→Loading→Ready→Running→Draining→Unloading→VerifyingRelease→Unloaded→Failed) with startup reconciliation, lease enforcement, and GPU release verification.

4. **Core and Node logic co-exist in the same crate** — The `rust-router` crate is a monolith containing both Mac-side canonical logic (`canonical/`) and Windows-side runtime logic (`residency/`, `process/`, `server/`, `db/`, `evidence/`). They are not physically separated.

5. **MCP is proposed but not active** — The architecture has draft MCP bridge scripts and template configs, but the active Core/Node communication uses HTTP/REST with bridge contracts, not MCP protocol.

---

## 12. Unknowns / Questions Requiring Mac/Core Clarification

1. **Where does the Mac-side CanonicalDatabase live at runtime?** The `canonical/db.rs` module is compiled into the Windows rust-router crate. Is the canonical DB intended to run on the Mac machine, or is it co-located with the Windows runtime?

2. **Does the `canonical/` module execute on Mac or Windows?** The bridge client makes HTTP requests `to` the Windows node — this suggests `canonical/` runs on Mac. But it's compiled into the same binary.

3. **What is the intended deployment model?** Single binary with both Core and Node logic? Or two separate deployments (Mac binary = canonical/, Windows binary = everything else)?

4. **Is the sprint ledger (`canonical/ledger/`) shared or per-node?**

5. **What is the relationship between `win-custody-ledger` (PowerShell) and `canonical/ledger/` (Rust)?** They appear to serve similar purposes with different scopes.

6. **What is the OperatorService (`operator/service.rs`)?** Dashboard? Taskbar agent? Event stream?

---

## 13. Recommended Next Architectural Question

**"Should the `canonical/` module be extracted into a separate crate/binary to enforce the Core/Node separation at compile time?"**

The current monolith (`rust-router`) contains both Core authority logic and Node execution logic. The packet contracts and bridge client define clear boundary types, but the physical co-location means there is no compile-time enforcement of the authority separation.

Extracting `canonical/` into a separate crate would:
- Enforce the Core→Node direction at the dependency level
- Prevent accidental Core logic from depending on Node logic
- Allow independent deployment cycles
- Clarify which machine runs which binary
- Make the authority boundary explicit in the build system
