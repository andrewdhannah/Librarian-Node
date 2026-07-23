# Core/Node Dependency Boundary Audit

**Objective:** Determine whether the current Rust implementation already respects Core/Node separation logically, and identify extraction risks.

**Prerequisite:** Discovery documents at `docs/planning/WINDOWS-NODE-ARCHITECTURE-DISCOVERY.md` (this repo) and `G:\Models\docs\planning\LIBRARIANOS-CORE-NODE-ARCHITECTURE-MAPPING.md`

**Do not modify code.** This is an analysis-only task.

---

## Background: The Inversion

The initial architectural assumption was:
- Core is conceptual (Mac-side, unimplemented)
- Node is implemented (Windows-side, running code)

Discovery revealed the inverse:
- **Core already exists** — `rust-router/src/canonical/` (19 submodules) implements full canonical authority
- **Node already exists** — `rust-router/src/residency/`, `process.rs`, `server.rs`, `evidence/`, `db/`, `runtime_state/`, `operator/` implement full execution
- **They are co-located in the same crate** with no compile-time separation

The ADR question is no longer *"should we create a Core/Node split?"* but *"should we enforce the split that already exists?"*

This audit determines whether extraction is technically feasible, and identifies any existing dependency violations.

---

## Audit 1: Dependency Direction

Map imports **by directory**, not by individual file. For each module group, determine which other module groups it imports from.

### Module Groups

| Group | Path | Role |
|-------|------|------|
| **canonical** | `src/canonical/` | Core authority |
| **residency** | `src/residency/` | Node — GPU residency supervisor |
| **evidence** | `src/evidence/` | Node — evidence recording + export |
| **db** | `src/db/` | Node — Windows operational DB |
| **runtime_state** | `src/runtime_state/` | Node — lease, run, lifecycle models |
| **operator** | `src/operator/` | Node — advisory dashboard surface |
| **process** | `src/process.rs` | Node — child process lifecycle |
| **server** | `src/server.rs` | Node — HTTP router + endpoints |
| **refusal** | `src/refusal.rs` | Node — request refusal logic |
| **config** | `src/config.rs` | Shared — configuration types |
| **models** | `src/models.rs` or `src/models/` | Shared — data models |

### Analysis Method

For each group, trace imports (use `use` statements and `pub mod` declarations). Report:

```
Group: canonical/
  Imports FROM:
    - crate::canonical::packets  (same group — allowed)
    - crate::config             (shared — allowed)
    - crate::db                 (NODE → NODE → BOUNDARY VIOLATION if canonical imports runtime db)
    - extern crates (rusqlite, serde, etc.)
    - std / tokio

  Exported TO (identifiable by pub use / pub fn signatures used elsewhere):
    - residency/
    - server/
```

### Record format

```
| Group | Imports From (same-crate) | Imports From (canonical/) | Imports From (runtime/) | Clean? |
|-------|--------------------------|---------------------------|------------------------|--------|
| canonical/ | config, models | — | db? evidence? | ✅/❌ |
| residency/ | db, runtime_state | canonical/...? | — | ✅/❌ |
| ... | | | | |
```

---

## Audit 2: Authority Leakage Detection

Check every `use` statement and function signature for authority-boundary violations.

### Category A: Node code accessing Core authority

Search for the following patterns in `residency/`, `evidence/`, `db/`, `runtime_state/`, `process.rs`, `server.rs`, `operator/`, `refusal.rs`:

```rust
// Patterns that would be violations:
use crate::canonical::ledger::...;       // Node accessing sprint ledger
use crate::canonical::db::...;           // Node accessing canonical DB
use crate::canonical::capability::...;   // Node accessing capability policy
use crate::canonical::provenance::...;   // Node accessing provenance
use crate::canonical::release::...;      // Node accessing release authority
use crate::canonical::review::...;       // Node accessing review
use crate::canonical::routing::...;      // Node accessing routing policy
```

**Allowed:** Node may import from `canonical::packets` (the contract types are shared).

### Category B: Core code accessing Node internals

Search for the following patterns in `canonical/`:

