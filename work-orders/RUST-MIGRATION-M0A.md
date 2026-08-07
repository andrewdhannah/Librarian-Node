# RUST-MIGRATION-M0A — Deterministic Startup Engine Foundation

**Status:** Approved
**Epic:** EPIC-RUST-MIGRATION-1
**Phase:** 0A — Core + Deterministic Startup Skeleton

---

## Objective

Implement the first Rust milestone slice: deterministic startup behavior sufficient to load canonical project state and emit an equivalent startup receipt against the frozen Swift baseline.

Per `docs/architecture/RUST-CORE-CONFORMANCE-SPECIFICATION.md` §8, the milestone diagram is:

```
Rust Runtime
Load Project
    ↓
Open SQLite
    ↓
Load Registry
    ↓
Load Contracts
    ↓
Execute Startup Protocol
    ↓
Expose Runtime API        ← M0B (deferred)
    ↓
Return Receipts
```

**M0A (this work order)** covers: Load Project → Open SQLite → Load Registry → Load Contracts → Execute Startup Protocol → Emit Startup Receipt.
**M0B (next work order)** covers: Startup Engine → Runtime API → Health/Status/Receipt Query.

Keeping the divergence domain small: if a receipt mismatch appears, it is attributable to a small, deterministic surface — not an opaque full pipeline.

---

## Baseline

Frozen references (unchanged from RUST-CORE-CONFORMANCE-1 §3):

| Reference | Role | Repository | Frozen Commit |
|-----------|------|------------|---------------|
| Git baseline | Canonical implementation | `andrewdhannah/TheLibrarian` (sprint-3, private) | `15c5ef2c1d4d4ba72af2a765f0bc4ce8bd4b16cc` |
| Rust runtime | Implementation under conformance | `andrewdhannah/Librarian-Node` | `82e3110` |
| Equivalence harness | Conformance and migration harness | `andrewdhannah/Librarian-Platform-Equivalence` | `e42a6c6` |

The Swift baseline is frozen. Any change requires a new baseline freeze and an equivalence gap analysis (spec §3).

---

## This Sprint DOES NOT

- UI
- AI / agents
- MCP
- Networking
- Scheduler, work compiler, node execution (runtime services)
- Runtime API exposure (health/status/receipt query — M0B)
- Concurrency beyond what the startup sequence requires
- Porting Swift code; architectural decisions outside the conformance contract

## This Sprint DOES

- Load a project directory and resolve canonical contract sources
- Open/create the SQLite database with schema conforming to the canonical schema
- Load registry state from the database
- Load contracts from canonical sources
- Execute the 6-phase startup protocol per `contracts/startup/STARTUP-PROTOCOL.md`
- Produce a startup receipt conforming to `schemas/startup-receipt.schema.json`
- Demonstrate receipt equivalence against the Swift reference and determinism across runs

---

## Ownership Split (unchanged)

| Crate | Owns |
|-------|------|
| `librarian-contracts` | Canonical structures — identities, receipts, evidence schemas, lifecycle enums, governance objects |
| `librarian-core` | Deterministic mechanisms — startup protocol, validation, registry loading, state transitions, receipt construction |
| `librarian-node` | Runtime host — executable, configuration, filesystem, process lifecycle, API exposure (M0B) |

Do not add a new crate unless a real boundary appears.

---

## Decisions (Owner-Approved 2026-08-06)

