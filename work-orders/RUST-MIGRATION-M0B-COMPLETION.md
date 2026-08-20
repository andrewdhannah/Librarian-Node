# RUST-MIGRATION-M0B-COMPLETION — Governed Runtime Exposure Boundary (Sealed)

**Status:** SEALED — M0B PASS
**Work order:** `work-orders/RUST-MIGRATION-M0B.md`
**Implementation commit:** `c60f4df` (RUST-MIGRATION-M0B, pushed to main)
**Conformance level achieved:** **C2 — Evidence Compatible** (runtime observation boundary; same level as M0A — C3/C4 not claimed)
**Sealed:** 2026-08-07 (live verification run stamps 2026-08-07T03:02, queries 2026-08-07T03:44)

---

## 1. Verdict

**M0B PASS — Governed Runtime Exposure Boundary Established.**

M0A proved the Rust node can perform governed startup equivalently. M0B proves a second, distinct property: **the Rust node can expose governed state without becoming an authority source.** The runtime API is a projection layer, not a control layer — it **exposes evidence-backed runtime state**; it does not create a new authority path.

```
librarian-node/src/startup.rs (single startup adapter)
        |
   +----+----+
   |         |
   v         v
startup_probe   router runtime (main.rs, pre-bind)
   |         |
   +----+----+
        |
        v
sealed StartupOutcome (in-memory, immutable)
        |
        v
runtime_api (read-only projections: /health, /runtime/status, /runtime/receipt)
        |
        v
observation only (no write path, no authority, no lifecycle mutation)
```

## 2. Acceptance Gates — Evidence

| Gate | Evidence | Verdict |
|------|----------|---------|
| Contract canonicalized | `contracts/runtime-api/RUNTIME-API-CONTRACT-001.md` — **Status: Canonical**, suite `RUNTIME-API-SCHEMA-001`, owner-approved lock 2026-08-06 with five refinements folded in (§2.4 observational lifecycle, surface-before-code, `observed_at`-only variability, §3.1 governed-availability `/health`, §4.3 no-regeneration receipt) | PASS |
| Surface lock updated | `conformance/contract-surface/contract-surface-manifest.json` re-baselined (`RuntimeLifecycleState` + re-pinned source hashes) BEFORE endpoint code; drift guard `librarian-contracts/tests/contract_surface_manifest.rs` 8/8 PASS | PASS |
| Lifecycle state derived | `RuntimeApiState::from_outcome` — lifecycle is derived from the sealed receipt (`GOVERNED_EXECUTION` → `SERVABLE_RUNTIME`), stored immutably, no setter; contract §2.4: `RuntimeLifecycleState` does not authorize transitions | PASS |
| Startup precedes serving | `main.rs` runs `run_node_startup` before any bind; fail-closed: non-`GOVERNED_EXECUTION` receipt → exit 1 pre-bind (RUST-M0B-3) | PASS |
| Failure containment | Defensive branch live-tested: `STARTUP_FAILED` state returns 503 on all three endpoints (`m0b_failed_startup_is_never_servable`); in the real binary the process exits pre-bind before any request can arrive | PASS |
| Read-only projection | Handlers read `Arc<StartupOutcome>`; no write path; POST/PUT/DELETE/PATCH → 405 with `Allow: GET,HEAD` (live + test); no request path invokes the startup engine (RUST-M0B-5) | PASS |
| Receipt integrity | `GET /runtime/receipt` serves the exact sealed receipt — parsed-JSON equality with the sealed receipt AND with the evidence artifact (one evidence event, no regeneration/normalization/mutation; `m0b_receipt_returns_exact_sealed_receipt` + `m0b_evidence_receipt_matches_api_receipt`) | PASS |
| Health semantics | `GET /health` = governed availability only: 200 `ok` iff `SERVABLE_RUNTIME`; legacy weaker liveness handler removed from ROUTER-RUST-HARDEN-1; backend liveness remains at `/backend/health` (contract §3.1) | PASS |
| Shared startup path | `librarian-node/src/startup.rs` single adapter consumed by both `startup_probe` and the router — the probe cannot drift into a separate "reference" implementation | PASS |

## 3. Evidence Paths

- `evidence/phase0/rust-core/m0b/README.md` — run provenance + live query results
- `evidence/phase0/rust-core/m0b/startup-receipt-20260807-030137-400.json` — first live router run (governed startup succeeded; process aborted at bind by legacy `/health` route overlap, fixed)
- `evidence/phase0/rust-core/m0b/startup-receipt-20260807-030256-622.json` — canonical live run; the exact receipt served by `GET /runtime/receipt`

