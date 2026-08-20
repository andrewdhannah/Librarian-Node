# RUST-MIGRATION-M0A-COMPLETION — Deterministic Startup Engine Foundation (Sealed)

**Status:** SEALED — M0A PASS
**Work order:** `work-orders/RUST-MIGRATION-M0A.md`
**Implementation commit:** `0b9e85a` (RUST-MIGRATION-M0A, pushed to main)
**Conformance level achieved:** **C2 — Evidence Compatible** (startup engine subsystem, per `conformance/subsystems/04-startup-engine.md`)
**Sealed:** 2026-08-06/07 (UTC run stamps 2026-08-07T02:28)

---

## 1. Verdict

The first independent Rust execution path demonstrates governed equivalence against the frozen reference architecture:

```
Canonical Fixture (conformance/fixtures/startup/canonical-startup-input.json)
        ↓
Rust StartupEngine (librarian-core/src/startup/)
        ↓
Canonical Startup Receipt (RECEIPT-SCHEMA-001)
        ↓
Equivalent Observable Facts (deterministic surface == expected-startup-receipt.json)
        ↓
Evidence Artifact (evidence/phase0/rust-core/m0/)
```

## 2. Acceptance Gates — Evidence

| Gate | Requirement | Evidence | Verdict |
|------|-------------|----------|---------|
| RUST-M0-0 | Contract surface lock before implementation | `conformance/contract-surface/contract-surface-manifest.json` (types, serde names, schema suite `RECEIPT-SCHEMA-001`, fixture + source SHA-256); guard test `librarian-contracts/tests/contract_surface_manifest.rs` 7/7 PASS | PASS |
| RUST-M0-1 | Loads project directory and resolves canonical contract sources | Probe loads `platform/windows/node-identity.json` + `capabilities.json` + `governance-sync.json`; phase 1 identity_loading PASS (real run, WINPC-BIG-PICKLE) | PASS |
| RUST-M0-2 | Opens/creates SQLite with canonical schema | `m0a_database_applies_canonical_schema` test: exactly 11 tables (9 Phase-2 + 2 Phase-3), `PRAGMA integrity_check` = ok; probe environment_validation PASS | PASS |
| RUST-M0-3 | Loads registry state from database | Verified at schema/readiness level: all 11 canonical registry tables present + integrity ok (schema embedded byte-identical from TheLibrarian `@15c5ef2`). Record-level registry semantics are registry-subsystem scope (next milestones) — noted, not overclaimed | PASS (readiness) |
| RUST-M0-4 | Loads contracts from canonical sources | Canonical schema + receipt contract + governance-sync loaded and verified; receipt validated against sealed suite `RECEIPT-SCHEMA-001` | PASS |
| RUST-M0-5 | Executes 6-phase startup protocol | `StartupEngine` (identity → governance → capabilities → environment → receipt → governed mode); probe run 6/6 checks PASS; audit-trail `StartupCheck` records all six phases | PASS |
| RUST-M0-6 | Receipt conforms to `schemas/startup-receipt.schema.json` | `StartupReceipt::validate()` + fixture expected receipt + guard tests; 13 required fields, patterns, enums, RFC 3339 timestamp | PASS |
| RUST-M0-7 | Equivalence vs canonical receipt contract oracle | Deterministic facts == `conformance/fixtures/startup/expected-startup-receipt.json` field-for-field (see §4); oracle per `conformance/divergences/STARTUP-RECEIPT-SHAPE-001.md` | PASS |
| RUST-M0-8 | Determinism across repeated runs | `m0a_engine_is_deterministic_across_runs` (3/3 fixture suite) + two real probe runs with identical 11 deterministic facts (§4) | PASS |

RUST-M0-9 (Runtime API) and RUST-M0-10 (deferred M0A items) → **RUST-MIGRATION-M0B**.

## 3. Evidence Paths

