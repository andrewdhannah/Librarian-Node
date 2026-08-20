# RUST-MIGRATION-M0B — Runtime API Boundary

**Status:** ACTIVE — contract LOCKED as Canonical (owner approval 2026-08-06); implementation COMPLETE, awaiting owner commit approval
**Epic:** EPIC-RUST-MIGRATION-1
**Phase:** 0B — Runtime API exposure of the validated startup engine
**Predecessor:** `RUST-MIGRATION-M0A-COMPLETION.md` (SEALED, C2 Evidence Compatible, commit `0b9e85a`)

---

## Objective

Expose the M0A-validated runtime through the node boundary. M0A proved the deterministic startup engine produces an equivalent canonical receipt; M0B makes that validated state queryable by operators and consumers via the existing hardened router (`librarian-node/src/main.rs`, ROUTER-RUST-HARDEN-1, axum on port 9130).

```
M0A startup engine (sealed)
        ↓  loads at process start
Runtime API (health / status / receipt query)
        ↓
HTTP boundary (existing router, port 9130)
```

## Decisions (Owner-Approved 2026-08-06)

| Decision | Result |
|----------|--------|
| SQLite at the API boundary | **rusqlite stays for M0B.** M0B is not introducing a concurrent service workload; it is exposing already-produced startup state. Boundary shape: HTTP/API request → read validated state → return evidence/status. NOT: request → start runtime workflow. Revisit when scheduler, concurrent work execution, or distributed node coordination arrive. |
| Endpoint contracts | **Create `contracts/runtime-api/RUNTIME-API-CONTRACT-001.md` before implementation**: endpoint names, request shape, response schema, failure behavior, authority restrictions. |
| API authority | **The API reports runtime state; it does not become an authority source.** Invariant: Observation ≠ Authority (matches Registry ≠ Authority, Recovery ≠ Authorization). |
| Router integration | **Module integration, not fork.** Target shape: `librarian-node` with `router` / `startup` / `runtime_api` / `evidence` — no parallel execution paths (`router-version` + `startup-version` is forbidden). |
| State transition boundary | **STARTUP_COMPLETE → SERVABLE_RUNTIME.** A node is available because startup succeeded AND receipt exists AND governance state is valid AND runtime state can be observed — not because the process exists. Prevents "service is running" being confused with "service is trustworthy." |

## Scope (M0B DOES)

- Node process startup: invoke `StartupEngine` before serving; hold the sealed `StartupOutcome` in memory
- Enforce the state boundary: `STARTUP_COMPLETE` → `SERVABLE_RUNTIME`; the router binds/serves only after the transition
- Runtime API endpoints over the existing router (names fixed by the locked contract):
  - `GET /health` — governed-availability health (200 `ok` only in `SERVABLE_RUNTIME`; NOT process liveness)
  - `GET /runtime/status` — governed availability status with provenance (node_id, runtime_state, startup_receipt_id, governance_commit, startup_status, checks, observed_at)
  - `GET /runtime/receipt` — the canonical startup receipt (RECEIPT-SCHEMA-001), served exactly as sealed
- Receipt served from memory (sealed at startup), NOT re-executed per request
- Conformance: endpoint responses carry the same deterministic facts as the M0A evidence receipts
- Exit behavior: process refuses to serve when startup fails (exit 1 before bind), consistent with probe semantics

## Scope (M0B DOES NOT)

- MCP integration (M1)
- Scheduler / work compiler / node execution (runtime services, M2+)
- Registry record-level reads (registry subsystem)
- UI, agents, networking beyond the existing local router

## Contract Lock (owner-approved 2026-08-06)

`contracts/runtime-api/RUNTIME-API-CONTRACT-001.md` — **Status: Canonical**, suite `RUNTIME-API-SCHEMA-001`. Refinements folded in at lock:

1. **`RuntimeLifecycleState` is observational, not authoritative** — a contract type derived from runtime state; it does not authorize transitions; lifecycle transitions remain owned by the startup/runtime state machine (contract §2.4).
2. **Surface manifest updated before implementation** — `RuntimeLifecycleState` added to the M0-0 contract surface manifest + drift guard (8/8 PASS) before any endpoint code.
3. **`observed_at` is the only variable response field** — two status queries can differ only in observation metadata (contract §4.2).
4. **`/health` = governed availability, not process liveness** — healthy only in `SERVABLE_RUNTIME` (contract §3.1). Implemented by removing the legacy weaker liveness handler from the ROUTER-RUST-HARDEN-1 router; backend liveness remains at `/backend/health`.
5. **`/runtime/receipt` returns the receipt it observed** — no regeneration, transformation, or normalization; a regenerated receipt would create a second evidence event (contract §4.3).

