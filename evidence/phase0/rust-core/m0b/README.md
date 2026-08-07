# M0B evidence — Runtime API live validation (RUST-MIGRATION-M0B)

Evidence from the live router run with governed startup (`librarian-node`
binary, `--port 9140`, node dir `platform/windows`, governance sync
`G:\OpenWork\runtime-node\governance-sync.json`).

| File | Run | Notes |
|------|-----|-------|
| `startup-receipt-20260807-030137-400.json` | First live router run | Governed startup succeeded; process aborted at bind by a legacy `GET /health` route overlap in the ROUTER-RUST-HARDEN-1 router. Fixed by removing the weaker legacy liveness handler (`/health` now = governed availability per RUNTIME-API-CONTRACT-001 §3.1; backend liveness remains at `/backend/health`). |
| `startup-receipt-20260807-030256-622.json` | Canonical live run | Router served all three endpoints; `GET /runtime/receipt` returned this exact receipt (same `receipt_id`, same bytes — no regeneration, §4.3). |

Live query results (2026-08-07T03:44:43Z):

- `GET /health` → `200` `{"node_id":"WINPC-BIG-PICKLE","runtime_state":"SERVABLE_RUNTIME","health":"ok","observed_at":"..."}`
- `GET /runtime/status` → `200` with `startup_receipt_id: WINDOWS-STARTUP-20260807-030256`, `startup_status: GOVERNED_EXECUTION`, `checks_passed: 6`, `checks_failed: 0` — deterministic fields match the M0A evidence receipts field-for-field (contract §4.2)
- `GET /runtime/receipt` → `200` — exact sealed receipt, unwrapped
- `POST /runtime/status`, `POST /health` → `405` with `Allow: GET,HEAD`
- `GET /nope` → `404`

Gate coverage: RUST-M0B-2 (read-only projections), RUST-M0B-3
(STARTUP_COMPLETE → SERVABLE_RUNTIME enforced pre-bind; `STARTUP_FAILED`
exits), RUST-M0B-4 (response conformance), RUST-M0B-5 (no startup through the
API — handlers are read-only projections over the sealed outcome), RUST-M0B-6
(evidence-backed: the served receipt is the sealed evidence artifact).

Supporting suite: `librarian-node/tests/m0b_runtime_api.rs` (8 tests).
