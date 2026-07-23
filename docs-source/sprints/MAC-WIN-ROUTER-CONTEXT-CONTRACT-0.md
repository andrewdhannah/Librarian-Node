# MAC/WIN-ROUTER-CONTEXT-CONTRACT-0 — Context Route Contract

## Sprint Summary
- **Sprint Type**: Contract / Design
- **Repository**: `librarian-runtime-node`
- **Starting HEAD**: `f3d2041`
- **Goal**: Define a stable `context_route` contract for The Librarian router.

## Honesty Statement
This sprint defines a **future interface only**. It does not:
- Change production router behavior
- Wire into live runtime execution
- Add a production cache
- Implement GPU/RDMA/KV-cache behavior
- Claim DualPath implementation

## Prior Sprint Context
This sprint follows `MAC/WIN-ROUTER-WORKLOAD-OPTIMIZER-1` (classified PROMOTE), which showed:
- Workload-aware routing improves decisions in 3/8 scenarios
- Scenario C stale-cache regression was resolved
- Graduated governance penalties work
- Decisions are explainable in receipt-like form

## Contract Summary
The `context_route` contract (v0.1) defines the structured object a future router may emit when deciding how to supply context for a model/runtime task.

### Object Shape
- `route_id` — unique decision identifier
- `contract_version` — `"0.1"`
- `workload_type` — 9 Librarian workload types
- `selected_context_route` — 7 context routes
- `selected_runtime_profile` — 4 hardware profiles
- `freshness_state` — 6 graduated governance states
- `provenance_state` — 5 provenance levels
- `governance_outcome` — 4 outcomes (safe/warning/revalidate/blocked)
- `estimated_latency_ms` — numeric, >= 0
- `performance_sacrificed_for_evidence` — boolean
- `reason_selected` — human-readable explanation
- `alternatives_rejected` — list of rejected routes with reasons
- `evidence_requirements` — git/test/source/stale constraints
- `receipt_summary` — label, detail, risk level

### Contract Invariants
13 invariants enforced by tests, including:
- Blocked outcome consistency
- Safe outcome consistency
- Receipt generation provenance rules
- Sprint closeout freshness rules
- Runtime qualification staleness rules
- Performance sacrifice explanation requirements
- Future node GPU/RDMA/KV-claim avoidance

## Files Created
- `docs/contracts/context-route-contract.md` — Contract specification
- `fixtures/context-route/*.json` — 10 machine-readable fixtures
- `scripts/tests/test-context-route-contract.py` — Contract tests (413 assertions)
- `reports/MAC-WIN-ROUTER-CONTEXT-CONTRACT-0.md` — Sprint report
- `docs/sprints/MAC-WIN-ROUTER-CONTEXT-CONTRACT-0.md` — This doc

## Test Results
```
413/413 passed, 0 failed
```
