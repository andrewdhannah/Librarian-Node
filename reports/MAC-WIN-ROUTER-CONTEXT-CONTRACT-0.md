# MAC/WIN-ROUTER-CONTEXT-CONTRACT-0 — Sprint Report

## Context Route Contract — Non-Production Draft

> **Non-production contract only.**
> No production router behavior changed. No GPU/RDMA/KV-cache acceleration claims.

---

## 1. Starting State

| Item | Value |
|------|-------|
| Starting HEAD | `f3d2041` |
| Pre-existing untracked files | 9 (WIN-RUNTIME-QUALIFICATION-1 artifacts — untouched) |

---

## 2. What Was Created

### Contract Specification
`docs/contracts/context-route-contract.md` — Draft v0.1

Defines the `context_route` object shape with:
- 14 required fields
- 6 controlled enumerations (workload_type, context_route, freshness_state, provenance_state, governance_outcome, runtime_profile)
- 13 contract invariants
- Receipt-like summary structure

### 10 Fixture Cases

| # | File | Workload | Route | Governance | Purpose |
|---|------|----------|-------|------------|---------|
| 1 | `sprint-planning.json` | sprint_planning | compressed_recall_packet | safe | Recall packet for planning context |
| 2 | `sprint-closeout.json` | sprint_closeout | canonical_evidence_read | safe | Fresh git/test evidence |
| 3 | `receipt-generation.json` | receipt_generation | canonical_evidence_read | safe | Canonical provenance-verified evidence |
| 4 | `agent-handoff.json` | agent_handoff | compressed_recall_packet | safe | Complete state snapshot for continuity |
| 5 | `long-session-continuation.json` | long_session_continuation | ram_cache | warning | Stale cache accepted for high-reuse workload |
| 6 | `runtime-node-qualification.json` | runtime_node_qualification | canonical_evidence_read | safe | Live node status verification |
| 7 | `ui-review-design.json` | ui_review_or_design_planning | ram_cache | warning | Stale design planning tolerated |
| 8 | `blocked-route.json` | receipt_generation | ram_cache | blocked | Weak provenance blocked for receipt |
| 9 | `performance-sacrificed.json` | sprint_closeout | canonical_evidence_read | safe | Slower route for evidence quality |
| 10 | `future-stronger-node.json` | long_session_continuation | ram_cache | safe | Scalability test, no GPU claims |

### Contract Tests
`scripts/tests/test-context-route-contract.py` — 413 assertions

Tests validate:
1. Required fields exist
2. Controlled enum values enforced
3. `route_id` non-empty
4. `contract_version` = `"0.1"`
5. `estimated_latency_ms` numeric and >= 0
6. `alternatives_rejected` structured
7. `reason_selected` non-empty
8. `receipt_summary` present with valid risk
9. Blocked outcome consistency
10. Receipt generation provenance rules
11. Sprint closeout freshness rules
12. Runtime qualification staleness rules
13. Performance sacrifice explanation
14. Future node GPU/RDMA/KV-claim avoidance
15. All 10 fixtures validate successfully

---

## 3. Test Results

```
Running contract tests against 10 fixtures...

Results: 413/413 passed, 0 failed
```

### Test Command
```bash
python scripts/tests/test-context-route-contract.py
```

---

## 4. Contract Design Rationale

### Why This Shape?
The `context_route` object is designed to:
- **Capture workload intent** — the router knows what kind of Librarian work is being done
- **Record governance decisions** — every route choice includes freshness/provenance state
- **Explain reasoning** — `reason_selected` and `alternatives_rejected` provide full traceability
- **Support receipt generation** — `receipt_summary` provides audit-ready output
- **Scale to future hardware** — `selected_runtime_profile` accommodates stronger nodes without GPU claims

### Why Graduated Governance?
The prior sprint (SIM-0) showed that hard-blocking stale cache causes 55% regressions. The graduated model:
- `safe` — no concerns
- `warning` — acceptable for tolerant workloads
- `requires_revalidation` — must refresh before use
- `blocked` — hard stop for critical workloads

This preserves governance while allowing workload-specific flexibility.

### Why Not Production Yet?
The contract defines a **future interface**. Production integration requires:
- Real hardware timing measurements (currently synthetic)
- Router middleware that produces `context_route` objects
- Receipt storage and audit pipeline
- Workload type detection (currently assumed per scenario)

---

## 5. Explicit Non-Production Statement

This sprint does **not**:
- Change production router behavior
- Wire into live runtime execution
- Add a production cache
- Implement GPU/RDMA/KV-cache behavior
- Claim DualPath implementation
- Alter model execution
- Change existing runtime-node HTTP behavior

All artifacts are contract definitions, fixtures, and tests.

---

## 6. Fixture Coverage Analysis

| Workload Type | Fixture | Expected Route | Governance | Evidence |
|--------------|---------|---------------|------------|----------|
| sprint_planning | ✅ | compressed_recall_packet | safe | allows stale |
| sprint_closeout | ✅ | canonical_evidence_read | safe | requires fresh |
| receipt_generation | ✅ (×2) | canonical_evidence_read / ram_cache(blocked) | safe / blocked | requires canonical |
| validation | ❌ | — | — | — |
| code_patch_preparation | ❌ | — | — | — |
| agent_handoff | ✅ | compressed_recall_packet | safe | allows stale |
| long_session_continuation | ✅ | ram_cache | warning | allows stale |
| runtime_node_qualification | ✅ | canonical_evidence_read | safe | requires canonical |
| ui_review_or_design_planning | ✅ | ram_cache | warning | allows stale |

**Note:** `validation` and `code_patch_preparation` workload types are defined in the contract enum but do not have dedicated fixtures. These can be added in future iterations if needed.

---

## 7. Recommendation

### Next Sprint: MAC/WIN-ROUTER-CONTEXT-MEASURE-1

The contract currently relies on **synthetic latency assumptions**. Before wiring into router behavior, collect real timing data:

**Purpose:** Collect real RAM/SSD/LAN/runtime-node/recompute/recall-packet timing data from current hardware profiles.

**Why measurement first:**
- The contract's `estimated_latency_ms` values are based on configurable assumptions, not real hardware
- Real measurements will validate or refute the simulator's latency model
- A measurement sprint produces grounded numbers for future contract fixtures

**What it would produce:**
- Real timing data for each context route on current Mac and Windows hardware
- LAN latency/jitter measurements under stable and unstable conditions
- RAM vs SSD vs recall-packet throughput benchmarks
- Updated contract fixtures with measured (not assumed) latency values

---

## 8. Closeout

| Item | Value |
|------|-------|
| Starting HEAD | `f3d2041` |
| Final HEAD | `f3d2041` (committed separately) |
| Test command | `python scripts/tests/test-context-route-contract.py` |
| Test result | 413/413 passed, 0 failed |
| Production router changed? | No |
| GPU/RDMA/KV claims avoided? | Yes |