| Decision | Result |
|----------|--------|
| SQLite crate | **rusqlite** — M0A is a deterministic bootstrap sequence (synchronous, transaction-like), not a service runtime. sqlx deferred to M0B+ when async runtime services, concurrent node operations, network-facing APIs, and scheduler execution exist. |
| RUST-M0-7 receipt oracle | **Canonical receipt contract + mapped Swift facts.** Do NOT reproduce the Swift `startup-execution-receipt-v1` shape (would be a regression). Migration target: *produce the canonical startup receipt contract while preserving equivalent observable facts*. See `conformance/divergences/STARTUP-RECEIPT-SHAPE-001.md`. |
| MCP inclusion in M0A | **Deferred.** MCP consumes a validated node runtime; it does not participate in proving the runtime exists. Dependency order: M0A (core startup/contracts/SQLite/registry/receipts) → M0B (runtime API) → M1 (MCP integration) → M2 (agent/runtime orchestration). |
| RUST-M0-0 contract surface lock | **Before engine implementation.** Snapshot the librarian-contracts interop surface (types, serde names, schema suite IDs, fixture/source hashes) into `conformance/contract-surface/contract-surface-manifest.json`; a drift-guard test enforces it. Prevents accidental contract drift while implementing against the freshly-restored contracts crate. |
| Receipt types ownership | **Contract objects, not core objects.** `StartupReceipt`, `StartupPhase`, `StartupStatus`, `StartupCheck` live in `librarian-contracts` (interop artifacts). Core holds mechanism (input parsing, verification, engine). Note: receipt `startup_phase` field is the schema outcome (`complete`/`failed`), distinct from the 6-value `StartupPhase` check-step enum — documented in code and manifest. |
| M0A node entrypoint | **Separate binary** `librarian-node/src/bin/startup_probe.rs`. The router (`src/main.rs`, ROUTER-RUST-HARDEN-1) is untouched; the first production runtime artifact must not couple to HTTP/service concerns. |
| Canonical schema embedding | Byte-identical copy of `capability-registry-schema.sql` + `-phase3.sql` into `librarian-core/assets/schema/` (provenance README; `include_str!`). Runtime is self-contained and deterministic — no external file lookup at startup. |

**Fixture anchor:** before coding the startup engine, the fixtures at `conformance/fixtures/startup/` are the deterministic target:

```
Given canonical startup fixture          (canonical-startup-input.json)
When  Rust startup engine executes       (6-phase STARTUP-PROTOCOL)
Then  emitted receipt matches expected   (expected-startup-receipt.json)
```

---

## Acceptance Gates

| Gate | Scope | Verification |
|------|-------|--------------|
| RUST-M0-0 | M0A | **Contract surface lock.** Before engine implementation, snapshot the librarian-contracts interop surface (`conformance/contract-surface/contract-surface-manifest.json`: type names, fields, serde names, schema suite IDs, fixture + source hashes). Drift fails the guard test `librarian-contracts/tests/contract_surface_manifest.rs`; deliberate re-baseline requires divergence-protocol documentation. |
| RUST-M0-1 | M0A | Loads a project directory and resolves canonical contract sources |
| RUST-M0-2 | M0A | Opens/creates SQLite database with schema conforming to canonical schema |
| RUST-M0-3 | M0A | Loads registry state from database |
| RUST-M0-4 | M0A | Loads contracts from canonical sources |
| RUST-M0-5 | M0A | Executes the 6-phase startup protocol per `STARTUP-PROTOCOL.md` |
| RUST-M0-6 | M0A | Produces a startup receipt conforming to `schemas/startup-receipt.schema.json` (sealed suite `RECEIPT-SCHEMA-001`) |
| RUST-M0-7 | M0A | Startup receipt passes equivalence check against the canonical receipt contract (oracle per `conformance/divergences/STARTUP-RECEIPT-SHAPE-001.md`; deterministic facts compared to `conformance/fixtures/startup/expected-startup-receipt.json`) |
| RUST-M0-8 | M0A | Same input produces same receipt across repeated runs (determinism) |
| RUST-M0-9 | M0B | Exposes Runtime API (health/status/receipt query) — deferred |

Conformance level target for the startup engine subsystem: **C2 (Evidence Compatible)** per `conformance/subsystems/04-startup-engine.md`.

---

## Required Deliverables

### 1. Work Order

| File | Purpose |
|------|---------|
| `work-orders/RUST-MIGRATION-M0A.md` | This work order |

### 2. Contracts (`librarian-contracts`)

- Startup receipt type per `contracts/startup/STARTUP-OUTPUT-CONTRACT.md` and `schemas/startup-receipt.schema.json` (add only what does not already exist in `identity` / `receipts` / `node`)

### 3. Core (`librarian-core`)

- Startup protocol module: 6-phase sequence (identity → governance → capabilities → environment → receipt → governed mode)
- Registry loading (state from SQLite)
- State transitions and receipt construction
- Deterministic serialization (reuse `serialization` contract utilities)
- Persistence via **rusqlite** with explicit transaction boundaries

### 4. Node (`librarian-node`)