Evidence is append-only. Receipt files are named with millisecond UTC stamps (receipt_id is second-precision per the contract-defined format).

## 4. Live Router Verification (real platform files)

Run: `librarian-node.exe --port 9140 --node-dir platform\windows --governance-sync G:\OpenWork\runtime-node\governance-sync.json --capability-db <tmp> --evidence-dir <tmp> --platform windows --governance-commit 6be76216a8048492526c4ca0ae751b6d2d507185`

| Query | Result |
|-------|--------|
| `GET /health` | `200` — `{"node_id":"WINPC-BIG-PICKLE","runtime_state":"SERVABLE_RUNTIME","health":"ok","observed_at":"2026-08-07T03:44:43Z"}` |
| `GET /runtime/status` | `200` — `startup_receipt_id: WINDOWS-STARTUP-20260807-030256`, `governance_commit: 6be76216a8...d507185`, `startup_status: GOVERNED_EXECUTION`, `checks_passed: 6`, `checks_failed: 0` — deterministic fields match M0A evidence receipts field-for-field |
| `GET /runtime/receipt` | `200` — exact sealed receipt, unwrapped (13 fields, same `receipt_id`/`timestamp`) |
| `POST /runtime/status`, `POST /health` | `405` — `Allow: GET,HEAD` |
| `GET /nope` | `404` |

## 5. Contract / Surface Hashes (SHA-256)

| Path | SHA-256 |
|------|---------|
| `contracts/runtime-api/RUNTIME-API-CONTRACT-001.md` | `daa1e5405d39c8e8c9050fcb0f83ad62c5200742d1baec92251fc7e02b956d3d` |
| `conformance/contract-surface/contract-surface-manifest.json` | `0c96510c9417f994abeb95c97d74fc243d0a8fc535cebcd5529ea80d2d68e762` |
| `evidence/phase0/rust-core/m0b/startup-receipt-20260807-030137-400.json` | `366774613eede5d0c92a3aea971d26e923545991c8302c0f9ac77f6787452463` |
| `evidence/phase0/rust-core/m0b/startup-receipt-20260807-030256-622.json` | `dfe6e5e0401b3245610c13a47a28bf528a60700498159960c5312634d3eb3443` |

## 6. Test Suite (M0B scope)

- `librarian-contracts` — 172/172 PASS (108 unit + 64 integration; includes 14 startup-receipt unit tests, 2 `RuntimeLifecycleState` unit tests + 8 contract-surface guard tests)
- `librarian-core` startup module — 5/5 unit PASS (M0A regression)
- `librarian-core` fixture conformance — 3/3 PASS (`m0a_fixture.rs`, M0A regression)
- `librarian-node` M0B integration — 8/8 PASS (`tests/m0b_runtime_api.rs`)
- Probe regression (real platform files) — 6/6 PASS, GOVERNED_EXECUTION, exit 0 (now on the shared adapter)
- `cargo check --workspace` — 0 errors

## 7. Explicit Non-Goals (deferred, recorded not fixed)

| Non-goal | Classification | Action |
|----------|----------------|--------|
| MCP integration | Scope decision | Deferred to M1 |
| Scheduler / concurrency / distributed node execution | Scope decision | Deferred to M2+ |
| Capability mutation through the API | Contract prohibition (§5.2 — MUST NOT) | Never part of the observation boundary |
| Registry record-level semantics (ownership, qualification, authority relationships) | Registry subsystem scope | M1 |
| SQLite driver reconsideration (rusqlite today; sqlx-style async driver) | Deferred until concurrency requirements exist (scheduler / concurrent work execution / distributed coordination) | Revisit at M2 planning |

## 8. Known Issues (carried forward, classified)

| Issue | Classification | Action |
|-------|----------------|--------|
| 48 legacy `governance::*` tests fail (`no such table: lifecycle_cursors`) | Pre-existing baseline issue; reproduced identically against baseline worktree — NOT introduced by M0B | Separate remediation scope |
| `librarian-node` test-target compile errors (custody `ProvenanceQuery`/`RetentionPolicy` field drift in test-gated code) | Pre-existing baseline issue; verified at HEAD via stash — NOT introduced by M0B | Separate remediation scope |

Neither issue blocks this migration increment or the next conformance target; the M0B path is fully exercised by the passing suites above.

## 9. Sign-off

- **Owner authorization:** contract lock 2026-08-06; commit `c60f4df` approved and pushed 2026-08-07
- **Implementation:** M0B complete per `RUST-MIGRATION-M0B.md`; phase boundary clean (working tree clean at `c60f4df`)
- **Next:** `work-orders/RUST-MIGRATION-M1.md` — Capability / Registry Semantics (record-level ownership, qualification, authority relationships)