## Implementation status

- `librarian-node/src/startup.rs` — node startup adapter (resolve inputs → engine → seal → evidence), shared by router and probe (no parallel path)
- `librarian-node/src/runtime_api.rs` — `RuntimeApiState` (sealed, lifecycle derived), read-only handlers, router builder
- `librarian-node/src/main.rs` — governed startup before bind (fail-closed exit 1), runtime API merged as a module
- `librarian-node/src/bin/startup_probe.rs` — refactored onto the shared adapter (M0A CLI unchanged; 6/6 PASS regression)
- `librarian-node/tests/m0b_runtime_api.rs` — 8/8 PASS
- Live router run: `/health` 200 ok, `/runtime/status` 200 (receipt `WINDOWS-STARTUP-20260807-030256`, facts match M0A evidence), `/runtime/receipt` 200 (exact sealed receipt), POST → 405 `Allow: GET,HEAD`, unknown → 404
- Evidence: `evidence/phase0/rust-core/m0b/` (README + 2 receipts)
- Pre-existing, unchanged: 48 legacy `governance::*` failures + librarian-node test-target custody compile errors (verified at HEAD via stash; not M0B regressions)

## Decisions Resolved (locked with contract, 2026-08-06)

| Decision | Resolution |
|----------|------------|
| Endpoint naming | `/health`, `/runtime/status`, `/runtime/receipt` — fixed by `RUNTIME-API-CONTRACT-001.md` §3 |
| Receipt history | In-memory latest (sealed) + evidence file written at startup; history = M2+ custody chain |

## Acceptance Gates

Owner-approved final gate list (supersedes RUST-M0-9/RUST-M0-10, which are absorbed as follows: API exposure → M0B-1/M0B-4; serving sealed state without re-execution → M0B-2/M0B-5/M0B-6).

| Gate | Verification |
|------|--------------|
| RUST-M0B-1 | Runtime API contract implemented (`contracts/runtime-api/RUNTIME-API-CONTRACT-001.md`, suite `RUNTIME-API-SCHEMA-001`) |
| RUST-M0B-2 | Read-only runtime projections: health/status/receipt handlers read sealed `StartupOutcome`; no write path |
| RUST-M0B-3 | `STARTUP_COMPLETE → SERVABLE_RUNTIME` enforced: no binding/serving before the transition; `STARTUP_FAILED` exits pre-bind |
| RUST-M0B-4 | API responses conform to the runtime contract (provenance fields, deterministic facts match M0A evidence receipts, `/health` = governed-availability health, receipt served unmodified) |
| RUST-M0B-5 | No startup execution through the API: no request path invokes the startup engine; lifecycle transitions are not API-reachable |
| RUST-M0B-6 | Evidence-backed validation: responses carry the observed receipt/status (no regenerated evidence events); validated by integration tests + evidence transcripts |

Conformance target: **C2 (Evidence Compatible)** — same level as M0A; C3/C4 not claimed.

## Deliverables

1. Work order: `work-orders/RUST-MIGRATION-M0B.md` (this file)
2. Contract: `contracts/runtime-api/RUNTIME-API-CONTRACT-001.md` — endpoint names, request shape, response schema, failure behavior, authority restrictions (API reports state; it is not an authority source)
3. `librarian-node` runtime API module (health/status/receipt) + router wiring (module integration, no fork)
4. State transition: `STARTUP_COMPLETE` → `SERVABLE_RUNTIME` enforced at the node boundary
5. Integration tests: start router with sealed outcome → query endpoints → assert deterministic facts
6. Evidence: `evidence/phase0/rust-core/m0b/` (API query transcripts + receipts)
7. Completion record: `work-orders/RUST-MIGRATION-M0B-COMPLETION.md`

## Dependencies

- M0A SEALED (commit `0b9e85a`): `StartupEngine`, `StartupReceipt`, contract surface lock
- Existing router: `librarian-node/src/main.rs` (axum 0.8, tokio, tracing, clap)

## Estimated Effort

1 sprint (~1 week)

---

## Divergence Protocol

Same as M0A (`docs/architecture/RUST-CORE-CONFORMANCE-SPECIFICATION.md` §9): Rust bug → fix + regression test; Swift bug → record, do not port; contract clarification → file + re-baseline. Evidence append-only.