- M0A startup entrypoint: loads config, filesystem fixtures, invokes core startup, emits receipt to `evidence/phase0/rust-core/m0/`
- Exit code 0 on success; 1 on startup failure (per subsystem spec §3)

### 5. Evidence

- `evidence/phase0/rust-core/m0/startup-receipt-<run>.json` (append-only)
- Equivalence comparison vs Swift reference receipt (deterministic fields must match)
- Determinism comparison across repeated runs

### 6. Tests

- Unit + integration tests mapped to RUST-M0-1..8 and sealed suite IDs (`RECEIPT-SCHEMA-001`)
- First test is fixture-driven (migration anchor): load `conformance/fixtures/startup/canonical-startup-input.json` → execute engine → emitted receipt deterministic fields match `conformance/fixtures/startup/expected-startup-receipt.json`

### 7. Fixtures (migration anchor)

| File | Purpose |
|------|---------|
| `conformance/fixtures/startup/canonical-startup-input.json` | Canonical input: node directory (identity, governance sync, capabilities), SQLite intent, expected contract outcome |
| `conformance/fixtures/startup/expected-startup-receipt.json` | Expected canonical receipt — deterministic-field equivalence target |
| `conformance/fixtures/startup/README.md` | Field policy (deterministic vs variable) and provenance |

### 8. Divergence Record

| File | Purpose |
|------|---------|
| `conformance/divergences/STARTUP-RECEIPT-SHAPE-001.md` | Contract clarification: canonical receipt contract supersedes Swift platform-harness receipt (SH-005 / EQ-004 mapping) |

---

## Evidence Format

Startup receipt (from `schemas/startup-receipt.schema.json`):

```json
{
  "receipt_id": "<unique-receipt-id>",
  "node_id": "<node-identifier>",
  "platform": "<windows|linux|macos>",
  "governance_commit": "<commit-sha>",
  "startup_phase": "complete",
  "identity_loaded": true,
  "governance_verified": true,
  "capabilities_loaded": true,
  "environment_validated": true,
  "checks_passed": 6,
  "checks_failed": 0,
  "status": "GOVERNED_EXECUTION",
  "timestamp": "<iso-8601-timestamp>"
}
```

Equivalence per `docs/architecture/THREE-WAY-EQUIVALENCE-PROTOCOL.md`:
- **Deterministic fields (must match):** `governance_commit`, `identity_loaded`, `governance_verified`, `capabilities_loaded`, `environment_validated`, `checks_passed`, `checks_failed`, `status`
- **Variable fields (expected to differ):** `receipt_id`, `node_id`, `platform`, `timestamp`

---

### 9. Contract Surface Lock (RUST-M0-0)

| File | Purpose |
|------|---------|
| `conformance/contract-surface/contract-surface-manifest.json` | Snapshot of the contracts interop surface (types, serde names, schema suite IDs, fixture + source hashes) at implementation start |
| `librarian-contracts/tests/contract_surface_manifest.rs` | Drift-guard test — any surface change fails until the manifest is deliberately re-baselined |

---

## Implementation Status (2026-08-06)

Implemented and verified (awaiting owner commit approval):

| Item | Status | Evidence |
|------|--------|----------|
| Contract types (`StartupReceipt`, `StartupReceiptFacts`, `StartupPhase`, `StartupStatus`, `StartupCheck`) | Done | `librarian-contracts/src/node/startup.rs`; 12 unit tests PASS |
| Contract surface lock (RUST-M0-0) | Done | `conformance/contract-surface/contract-surface-manifest.json`; 7 guard tests PASS |
| Canonical schema embedded (byte-identical) | Done | `librarian-core/assets/schema/` (provenance README; SHA-256 pinned in manifest) |
| 6-phase startup engine | Done | `librarian-core/src/startup/` (mod, identity, governance, capabilities, environment, receipt, engine) |
| Fixture-driven conformance test | Done | `librarian-core/tests/m0a_fixture.rs` — 3/3 PASS (canonical input → expected receipt facts; schema creates 11 tables + integrity ok; determinism across runs) |
| Node probe binary | Done | `librarian-node/src/bin/startup_probe.rs` — real run: 6/6 PASS, `GOVERNED_EXECUTION`, exit 0 |
| Evidence emission | Done | `evidence/phase0/rust-core/m0/startup-receipt-20260807-022831-071.json` + `-022838-223.json`; deterministic facts equal across runs and match fixture expectation field-for-field |
| rusqlite dependency | Done | `rusqlite 0.31` (bundled) in `librarian-core` |
| Pre-existing condition | Noted | 48 `governance::*` legacy tests fail with `no such table: lifecycle_cursors` — reproduced identically at `f5959d3` (before M0A changes) via worktree; environmental (un-migrated test DB), out of M0A scope |

