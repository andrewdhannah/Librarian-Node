# Core/Node Dependency Boundary Audit Report

**Date:** 2026-07-15  
**Audited crate:** `rust-router` at `G:\openwork\librarian-runtime-node\rust-router\`  
**Methodology:** Per `docs/planning/CORE-NODE-DEPENDENCY-BOUNDARY-AUDIT.md`  
**Constraint:** No code modified — snapshot of current architecture only.

---

## 1. Dependency Graph

### 1.1 Module Group Import Matrix

Every `use crate::...` import traced per module group. External crate imports and `std`/`tokio` omitted.

| Group | `crate` Imports From | Imports canonical/? | Imports Node? | Clean? |
|-------|---------------------|---------------------|---------------|--------|
| **canonical/** | canonical::{all submodules} | Self | **None** | ✅ |
| **config/** | (none — serde, std only) | No | No | ✅ |
| **models/** | (none — serde, chrono only) | No | No | ✅ |
| **db/** | config, models, runtime_state | No | Yes (models, runtime_state) | ✅ |
| **runtime_state/** | (none — serde only) | No | No | ✅ |
| **residency/** | db, runtime_state | No | Yes (db, runtime_state) | ✅ |
| **evidence/** | db, runtime_state, **canonical::packets** | **Yes (packets only)** | Yes (db, runtime_state) | ✅* |
| **operator/** | db, process, residency | No | Yes (db, process, residency) | ✅ |
| **process/** | config | No | Yes (config) | ✅ |
| **server/** | config, db, evidence, process, refusal, residency, operator | No | Yes (config, db, evidence, process, refusal, residency, operator) | ✅ |
| **refusal/** | (none — chrono, serde_json only) | No | No | ✅ |
| **main/** | config, db, evidence, operator, residency, server | No | Yes (all Node modules) | ✅ |
| **lib/** | — (module declarations only) | — | — | ✅ |

> \* `evidence/export.rs` and `evidence/residency_status.rs` import `crate::canonical::packets::*` — **allowed** (contract types).

### 1.2 Dependency Edge Summary

```
canonical/ ──→ canonical/{submodules} only
                (NO edges to Node)

Node modules ──→ config, models, runtime_state, db
              ──→ canonical::packets (evidence/ only)
              ──→ each other (intra-Node)
