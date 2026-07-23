# EPIC-CORE-NODE-CRATE-SEPARATION

**Status:** Planned  
**ADR:** ADR-NODE-001 (Accepted)  
**Preceded By:** Dependency boundary audit (Case A — clean, zero violations)  
**Repository:** `G:\openwork\librarian-runtime-node\`  
**Design Constraint:** No behavioral changes — mechanical extraction only

---

## 1. Objective

Extract the existing `rust-router` crate into a three-crate Cargo workspace that enforces Core/Node authority separation at compile time.

The existing codebase already respects the boundary logically (zero forbidden imports, zero circular dependencies). This epic makes that boundary **impossible to violate accidentally** by ensuring a forbidden cross-crate import produces a compilation error.

## 2. Scope

### In Scope

- Create `librarian-contracts` — shared packet types, validation primitives, bridge client
- Create `librarian-core` — canonical authority (qualification, governance, provenance, routing, capability)
- Create `librarian-node` — runtime execution (server, process, residency, evidence, operator, runtime DB)
- Configure Cargo workspace in repository root
- Move all source files to their respective crates with updated import paths
- Preserve all existing behavior (zero behavioral changes)
- Preserve all existing tests (move with their domain)
- Verify forbidden imports produce compile errors
- Verify workspace builds in release mode

### Not In Scope

- MCP implementation or bridge activation
- Node identity or registration
- Authentication or authorization
- Evidence reconciliation protocol
- Owner decision workflow
- Mac deployment or Core service activation
- Any new runtime behavior
- Refactoring, renaming, or redesigning existing types

The crate split is **architectural enforcement only**. Every capability that works before the split must work identically after it.

## 3. Extraction Order

The dependency graph dictates the order:

```
1. librarian-contracts     ← zero dependencies on core or node
        ↑
2. librarian-core          ← depends on contracts only
        ↑