Timing note: engine + probe cold-start well under the <5 s bound; startup is single-threaded, no network.

---

## Dependencies / Predecessors

- `RUST-CORE-CONFORMANCE-1` complete — conformance spec + subsystem registry + startup engine subsystem spec (`82e3110`)
- **Workspace compile restoration — committed and pushed (`f5959d3` RUST-MIGRATION-M0A-PREREQ):**
  - `librarian-contracts/src/lib.rs`: declared all 30+ existing modules (were silently undeclared; caused ~100 E0432/E0433 errors)
  - `librarian-contracts/src/custody.rs`: added 11 missing custody contract types (`CustodyMetadata`, `ReceiptEnvelope`, `CustodyChain`, `ProvenanceQuery`, `ProvenanceResult`, `ProvenanceLink`, `ProvenanceGraph`, `IntegrityError`, `IntegrityReport`, `RetentionPolicy`, `RetentionResult`)
  - `cargo check --workspace`: 0 errors; `cargo test -p librarian-contracts`: 150/150 pass
- Canonical fixtures: `contracts/startup/*`, `schemas/startup-receipt.schema.json`, `evidence/phase0/reference-architecture/startup-receipt-{windows,linux,macos}.json`, node fixtures `platform/<os>/node-identity.json` + `capabilities.json`
- Conformance fixtures: `conformance/fixtures/startup/` (migration anchor)
- Dependency added: `rusqlite 0.31` (bundled) in `librarian-core` — offline deterministic builds

---

## Divergence Protocol

Per `docs/architecture/RUST-CORE-CONFORMANCE-SPECIFICATION.md` §9:

| Case | Determination | Action |
|------|---------------|--------|
| Rust bug | Rust behavior differs from contract and Swift | Fix Rust; add regression test |
| Swift bug | Swift behavior differs from contract and Rust | Record finding; do NOT port the bug |
| Contract clarification | Neither matches; contract ambiguous | File contract clarification; update contract; re-baseline |

Every divergence is recorded with: input, expected (contract), Swift behavior, Rust behavior, disposition.

---

## Performance Expectations (per subsystem spec §7)

- Startup completes within a bounded time (target < 5 s cold start; exact bound measured and recorded)
- No network access during startup (deterministic fast path)
- Startup is single-threaded

---

## Estimated Effort

1 sprint (~1 week)

---

## Key Files

| Path | Description |
|------|-------------|
| `docs/architecture/RUST-CORE-CONFORMANCE-SPECIFICATION.md` | Conformance contract (baseline, gates, divergence) |
| `conformance/subsystems/04-startup-engine.md` | Startup engine subsystem spec (C2 target) |
| `contracts/startup/STARTUP-PROTOCOL.md` | 6-phase startup protocol (authoritative) |
| `contracts/startup/STARTUP-OUTPUT-CONTRACT.md` | Startup receipt format |
| `contracts/startup/SESSION-IDENTITY-CONTRACT.md` | Node identity requirements |
| `schemas/startup-receipt.schema.json` | Startup receipt JSON Schema |
| `evidence/phase0/reference-architecture/startup-receipt-windows.json` | Reference receipt (equivalence oracle source) |
| `conformance/fixtures/startup/canonical-startup-input.json` | Canonical startup input fixture |
| `conformance/fixtures/startup/expected-startup-receipt.json` | Expected receipt fixture (deterministic facts) |
| `conformance/divergences/STARTUP-RECEIPT-SHAPE-001.md` | Receipt-shape contract clarification |
| `librarian-core/src/` | Startup protocol, registry loading, receipt construction |
| `librarian-node/src/` | M0A entrypoint, config, evidence emission |

---

## Governance Model

All changes follow the Librarian governance process:

**Proposal - Impact Analysis - Invariant Review - Owner Authorization - Implementation - Certification**

Evidence is append-only. State may change; evidence does not.
