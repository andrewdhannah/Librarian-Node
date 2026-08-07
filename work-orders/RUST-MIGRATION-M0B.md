# RUST-MIGRATION-M0B — Runtime API Boundary

**Status:** DRAFT (for owner review)
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
- Runtime API endpoints over the existing router:
  - `GET /health` — liveness (200 when runtime is up; reflects startup outcome)
  - `GET /status` — node status (identity, state, uptime, last state change)
  - `GET /receipt/latest` (or `/startup/receipt`) — the canonical startup receipt (RECEIPT-SCHEMA-001)
- Receipt served from memory (sealed at startup), NOT re-executed per request
- Conformance: endpoint responses carry the same deterministic facts as the M0A evidence receipts
- Exit behavior: process refuses to serve when startup fails (exit 1 before bind), consistent with probe semantics

## Scope (M0B DOES NOT)

- MCP integration (M1)
- Scheduler / work compiler / node execution (runtime services, M2+)
- Registry record-level reads (registry subsystem)
- UI, agents, networking beyond the existing local router

## Decisions Required (owner)

| Decision | Options | Recommendation |
|----------|---------|----------------|
| Endpoint naming | `/health`, `/status`, `/receipt/latest` vs alternatives | Fixed by `RUNTIME-API-CONTRACT-001.md` (created before implementation) |
| Receipt history | In-memory latest only vs persistence to evidence dir | M0B: in-memory latest + evidence file already written by probe; history = M2+ custody chain |

## Acceptance Gates

| Gate | Scope | Verification |
|------|-------|--------------|
| RUST-M0-9 | M0B | Runtime API exposed: health/status/receipt query endpoints live on the existing router |
| RUST-M0-10 | M0B | Node boundary serves the sealed M0A startup state (receipt + status) without re-executing the protocol; deterministic facts match the M0A evidence receipts |
| RUST-M0B-1 | M0B | Process refuses to serve on startup failure (exit 1 pre-bind; consistent with M0A probe semantics) |
| RUST-M0B-2 | M0B | API responses conform to canonical receipt contract (RECEIPT-SCHEMA-001) and `RUNTIME-API-CONTRACT-001.md` |
| RUST-M0B-3 | M0B | State transition boundary enforced: `STARTUP_COMPLETE` → `SERVABLE_RUNTIME`; no serving before the transition; `/status` reports the boundary state |

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