```rust
// Patterns that would be violations:
use crate::residency::...;               // Core controlling residency
use crate::process::...;                 // Core accessing process management
use crate::server::...;                  // Core accessing HTTP endpoints
use crate::evidence::...;               // Core accessing evidence recording
use crate::db::...;                      // Core accessing Windows operational DB
use crate::runtime_state::...;          // Core accessing lease/run state
use crate::operator::...;               // Core accessing operator surface
```

**Allowed:** `canonical::bridge::client.rs` makes HTTP requests to the Node — this is the defined communication path. The bridge client should NOT import Node internals.

### Category C: Shared type dependencies

Identify types used by both Core and Node that currently live in one domain:

| Type | Current Location | Used By | Should Move To Contracts? |
|------|-----------------|---------|--------------------------|
| BackendState | process.rs | residency, server | No — Node-only |
| ResidencyState | residency/state.rs | residency, evidence | No — Node-only |
| LeaseState | runtime_state/ | residency, db | No — Node-only |
| EvidencePacket | canonical/packets/ | canonical, bridge | Yes — contract layer |
| QualificationRequest | canonical/packets/ | canonical | Yes — contract layer |
| ResidencyStatusResponse | canonical/packets/ | canonical, bridge | Yes — contract layer |
| ... | | | |

---

## Audit 3: Extraction Feasibility

Estimate what would need to move if `canonical/` became `librarian-core` and the rest became `librarian-node`.

### 3.1 File Movement Estimate

```
Current: rust-router/src/canonical/  (40+ files)
Target:  librarian-core/src/         (40+ files)

Current: rust-router/src/* (minus canonical/)  (30+ files)
Target:  librarian-node/src/                    (30+ files)
```

### 3.2 Shared Dependency Analysis

Identify external crates used by both groups that would need to be in both `Cargo.toml` files:

| Crate | canonical/ uses? | runtime uses? | Both? |
|-------|-----------------|---------------|-------|
| rusqlite | Yes (CanonicalDatabase) | Yes (RuntimeDatabase) | **Yes** |
| serde/serde_json | Yes (packets) | Yes (evidence, config) | **Yes** |
| chrono | Yes | Yes | **Yes** |
| sha2 | Yes (packet hashing) | No? | No |
| axum | No | Yes | No |
| tokio | No (bridge uses reqwest) | Yes | No |
| reqwest | Yes (bridge client) | No? | No |
| anyhow | Yes | Yes | **Yes** |
| uuid | Yes | Yes | **Yes** |

### 3.3 Circular Dependency Detection

**Check for circular dependencies** between `canonical/` and runtime modules:

```
Example circular dependency (if it exists):
  canonical/db.rs  →  (imports from)  db/mod.rs  →  (imports from)  canonical/db.rs
```

Report any circular dependencies found. If none exist, the extraction is clean.

### 3.4 Test Dependency Analysis

Check whether any test files in one domain reference types from the other domain:

| Test File | Domain | References Other Domain? |
|-----------|--------|--------------------------|
| tests/integration_test.rs | Node | canonical? |
| tests/bridge_integration_test.rs | Bridge | canonical + runtime? |
| tests/registry_persistence_test.rs | Canonical | runtime? |
| tests/capability_evidence_*.rs | Canonical | runtime? |
| ... | | |

---

## Audit 4: Classification

Return a completed version of this table:

| Question | Answer |
|----------|--------|
| Is Core/Node separation logically respected today? (Do canonical/ modules import from runtime modules?) | Yes/No — with evidence |
| Are there any current authority leakage violations? (Node accessing canonical state, or Core accessing runtime internals?) | List each violation |
| Is extraction technically feasible today? | Yes/No — with blockers |
| Biggest extraction blocker? | Single item |
| Do the packet types need to move to a shared contract crate? | Yes/No |
| How many Node modules currently import from canonical/? | Count |
| How many canonical/ modules currently import from Node? | Count |
| Recommended crate boundary? | `librarian-core`, `librarian-node`, `librarian-contracts` |
| Recommended ADR decision? | Model B (workspace separation) / Model C (distributed services) — with rationale |

---

## Deliverable

Return:
1. Completed audit tables (Audit 1-4)
2. List of any authority leakage violations found (with file paths and line numbers)
3. Extraction estimate: "This will take X file moves, Y shared dependency updates, Z test updates"
4. Recommended ADR decision based on audit evidence

Do not modify any files. Report only.
