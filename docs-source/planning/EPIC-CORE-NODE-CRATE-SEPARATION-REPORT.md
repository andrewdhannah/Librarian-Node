# EPIC-CORE-NODE-CRATE-SEPARATION — Completion Report

## 1. Workspace Result

**Before:**
```
rust-router/          ← single monolithic crate
  ├── src/canonical/  ← authority modules
  ├── src/            ← runtime modules
  └── tests/          ← all tests
librarian-contracts/  ← already extracted
```

**After:**
```
librarian-contracts/  ← neutral packet contracts (sealed previously)
librarian-core/       ← canonical authority (qualification, governance, provenance, routing, capability)
librarian-node/       ← runtime execution (server, process, residency, evidence, operator)
```

## 2. Gate Results Table

| Gate | Result |
|------|--------|
| G-CONTRACTS | ✅ Sealed (pre-existing) |
| G-CORE | ✅ 580+ tests pass |
| G-NODE | ✅ 85 tests pass |
| WORKSPACE-CLOSURE | ✅ Full workspace builds and tests pass |

## 3. Dependency Proof

```
librarian-contracts:
  imports: anyhow, chrono, reqwest, serde, serde_json, sha2, uuid
  forbidden: librarian-core ❌, librarian-node ❌

librarian-core:
  imports: librarian-contracts, anyhow, chrono, reqwest, rusqlite, serde, serde_json, sha2, uuid
  forbidden: librarian-node ❌

librarian-node:
  imports: librarian-contracts, anyhow, axum, chrono, clap, reqwest, rusqlite, serde, serde_json, sha2, tokio, tower, tower-http, tracing, tracing-subscriber, uuid
  forbidden: librarian-core ❌
```

**Architectural constraint verified:** Core does NOT depend on Node. Node does NOT depend on Core. Both depend only on Contracts + external crates.

## 4. Behavioral Preservation

- **`cargo build --release`** — succeeds (zero errors, only pre-existing unused-import warnings)
- **`cargo test --workspace`** — all tests pass across all 3 crates:
  - `librarian-contracts`: 56 passed, 0 failed
  - `librarian-core`: 580 lib + numerous integration tests, 0 failed
  - `librarian-node`: 71 lib + 14 integration tests, 0 failed
- **No schema changes** — all DB migrations, packet types, and protocol fields unchanged
- **No feature additions** — zero new dependencies, zero new types, zero new endpoints
- **Pre-existing warnings preserved** — unused import warnings existed before extraction

## 5. Migration Exceptions

**None.** Every file from `rust-router/` was moved to its target crate mechanically:

| Domain | Files | Target |
|--------|-------|--------|
| Canonical authority models | ~72 source files | `librarian-core/src/` |
| Runtime modules | ~23 source files | `librarian-node/src/` |
| Core integration tests | 16 test files | `librarian-core/tests/` |
| Node integration tests | 1 test file | `librarian-node/tests/` |
| Runtime UI assets | 4 asset files | `librarian-node/runtime-ui/` |

**Item worth noting:** `runtime-ui/` directory was duplicated from `rust-router/` to `librarian-node/` to satisfy `include_str!` macro paths. This is a static asset copy, not a behavior change.

## 6. Final State

```
G:\openwork\librarian-runtime-node\
├── Cargo.toml                  ← workspace: contracts, core, node
├── librarian-contracts/        ← sealed (56 tests)
├── librarian-core/             ← 580+ tests, depends on contracts only
├── librarian-node/             ← 85 tests, depends on contracts only
└── docs/planning/
    ├── EPIC-CORE-NODE-CRATE-SEPARATION.md
    └── EPIC-CORE-NODE-CRATE-SEPARATION-REPORT.md
```

**Original `rust-router/` crate has been removed.**

**Architectural boundary is now enforced at compile time:** any attempt by `librarian-core` to import from `librarian-node` (or vice versa) produces a compilation error.
