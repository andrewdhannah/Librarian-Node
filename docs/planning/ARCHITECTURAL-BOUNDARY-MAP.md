# Architectural Boundary Map

**Status:** Active  
**Date:** 2026-07-19  
**Preceded By:** ADR-PLATFORM-001-CORE-NODE-AUTHORITY-ARCHITECTURE.md  
**Workspace:** G:\Models  
**Purpose:** Map existing code modules to architectural layers and identify extraction boundaries

---

## 1. Purpose

This document maps the current codebase to the architectural layers defined in ADR-PLATFORM-001. It serves as the prerequisite for workspace separation — identifying exactly what belongs where before any code moves.

**Do not skip this step.** Moving code without a boundary map risks breaking contracts, creating circular dependencies, or losing test coverage.

---

## 2. Current Codebase Map

### 2.1 Rust Router Crate (`rust-router`)

**Location:** `G:\openwork\librarian-runtime-node\rust-router/`

```
rust-router/src/
├── main.rs                    ← Entry point (axum/tokio)
├── lib.rs                     ← Module declarations
│
├── canonical/                 ← CORE DOMAIN (19 submodules)
│   ├── mod.rs
│   ├── db.rs                  ← CanonicalDatabase (model_identity, task_pack, validator_pack)
│   ├── bridge/                ← BridgeClient (Mac→Windows HTTP)
│   ├── packets/               ← QualificationRequest, EvidencePacket, ResidencyStatus
│   ├── qualification/         ← Runner, stages, validator engine, batch, custom executor
│   ├── capability_evidence/   ← 13 modules, benchmark adapters
│   ├── capability/            ← Manifest builder, decisions
│   ├── ledger/                ← Sprint governance, authorization, receipts
│   ├── release/               ← Trust packages, provenance
│   ├── comparative/           ← Analyzer, audit, finding, roster
│   ├── routing/               ← Projections, execution profiles
│   ├── provenance/            ← Model provenance builder
│   ├── registry/              ← Registry store operations
│   └── review/                ← Review/builder
│
├── residency/                 ← NODE DOMAIN
│   ├── mod.rs
│   └── ...                    ← 8-state residency supervisor
│
├── process.rs                 ← NODE DOMAIN
│                                BackendProcess, child-process lifecycle
│
├── server.rs                  ← NODE DOMAIN
│                                AppState, HTTP endpoints, health polling
│
├── evidence/                  ← NODE DOMAIN
│   ├── mod.rs
│   └── ...                    ← EvidenceWriter, residency status
│
├── db/                        ← NODE DOMAIN
│   ├── mod.rs
│   ├── migrations.rs          ← Schema migration logic
│   ├── connection.rs          ← PRAGMA configuration
│   └── ...                    ← RuntimeDatabase, 6 operational tables
│
├── runtime_state/             ← NODE DOMAIN
│   ├── mod.rs
│   └── ...                    ← ModelLease, RuntimeRun, lifecycle models
│
├── operator/                  ← NODE DOMAIN
│   ├── mod.rs
│   └── ...                    ← Dashboard surface models
│
├── config.rs                  ← SHARED (RouterConfig, ProfileManager)
├── refusal.rs                 ← NODE DOMAIN (Refusal logic)
└── evidence.rs                ← NODE DOMAIN (EvidenceWriter)
```

### 2.2 Boundary Analysis

| Module | Target Crate | Dependencies | Extraction Risk |
|--------|-------------|--------------|-----------------|
| `canonical/db.rs` | librarian-core | rusqlite, serde | **Low** — self-contained |
| `canonical/bridge/` | librarian-core | reqwest, contracts | **Low** — depends only on contracts |
| `canonical/packets/` | librarian-contracts | serde, sha2 | **None** — this IS the contracts |
| `canonical/qualification/` | librarian-core | canonical/db, contracts | **Low** — internal to core |
| `canonical/capability_evidence/` | librarian-core | canonical/db, contracts | **Low** — internal to core |
| `canonical/ledger/` | librarian-core | canonical/db, contracts | **Low** — internal to core |
| `canonical/release/` | librarian-core | canonical/db, contracts | **Low** — internal to core |
| `canonical/comparative/` | librarian-core | canonical/db, contracts | **Low** — internal to core |
| `canonical/routing/` | librarian-core | canonical/db, contracts | **Low** — internal to core |
| `canonical/provenance/` | librarian-core | canonical/db, contracts | **Low** — internal to core |
| `canonical/registry/` | librarian-core | canonical/db, contracts | **Low** — internal to core |
| `canonical/capability/` | librarian-core | canonical/db, contracts | **Low** — internal to core |
| `canonical/review/` | librarian-core | canonical/db, contracts | **Low** — internal to core |
| `residency/` | librarian-node | contracts, db | **Low** — depends only on contracts |
| `process.rs` | librarian-node | contracts | **Low** — self-contained |
| `server.rs` | librarian-node | contracts, config | **Medium** — main entry point |
| `evidence/` | librarian-node | contracts, db | **Low** — depends only on contracts |
| `db/` | librarian-node | rusqlite, contracts | **Low** — self-contained |
| `runtime_state/` | librarian-node | contracts, db | **Low** — depends only on contracts |
| `operator/` | librarian-node | contracts | **Low** — depends only on contracts |
| `config.rs` | librarian-node (or shared) | serde | **Low** — configuration |
| `refusal.rs` | librarian-node | contracts | **Low** — self-contained |
| `evidence.rs` | librarian-node | contracts | **Low** — self-contained |
| `main.rs` | librarian-node (binary) | all | **Medium** — entry point |