```

All non-packet `crate::canonical` imports from Node modules: **zero**.

---

## 2. Forbidden Dependency Check

### 2.1 Core→Node Violations

**Result: NONE.**

All files under `canonical/` (19 submodules + db.rs, connection.rs, migrations.rs, mod.rs) were searched for `use crate::` patterns. Every import targets either:
- `crate::canonical::{submodule}`
- External crates (anyhow, rusqlite, serde, sha2, chrono, reqwest)

No file in `canonical/` imports `residency/`, `evidence/`, `db/` (the Node db, not canonical::db), `runtime_state/`, `operator/`, `process.rs`, `server.rs`, or `refusal.rs`.

### 2.2 Node→Core Violations

**Result: NONE (all allowed).**

Searched: `residency/`, `evidence/`, `db/`, `runtime_state/`, `operator/`, `process.rs`, `server.rs`, `refusal.rs`.

| File | Imports from canonical | Classification |
|------|----------------------|----------------|
| `evidence/export.rs:20-24` | `canonical::packets::common::*` | **Allowed** — contract types |
| `evidence/export.rs:24` | `canonical::packets::evidence_packet::EvidencePacket` | **Allowed** — contract types |
| `evidence/residency_status.rs:19` | `canonical::packets::residency_status::*` | **Allowed** — contract types |

No Node file imports from `canonical::ledger`, `canonical::db`, `canonical::capability`, `canonical::provenance`, `canonical::release`, `canonical::review`, `canonical::routing`, or any other canonical authority submodule.

### 2.3 Severity Classification

| Category | Count | Severity |
|----------|-------|----------|
| Core→Node violations | 0 | — |
| Node→Core (non-packet) violations | 0 | — |
| Node→Core (packet — allowed) | 2 files | ✅ No action needed |

**The architecture already respects the Core/Node boundary at the source-code level.**

---

## 3. Shared Contract Candidates

### 3.1 Packet Types (→ `librarian-contracts`)

All currently in `canonical::packets`, used by both Core and Node:

| Type | File | Used By | Move? |
|------|------|---------|-------|
| `EvidencePacket` | `canonical/packets/evidence_packet.rs` | canonical/bridge, evidence/export | **Yes** |
| `QualificationRequest` | `canonical/packets/qualification_request.rs` | canonical/qualification, canonical/bridge | **Yes** |
| `ResidencyStatusResponse` | `canonical/packets/residency_status.rs` | canonical/bridge, evidence/residency_status | **Yes** |
| `ActiveLease` | `canonical/packets/residency_status.rs` | canonical/bridge, evidence/residency_status | **Yes** |
| `ActiveRun` | `canonical/packets/residency_status.rs` | canonical/bridge, evidence/residency_status | **Yes** |
| `ResidencyStatusQuery` | `canonical/packets/residency_status.rs` | canonical/bridge, server.rs | **Yes** |
| `PacketModelIdentity` | `canonical/packets/common.rs` | evidence_packet, qualification_request | **Yes** |
| `PacketExecutionConfig` | `canonical/packets/common.rs` | qualification_request | **Yes** |
| `PacketConstraints` | `canonical/packets/common.rs` | qualification_request | **Yes** |
| `PacketExecutionIdentity` | `canonical/packets/common.rs` | evidence_packet | **Yes** |
| `PacketLeaseLifecycle` | `canonical/packets/common.rs` | evidence_packet | **Yes** |
| `PacketExecutionMetrics` | `canonical/packets/common.rs` | evidence_packet | **Yes** |
| `PacketLifecycleEvent` | `canonical/packets/common.rs` | evidence_packet | **Yes** |
| `PacketReleaseVerification` | `canonical/packets/common.rs` | evidence_packet | **Yes** |
| `LifecycleEvent` (raw bridge) | `canonical/bridge/client.rs` | canonical/bridge | **Yes** |
| `LifecycleResponse` (raw bridge) | `canonical/bridge/client.rs` | canonical/bridge | **Yes** |
| `BridgeError` | `canonical/bridge/client.rs` | canonical/bridge | **Yes** |
| `EvidenceRunResponse` | `canonical/bridge/client.rs` | canonical/bridge | **Yes** (type alias) |
| `BridgeClient` | `canonical/bridge/client.rs` | canonical | **Yes** |

### 3.2 Shared Models (→ contracts or duplicated)

| Type | Current Location | Used By | Assessment |
|------|-----------------|---------|------------|
| `LeaseState` | `runtime_state/model_lease.rs` | db, residency, runtime_state, evidence | Node-only — stays |
| `LifecycleEventType` | `runtime_state/lifecycle_evidence.rs` | db, residency, runtime_state, evidence | Node-only — stays |
| `ResolverState` | `residency/state.rs` | residency only | Node-only — stays |
| `RuntimeStopStrategy` | `residency/state.rs` | residency only | Node-only — stays |
| `BackendState` | `process.rs` | server, operator | Node-only — stays |
| `BackendProcess` | `process.rs` | server, operator | Node-only — stays |
| `LocalModel` | `models/local_model.rs` | db, evidence tests | Node-only — stays |
| `RuntimeProfile` | `models/runtime_profile.rs` | db, evidence tests | Node-only — stays |
| `HardwareProfile` | `models/hardware_profile.rs` | db only | Node-only — stays |
| `Profile`, `RouterConfig`, `ProfileManager` | `config.rs` | process, server, db, main | Shared — could move to contracts |

### 3.3 Validation Primitives

| Function/Type | Location | Used By |
|--------------|----------|---------|
| `validate_transition()` | `residency/state.rs` | residency/supervisor.rs (Node-only) |
| `Packet::validate()` | `canonical/packets/*` | evidence/export, server, canonical/bridge |
| `Packet::assert_no_capability_data()` | `canonical/packets/*` | evidence/export, evidence/residency_status, server |

The validation methods are inherent to the packet types and would move with them to contracts.

### 3.4 Serialization Helpers

The packet types use `serde::Serialize`/`Deserialize` with standard derives — no custom serialization helpers. `to_json()`, `from_json()`, and `compute_hash()` are inherent methods on each packet type and would move with them.

---

## 4. Extraction Plan

### 4.1 Move Unchanged to `librarian-contracts`

All files from `canonical/packets/` and `canonical/bridge/client.rs`:
- `canonical/packets/mod.rs` → `librarian-contracts/src/lib.rs` (or similar)
- `canonical/packets/common.rs` → `librarian-contracts/src/common.rs`
- `canonical/packets/evidence_packet.rs` → `librarian-contracts/src/evidence_packet.rs`
- `canonical/packets/qualification_request.rs` → `librarian-contracts/src/qualification_request.rs`
- `canonical/packets/residency_status.rs` → `librarian-contracts/src/residency_status.rs`
- `canonical/bridge/client.rs` → `librarian-contracts/src/bridge/client.rs`

**External crates needed in contracts:** serde, serde_json, anyhow, sha2, chrono, reqwest (for bridge client).

### 4.2 Move Unchanged to `librarian-core`

All remaining files under `canonical/`:
- `canonical/db.rs`, `canonical/connection.rs`, `canonical/migrations.rs`
- `canonical/capability/`, `canonical/capability_evidence/`
- `canonical/comparative/`, `canonical/ledger/`, `canonical/lifecycle/`
- `canonical/models/`, `canonical/observability/`
- `canonical/pipeline/`, `canonical/provenance/`
- `canonical/qualification/`, `canonical/registry/`
- `canonical/release/`, `canonical/review/`, `canonical/routing/`

These files import only from `crate::canonical::*` (same-group) and external crates — no adaptation needed except renaming the crate prefix.

**External crates needed in core:** rusqlite (bundled), serde, serde_json, anyhow, sha2, chrono, uuid, reqwest (for bridge client), `librarian-contracts` (as dependency).

### 4.3 Move Unchanged to `librarian-node`

- `config.rs` → `librarian-node/src/config.rs`
- `models/*` → `librarian-node/src/models/`
- `runtime_state/*` → `librarian-node/src/runtime_state/`
- `residency/*` → `librarian-node/src/residency/`
- `evidence/*` → `librarian-node/src/evidence/`
- `db/*` → `librarian-node/src/db/`
- `operator/*` → `librarian-node/src/operator/`
- `process.rs` → `librarian-node/src/process.rs`
- `server.rs` → `librarian-node/src/server.rs`
- `refusal.rs` → `librarian-node/src/refusal.rs`
- `main.rs` → `librarian-node/src/main.rs`
- `lib.rs` → `librarian-node/src/lib.rs` (module declarations only)

**External crates needed in node:** axum, tokio, tower, tower-http, serde, serde_json, reqwest, tracing, tracing-subscriber, clap, chrono, uuid, rusqlite, sha2, anyhow, `librarian-contracts` (as dependency).

### 4.4 Move with Adaptation

| File | Adaptation Required |
|------|-------------------|
| `evidence/export.rs` | Change `crate::canonical::packets::*` to `librarian_contracts::*` |
| `evidence/residency_status.rs` | Change `crate::canonical::packets::*` to `librarian_contracts::*` |
| `server.rs` | Change `crate::evidence::*` to `librarian_node::evidence::*` (already intra-crate) |
| `canonical/bridge/client.rs` | Change `crate::canonical::packets::*` to `librarian_contracts::*` |
| All canonical files with `crate::canonical` imports | Change to `librarian_core::canonical` paths |
| All Node files with `crate::{config,db,evidence,...}` | Change to `librarian_node::{config,db,...}` |

### 4.5 Remains in Place (no change)

Nothing truly "remains in place" — the physical split requires every file to move to its respective crate. However, the source files themselves need **minimal content changes** (only import paths).

---

## 5. Risk Assessment

### 5.1 Circular Dependencies

**None detected.** The import graph is strictly acyclic:

```
canonical/ ──────→ canonical/{submodules}
     │
     └──(packets)──→ contracts (proposed)

Node modules ──────→ each other (config → process → server → {db, evidence, residency, operator})
                         ↑                              │
                         └── (no cycles) ───────────────┘

contracts ──────→ (standalone, no crate imports)
core ───────────→ contracts, external crates (NO dependency on node)
node ───────────→ contracts, external crates (NO dependency on core)
```

**Risk: Low.** No circular dependencies exist in the current codebase.

### 5.2 Test Migration Impact

| Test File | Domain(s) Referenced | Impact |
|-----------|---------------------|--------|
| `tests/integration_test.rs` | Node (server, config, db, evidence, operator, residency) | Low — all Node, stays in node |
| `tests/registry_persistence_test.rs` | Core (capability, comparative, registry, routing) | Low — all Core, stays in core |
| `tests/bridge_integration_test.rs` | Core (bridge client) | Low — all Core, stays in core |
| `tests/capability_evidence_*.rs` (8 files) | Core (capability_evidence) | Low — all Core, stays in core |
| `tests/comparative_persistence_test.rs` | Core (capability, comparative, registry, routing) | Low — all Core, stays in core |
| `tests/release_trust_test.rs` | Core (release) | Low — all Core, stays in core |
| `tests/regression_harness.rs` | Core (qualification, routing, observability, provenance, review) | Low — all Core, stays in core |
| `tests/custom_evidence_integration_test.rs` | Core (qualification) | Low — all Core, stays in core |
| `tests/batch_qualification_test.rs` | Core (packets, qualification) | Low — all Core, stays in core |

**Assessment:** No test file references both Core and Node modules. Each test targets a single domain. **Zero integration tests span the boundary.** This makes migration trivial — each test file moves with its domain.

**Risk: Very Low.** No test migration conflicts expected.

### 5.3 Temporary Compatibility Requirements

| Issue | Requirement | Duration |
|-------|------------|----------|
| Packet types currently in `canonical::packets` | Both core and node need these during extraction | Publish `librarian-contracts` first, then update both crates |
| Bridge client in `canonical::bridge` | Core needs it; Node does not | Moves to contracts (or stays in core as a consumer) |
| `config.rs` types (RouterConfig, Profile, ProfileManager) | Used by both process.rs and server.rs | Move to node; both are Node modules |
| `models/` types (LocalModel, RuntimeProfile, HardwareProfile) | Only used by Node modules | Move to node — no split needed |

**Required order of extraction:**
1. Extract `librarian-contracts` (packet types + bridge client) — zero dependency on core or node
2. Extract `librarian-core` (canonical/ minus packets) — depends on contracts
3. Extract `librarian-node` — depends on contracts, zero dependency on core
4. Remove original `rust-router` crate

### 5.4 Inline Test Impact

Tests embedded within source files (under `#[cfg(test)]`) reference the same crate's types. During extraction:
- `canonical/db.rs` tests use `tempfile` and canonical types only — move to core crate
- `residency/supervisor.rs` tests use `db::RuntimeDatabase`, `models::*` — move to node crate
- `evidence/export.rs` tests use `db`, `models`, `runtime_state` — move to node crate
- `db/mod.rs` tests use `models`, `runtime_state` — move to node crate

No test references across the Core/Node boundary. **No test rewriting needed.**

---

## 6. ADR Recommendation

### 6.1 Classification Table

| Question | Answer |
|----------|--------|
| Is Core/Node separation logically respected today? | **Yes** — canonical/ modules import zero Node modules. Node modules only import canonical::packets (allowed contract boundary). |
| Are there any current authority leakage violations? | **None.** Zero forbidden imports found. |
| Is extraction technically feasible today? | **Yes** — no blockers. The architecture is already clean. |
| Biggest extraction blocker? | **None.** The primary effort is mechanical (file moves + import path rewrites). |
| Do the packet types need to move to a shared contract crate? | **Yes** — 19 types across 4 files currently in canonical::packets used by both domains. |
| How many Node modules currently import from canonical/? | **2 files** (evidence/export.rs, evidence/residency_status.rs) — both only import from canonical::packets. |
| How many canonical/ modules currently import from Node? | **0.** |
| Recommended crate boundary? | `librarian-core`, `librarian-node`, `librarian-contracts` (workspace members in the same repo). |
| Recommended ADR decision? | **Model B (workspace separation)** — no circular dependencies, no shared mutable state, clean dependency direction. |

### 6.2 Recommended Decision: Model B — Workspace Separation

**Rationale:**

1. **Architecture is already clean** — the audit found zero violations and zero circular dependencies. The separation already exists logically; it just needs physical enforcement.

2. **Minimal migration effort** — only import path changes needed. No behavioral code changes.

3. **Test separation is already present** — no integration test spans both domains. Each test targets only Core or only Node.

4. **Extraction estimate:**
   - ~19 files moved unchanged to `librarian-contracts` (packets + bridge client)
   - ~60+ files moved unchanged to `librarian-core` (canonical/ minus packets)
   - ~20 files moved unchanged to `librarian-node` (everything else)
   - ~5 files with import path adaptation (evidence/export.rs, evidence/residency_status.rs, canonical/bridge/client.rs, canonical files referencing packets)
   - 0 test files needing rewriting
   - 0 shared dependency conflicts

5. **Why Model B over Model C (distributed services):** The current architecture uses in-process function calls, not networked services. The bridge client (canonical/bridge) already handles HTTP communication. Turning the Node into a separate service would add latency, deployment complexity, and error-handling overhead without architectural benefit at this stage. Model C can be adopted later when scaling demands it.