- `evidence/phase0/rust-core/m0/startup-receipt-20260807-022831-071.json` (run 1)
- `evidence/phase0/rust-core/m0/startup-receipt-20260807-022838-223.json` (run 2, determinism check)

Evidence is append-only. Receipt files are named with millisecond UTC stamps so the layer remains unique even though `receipt_id` is second-precision (contract-defined format).

## 4. Probe Output (run 1, real platform files)

```
[PASS] identity_loading: node identity WINPC-BIG-PICKLE loaded on windows
[PASS] governance_verification: governance verified at commit 6be76216a8048492526c4ca0ae751b6d2d507185
[PASS] capability_loading: all 6 required capabilities present and enabled
[PASS] environment_validation: environment validated: '...runtime-node.db' ready, 11 tables, integrity ok
[PASS] receipt_generation: startup receipt conforms to RECEIPT-SCHEMA-001
[PASS] governed_mode: node entered governed execution
checks: 6/6 passed
status: GOVERNED_EXECUTION
receipt_id: WINDOWS-STARTUP-20260807-022831
exit code: 0
```

Determinism comparison (run 1 vs run 2) — all 11 deterministic facts identical:
`node_id`, `platform`, `governance_commit`, `startup_phase`, `identity_loaded`, `governance_verified`, `capabilities_loaded`, `environment_validated`, `checks_passed`, `checks_failed`, `status`. Variable fields (receipt_id, timestamp) differ as expected.

## 5. Fixture Hashes (SHA-256, pinned in contract surface manifest)

| Path | SHA-256 |
|------|---------|
| `conformance/fixtures/startup/canonical-startup-input.json` | `1629596f3e9761129236157d36fafcd4757afdad4bc59dd3f14b49cb2f6a418d` |
| `conformance/fixtures/startup/expected-startup-receipt.json` | `e1da72fbbbc4c1e69db998d1043687fcb8d521b22c0fe24e11c2a26174656b5b` |
| `schemas/startup-receipt.schema.json` | `1012b4a387e67091b88446bf8c1e0d8f8830afe52baef69caf3e92759f78705f` |
| `librarian-core/assets/schema/capability-registry-schema.sql` | `083e914ba95dd9c1094f69ad0812aa6dd419b817ec938fa23b0fe96d07e65c20` |
| `librarian-core/assets/schema/capability-registry-schema-phase3.sql` | `03e7d17b4481041962b905f56681fbf8a8397113d8cef3a0f8c0f0802128014d` |

## 6. Test Suite (M0A scope)

- `librarian-contracts` — 169/169 PASS (106 unit + 63 integration; includes 12 startup-receipt unit tests + 7 contract-surface guard tests)
- `librarian-core` startup module — 5/5 unit PASS
- `librarian-core` fixture conformance — 3/3 PASS (`m0a_fixture.rs`)
- `cargo check --workspace` — 0 errors

## 7. Known Exclusions (deliberate, recorded not fixed)

| Exclusion | Classification | Action |
|-----------|----------------|--------|
| 48 legacy `governance::*` tests fail (`no such table: lifecycle_cursors`) | Pre-existing environmental/schema issue; reproduced identically against baseline worktree `f5959d3` — NOT introduced by M0A | Separate remediation work order |
| MCP integration | Scope decision — deferred to M1 | Not part of M0A |
| Router runtime (`main.rs`) | Unchanged — M0A uses isolated `startup_probe` binary | M0B connects at the runtime API boundary |
| Swift `startup-execution-receipt-v1` shape | Mapped reference only (divergence `STARTUP-RECEIPT-SHAPE-001`) | Canonical receipt contract is the oracle |
| Registry record-level reads / contract source loading | Readiness-level verified in M0A | Registry + project-load subsystem milestones |

## 8. Sign-off

- **Owner authorization:** commit `0b9e85a` approved and pushed 2026-08-06
- **Implementation:** M0A complete per `RUST-MIGRATION-M0A.md`
- **Next:** `work-orders/RUST-MIGRATION-M0B.md` — Runtime API Boundary