### 2.3 Dependency Direction Audit

**Current state (monolith):**
```
canonical/ ←→ residency/
canonical/ ←→ process.rs
canonical/ ←→ server.rs
... (unrestricted)
```

**Target state (enforced):**
```
librarian-contracts
     ^        ^
     |        |
librarian-core  librarian-node
```

**Audit results:**
- Core → Node imports: **0** (no forbidden dependencies)
- Node → Core imports: **0** (no forbidden dependencies)
- Circular dependencies: **0**
- Cross-domain test contamination: **0**

**Finding:** The code already respects the boundary logically. Extraction is purely mechanical — no behavioral code changes required.

---

## 3. Extraction Plan

### 3.1 Phase 1: Contracts Extraction

**Target:** `librarian-contracts`

**What moves:**
- `canonical/packets/` — QualificationRequest, EvidencePacket, ResidencyStatus
- `canonical/packets/evidence_packet.rs` — EvidencePacket with lifecycle chain
- `canonical/packets/assert_no_capability_data()` — Type-level enforcement
- Shared validation schemas
- Cross-boundary DTOs

**Dependencies:**
- serde, serde_json
- sha2 (for packet hashing)
- chrono (for timestamps)
- uuid (for packet IDs)

**Tests:** All packet tests move with the code

### 3.2 Phase 2: Core Extraction

**Target:** `librarian-core`

**What moves:**
- `canonical/` (all submodules except `packets/`)
- `canonical/db.rs` — CanonicalDatabase
- `canonical/bridge/` — BridgeClient
- `canonical/qualification/` — Runner, stages, validator engine
- `canonical/capability_evidence/` — Benchmark adapters
- `canonical/ledger/` — Sprint governance
- `canonical/release/` — Trust packages
- `canonical/comparative/` — Analysis
- `canonical/routing/` — Projections
- `canonical/provenance/` — Model provenance
- `canonical/registry/` — Registry operations
- `canonical/capability/` — Manifest, decisions
- `canonical/review/` — Review/builder

**Dependencies:**
- librarian-contracts (packet types)
- rusqlite (canonical DB)
- reqwest (bridge client)
- serde, serde_json
- chrono, uuid

**Tests:** All canonical tests move with the code (580+ tests)

### 3.3 Phase 3: Node Extraction

**Target:** `librarian-node`

**What moves:**
- `residency/` — 8-state residency supervisor
- `process.rs` — BackendProcess lifecycle
- `server.rs` — HTTP endpoints, AppState
- `evidence/` — Evidence recording + export
- `db/` — RuntimeDatabase, 6 operational tables
- `runtime_state/` — Lease, run, lifecycle models
- `operator/` — Dashboard surface
- `config.rs` — RouterConfig, ProfileManager
- `refusal.rs` — Refusal logic
- `evidence.rs` — EvidenceWriter
- `main.rs` — Entry point (binary)

**Dependencies:**
- librarian-contracts (packet types)
- rusqlite (operational DB)
- axum, tokio (HTTP server)
- reqwest (health polling)
- serde, serde_json
- chrono, uuid

**Tests:** All runtime tests move with the code (85+ tests)

### 3.4 Extraction Order

```
1. librarian-contracts    ← extract first (no dependencies)
2. librarian-core         ← extract second (depends on contracts)
3. librarian-node         ← extract third (depends on contracts)
4. verify build           ← all crates compile independently
5. verify tests           ← all tests pass
6. verify forbidden deps  ← no Core↔Node imports
```

---

## 4. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Breaking packet contracts | Low | High | Contracts extracted first; type-level enforcement |
| Circular dependencies | Low | High | Dependency audit shows 0 circular deps currently |
| Test coverage loss | Low | Medium | All tests move with code; verify coverage post-extraction |
| Build failure | Medium | Medium | Incremental extraction; verify at each phase |
| Migration drift | Medium | Medium | Shared migration verification in CI |
| Lost institutional knowledge | Low | Low | This document captures the boundary map |

---

## 5. Verification Checklist

After extraction, verify:

- [ ] All three crates compile independently
- [ ] All tests pass (contracts: 56, core: 580+, node: 85+)
- [ ] No forbidden imports (Core→Node: 0, Node→Core: 0)
- [ ] No circular dependencies
- [ ] No cross-domain test contamination
- [ ] Binary builds and runs correctly
- [ ] HTTP endpoints respond correctly
- [ ] Database migrations run correctly
- [ ] Evidence pipeline works end-to-end

---

## 6. Next Steps

1. **Review this map** — Confirm module classifications are correct
2. **Create workspace Cargo.toml** — Define the Rust workspace with three crates
3. **Extract contracts** — Move packet types to librarian-contracts
4. **Extract core** — Move canonical modules to librarian-core
5. **Extract node** — Move runtime modules to librarian-node
6. **Verify** — Run the verification checklist
7. **Update documentation** — Reference the new crate structure

---

## 7. References

1. ADR-PLATFORM-001-CORE-NODE-AUTHORITY-ARCHITECTURE.md — Platform architecture
2. ADR-NODE-001-DISTRIBUTED-LIBRARIAN-AUTHORITY-MODEL.md — Original Core/Node ADR
3. CORE-NODE-DEPENDENCY-BOUNDARY-AUDIT.md — Dependency extraction analysis
4. CORE-NODE-DEPENDENCY-BOUNDARY-AUDIT-REPORT.md — Audit results
5. LOCAL-MODEL-ORCHESTRATION-SPRINT-PLAN.md — Sprint definitions