3. librarian-node          ← depends on contracts only
```

Each step must build and pass tests before the next step begins.

## 4. Crate Definitions

### 4.1 `librarian-contracts`

**Purpose:** Neutral ground — no single domain owns these types.

**Contents (from current `canonical::packets` + `canonical::bridge`):**

| Source File | Target |
|-------------|--------|
| `canonical/packets/mod.rs` | `src/lib.rs` (re-export) |
| `canonical/packets/common.rs` | `src/common.rs` |
| `canonical/packets/evidence_packet.rs` | `src/evidence_packet.rs` |
| `canonical/packets/qualification_request.rs` | `src/qualification_request.rs` |
| `canonical/packets/residency_status.rs` | `src/residency_status.rs` |
| `canonical/bridge/client.rs` | `src/bridge/client.rs` |
| `canonical/bridge/mod.rs` | `src/bridge/mod.rs` |

**External dependencies:** serde, serde_json, anyhow, sha2, chrono, reqwest, uuid

**What this crate does NOT contain:**
- No database logic
- No qualification logic
- No governance logic
- No runtime/process logic
- No HTTP server

**Tests:** Tests embedded in source files move with their files. No external test files reference only contract types.

### 4.2 `librarian-core`

**Purpose:** Canonical authority implementation.

**Contents (from current `canonical/` minus `packets/` and `bridge/`):**

| Source | Target |
|--------|--------|
| `canonical/db.rs` | `src/db.rs` |
| `canonical/connection.rs` | `src/connection.rs` |
| `canonical/migrations.rs` | `src/migrations.rs` |
| `canonical/models/*` | `src/models/` |
| `canonical/capability/*` | `src/capability/` |
| `canonical/capability_evidence/*` | `src/capability_evidence/` |
| `canonical/comparative/*` | `src/comparative/` |
| `canonical/ledger/*` | `src/ledger/` |
| `canonical/lifecycle/*` | `src/lifecycle/` |
| `canonical/observability/*` | `src/observability/` |
| `canonical/pipeline/*` | `src/pipeline/` |
| `canonical/provenance/*` | `src/provenance/` |
| `canonical/qualification/*` | `src/qualification/` |
| `canonical/registry/*` | `src/registry/` |
| `canonical/release/*` | `src/release/` |
| `canonical/review/*` | `src/review/` |
| `canonical/routing/*` | `src/routing/` |
| `canonical/mod.rs` | `src/lib.rs` |

**Dependencies:** `librarian-contracts`, rusqlite (bundled), serde, serde_json, anyhow, sha2, chrono, uuid, reqwest, tempfile (dev)

**What this crate does NOT contain:**
- No process management
- No GPU/residency management
- No HTTP server endpoints
- No evidence collection (Node responsibility)
- No Windows-specific code

### 4.3 `librarian-node`

**Purpose:** Runtime execution environment.

**Contents (from current `rust-router` minus `canonical/`):**

| Source | Target |
|--------|--------|
| `config.rs` | `src/config.rs` |
| `models/*` | `src/models/` |
| `db/*` | `src/db/` |
| `runtime_state/*` | `src/runtime_state/` |
| `residency/*` | `src/residency/` |
| `evidence/*` | `src/evidence/` |
| `operator/*` | `src/operator/` |
| `process.rs` | `src/process.rs` |
| `server.rs` | `src/server.rs` |
| `refusal.rs` | `src/refusal.rs` |
| `main.rs` | `src/main.rs` |
| `lib.rs` | `src/lib.rs` |

**Dependencies:** `librarian-contracts`, axum, tokio, tower, tower-http, serde, serde_json, reqwest, tracing, tracing-subscriber, clap, chrono, uuid, rusqlite, sha2, anyhow, tempfile (dev)

**What this crate does NOT contain:**
- No canonical database
- No qualification logic
- No governance or ledger
- No capability policy
- No provenance

## 5. Workspace Structure

```
G:\openwork\librarian-runtime-node\
│
├── Cargo.toml              ← workspace manifest
│
├── librarian-contracts/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── common.rs
│       ├── evidence_packet.rs
│       ├── qualification_request.rs
│       ├── residency_status.rs
│       └── bridge/
│           ├── mod.rs
│           └── client.rs
│
├── librarian-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── db.rs
│       ├── connection.rs
│       ├── migrations.rs
│       ├── capability/
│       ├── capability_evidence/
│       ├── comparative/
│       ├── ledger/
│       ├── lifecycle/
│       ├── models/
│       ├── observability/
│       ├── pipeline/
│       ├── provenance/
│       ├── qualification/
│       ├── registry/
│       ├── release/
│       ├── review/
│       └── routing/
│
├── librarian-node/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── config.rs
│       ├── process.rs
│       ├── server.rs
│       ├── refusal.rs
│       ├── models/
│       ├── db/
│       ├── runtime_state/
│       ├── residency/
│       ├── evidence/
│       └── operator/
│
└── docs/
    └── planning/
        ├── CORE-NODE-DEPENDENCY-BOUNDARY-AUDIT.md
        ├── CORE-NODE-DEPENDENCY-BOUNDARY-AUDIT-REPORT.md
        └── EPIC-CORE-NODE-CRATE-SEPARATION.md
```

## 6. Dependency Enforcement

### 6.1 Workspace `Cargo.toml`

```toml
[workspace]
members = [
    "librarian-contracts",
    "librarian-core",
    "librarian-node",
]

[workspace.dependencies]
# Shared version pinning for common dependencies
serde = "1"
serde_json = "1"
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
sha2 = "0.10"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
rusqlite = { version = "0.31", features = ["bundled"] }
tempfile = "3"
```

### 6.2 Forbidden Dependency Verification

The workspace layout makes forbidden imports impossible at the `Cargo.toml` level:

| Import | Allowed In | Effect |
|--------|-----------|--------|
| `librarian-node` → `librarian-core` | **Nowhere** | Not listed as dependency → compile error |
| `librarian-core` → `librarian-node` | **Nowhere** | Not listed as dependency → compile error |
| `librarian-contracts` → `librarian-core` | **Nowhere** | Not listed as dependency → compile error |
| `librarian-contracts` → `librarian-node` | **Nowhere** | Not listed as dependency → compile error |
| `*` → `librarian-contracts` | **Everywhere** | Explicit dependency required |

Additional verification (CI gate):
```bash
# Verify no core→node or node→core dependencies exist
cargo metadata --format-version 1 | jq '.packages[] | select(.name == "librarian-core") | .dependencies[].name'
# Should not include "librarian-node"

cargo metadata --format-version 1 | jq '.packages[] | select(.name == "librarian-node") | .dependencies[].name'
# Should not include "librarian-core"
```

## 7. Acceptance Gates

| Gate | Criteria | Verification |
|------|----------|-------------|
| G-CONTRACTS | `librarian-contracts` builds in release mode | `cargo build -p librarian-contracts --release` |
| G-CORE | `librarian-core` builds in release mode | `cargo build -p librarian-core --release` |
| G-NODE | `librarian-node` builds in release mode | `cargo build -p librarian-node --release` |
| G-WORKSPACE | Workspace builds entirely | `cargo build --release` |
| G-NO-FORBIDDEN | No core→node or node→core dependencies | `cargo metadata` inspection |
| G-TESTS-CONTRACTS | All contract tests pass | `cargo test -p librarian-contracts` |
| G-TESTS-CORE | All core tests pass | `cargo test -p librarian-core` |
| G-TESTS-NODE | All node tests pass | `cargo test -p librarian-node` |
| G-TESTS-WORKSPACE | All workspace tests pass | `cargo test --workspace` |
| G-OLD-DELETED | Original `rust-router` crate removed | No `rust-router/Cargo.toml` |
| G-NO-BEHAVIOR-CHANGE | Existing HTTP contract tests pass against node crate | Run existing endpoint test suite |
| G-NO-CORE-NODE-MIX | `cargo test -p librarian-node` does not run core tests | Confirmed separately |
| G-DOCS | Architectural docs reference new crate structure | Update any stale references |

## 8. Test Migration

Each test file moves with its domain:

| Current Path | Moves To | Domain |
|-------------|----------|--------|
| `tests/integration_test.rs` | `librarian-node/tests/` | Node |
| `tests/registry_persistence_test.rs` | `librarian-core/tests/` | Core |
| `tests/bridge_integration_test.rs` | `librarian-core/tests/` | Core |
| `tests/capability_evidence_*.rs` (8 files) | `librarian-core/tests/` | Core |
| `tests/comparative_persistence_test.rs` | `librarian-core/tests/` | Core |
| `tests/release_trust_test.rs` | `librarian-core/tests/` | Core |
| `tests/regression_harness.rs` | `librarian-core/tests/` | Core |
| `tests/custom_evidence_integration_test.rs` | `librarian-core/tests/` | Core |
| `tests/batch_qualification_test.rs` | `librarian-core/tests/` | Core |

Inline tests (`#[cfg(test)]`) within source files move with their source files. The audit confirmed zero cross-domain inline test references.

## 9. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Import path rewrites introduce typos | Extract one file at a time; compile after each move |
| Test paths become stale | Move tests with their domain files, not separately |
| External tooling references old crate name | Update scripts, docs, and CI configs in the same pass |
| Version drift between contract crate consumers | Use workspace-level dependency pinning |
| Accidental behavior change during split | Compare test pass counts before/after each crate extraction |

## 10. Effort Estimate

| Step | Files | Import Changes | Risk |
|------|-------|---------------|------|
| Create workspace + `librarian-contracts` | ~7 source + 1 Cargo.toml | None (new crate, no existing consumers) | Low |
| Create `librarian-core` | ~60+ source + 1 Cargo.toml | `crate::canonical::*` → `librarian_core::*` | Low — mechanical find/replace |
| Create `librarian-node` | ~20 source + 1 Cargo.toml | `crate::*` → `librarian_node::*` | Low — mechanical find/replace |
| Move tests | ~9 test files | Update `use` paths | Low |
| Remove old crate | 1 directory + 1 Cargo.toml deletion | — | Low (after all files moved) |
| CI config update | 1 CI config file | Update build/test commands | Low |

**Total:** ~90 file movements, ~5 files needing import adaptation, 0 behavioral changes expected.
**Estimated duration:** Single focused implementation pass.

## 11. Completion Criteria

The epic is complete when:

- [ ] `cargo build --release` succeeds from workspace root
- [ ] `cargo test --workspace` passes with same count as pre-split
- [ ] `librarian-contracts` has no database, process, or HTTP dependencies
- [ ] `librarian-core` depends on `librarian-contracts` only (plus external)
- [ ] `librarian-node` depends on `librarian-contracts` only (plus external) — NOT on `librarian-core`
- [ ] Original `rust-router/` crate directory is removed
- [ ] No remaining references to old crate name in source, tests, or scripts
- [ ] Existing HTTP endpoint contract tests pass against new `librarian-node` binary
- [ ] Architectural docs reflect new crate structure

## 12. Gate Structure

The epic executes as gated sub-sprints. Each gate has fixed acceptance criteria that must not be modified during execution. If extraction discovers a problem, the result is "Gate blocked — remediation required," not "Gate passed with changed criteria."

```
EPIC-CORE-NODE-CRATE-SEPARATION
│
├── G-CONTRACTS    ✅ Sealed
│
├── G-CORE
│   ├── Create librarian-core crate
│   ├── Move canonical authority modules (minus packets/bridge)
│   ├── Rewire imports from crate::canonical:: to librarian_core::
│   ├── Rewire packet references from crate::canonical::packets to librarian_contracts
│   ├── Validate dependency direction (no librarian-node dependency)
│   └── Seal: cargo build + test pass
│
├── G-NODE
│   ├── Create librarian-node crate
│   ├── Move runtime modules (residency, process, evidence, server, operator, db, runtime_state)
│   ├── Rewire imports from crate:: to librarian_node::
│   ├── Rewire packet references to librarian_contracts
│   ├── Validate no librarian-core dependency
│   └── Seal: cargo build + test pass
│
└── WORKSPACE-CLOSURE
    ├── Remove rust-router migration shell
    ├── Validate full workspace build
    ├── Run complete test suite
    ├── Verify dependency graph: contracts ← core, contracts ← node (no cross-deps)
    └── Produce completion report
```

## 13. Final Report Requirements

The completion report must include evidence for each of the following:

### 13.1 Workspace Result

Diagram showing the transformation:

```
Before:
rust-router
 ├── canonical
 ├── runtime/residency
 ├── evidence
 └── process

After:
librarian-contracts
librarian-core
librarian-node
```

### 13.2 Gate Results Table

| Gate | Result |
|------|--------|
| G-CONTRACTS | ✅ Sealed |
| G-CORE | ✅ or ⏳ or ❌ |
| G-NODE | ✅ or ⏳ or ❌ |
| WORKSPACE-CLOSURE | ✅ or ⏳ or ❌ |

### 13.3 Dependency Proof

Explicit verification that:

```
librarian-core
  imports: librarian-contracts, core dependencies
  forbidden: librarian-node ❌

librarian-node
  imports: librarian-contracts, runtime dependencies  
  forbidden: librarian-core ❌
```

### 13.4 Behavioral Preservation

Evidence that:
- Existing qualification tests pass
- Existing packet tests pass
- Existing evidence tests pass
- Existing runtime tests pass
- No schema changes
- No protocol changes
- No feature additions

### 13.5 Migration Exceptions

If anything cannot be moved mechanically, record:

```
Temporary Compatibility Exception
- File/module:
- Reason:
- Owner:
- Removal condition:
```

No silent compromises. Every exception must have a documented removal condition.

## 14. Non-Negotiable Constraint

**Do not modify gate acceptance criteria during execution.**

If extraction discovers a problem, the result is:

"Gate blocked — remediation required"

not:

"Gate passed with changed criteria."

This preserves ADR-NODE-001 as an architectural control point. The acceptance criteria are the evidence that the authority boundary is enforceable — changing them during extraction invalidates that evidence.
