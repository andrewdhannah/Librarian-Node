# MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1 — Sprint Closeout

## Sprint Details

| Field | Value |
|-------|-------|
| Sprint ID | MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1 |
| Type | Prototype / non-production |
| Status | **Complete — Promote** |
| Starting HEAD | 9b1b4b1 |
| Platform | Windows 10 (AMD64) |
| Date | 2026-06-28 |

## What Was Done

Built a non-production prototype that generates mock router decisions containing both `model_route` and `context_route` objects using measured costs from MAC/WIN-ROUTER-CONTEXT-MEASURE-1.

### Generated Decisions

- **9 workload types** — each produces a complete model_route + context_route decision
- **7 scenario cases** — receipt generation, long session, degraded node, agent handoff, sprint closeout, UI review, parallel mixed
- **176 tests pass** — contract invariants, governance rules, measured-cost requirements

### Key Decision Patterns

| Workload | Route | Latency | Why |
|----------|-------|---------|-----|
| sprint_planning | recent_turn_window | 0.30ms | Cheap local, moderate governance |
| sprint_closeout | canonical_evidence_read | 70.90ms | Governance mandate, fresh evidence |
| receipt_generation | canonical_evidence_read | 70.90ms | Governance mandate, provenance |
| validation | canonical_evidence_read | 70.90ms | Governance mandate, test results |
| code_patch_preparation | compressed_recall_packet | 4.18ms | Cheap local recall |
| agent_handoff | compressed_recall_packet | 4.17ms | Governance mandate, state snapshot |
| long_session_continuation | recent_turn_window | 0.30ms | Cheap local, high stale tolerance |
| runtime_node_qualification | canonical_evidence_read | 70.90ms | Governance mandate, live health |
| ui_review_or_design_planning | recent_turn_window | 0.30ms | Cheap local, warning governance |

### Measured Costs Used

- Git subprocess overhead: ~71ms (git status), ~55ms (git rev-parse)
- Degraded-node penalty: ~4016ms (TCP timeout)
- Recall packet local processing: ~0.50ms for 32K tokens
- Small append pipeline: ~0.80ms
- Large context pipeline: ~1.91ms for 32K tokens

## What Was NOT Changed

- Production router behavior: **UNCHANGED**
- Model execution behavior: **UNCHANGED**
- Runtime-node HTTP behavior: **UNCHANGED**
- No cache engine added
- No GPU/RDMA/KV-cache claims made

## Acceptance Criteria Check

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Prototype runs locally | PASS |
| 2 | No production router changes | PASS |
| 3 | No runtime-node HTTP changes | PASS |
| 4 | No model execution changes | PASS |
| 5 | Decisions include model_route + context_route | PASS |
| 6 | All 9 workload types covered | PASS |
| 7 | All 7 scenarios (A-G) covered | PASS |
| 8 | Measured profile data used | PASS |
| 9 | Contract compliance tested | PASS |
| 10 | Report separates prototype from production | PASS |
| 11 | Service state verified | PASS |
| 12 | No GPU/RDMA/KV-cache claims | PASS |
| 13 | Working tree documented | PASS |

## Files Created

| File | Type |
|------|------|
| `scripts/prototypes/router_context_decision_prototype.py` | Prototype generator |
| `scripts/tests/test-router-context-prototype.py` | 176 tests |
| `fixtures/router-context-prototype/decision-*.json` | 9 workload fixtures |
| `fixtures/router-context-prototype/scenario-*.json` | 7 scenario fixtures |
| `reports/router-context-prototype-decisions.json` | Machine-readable output |
| `reports/MAC-WIN-ROUTER-CONTEXT-PROTOTYPE-1.md` | Human-readable report |
| `docs/sprints/MAC-WIN-ROUTER-CONTEXT-PROTOTYPE-1.md` | This closeout doc |

## Test Results

```
Results: 176 passed, 0 failed, 176 total
```

All 16 test categories passed:
1. Prototype emits both model_route and context_route
2. All 9 workload types produce decisions
3. Contract v0.1 compliance verified
4. Receipt generation uses canonical_evidence_read
5. Receipt generation blocks weak-provenance + safe
6. Sprint closeout requires fresh evidence
7. Degraded node applies ~4016ms penalty
8. Long-session uses cheap local context
9. Recall packet local cost separate from network
10. Every decision has rejected alternatives
11. Every decision has human-readable reason
12. Every decision has receipt summary
13. No production router files modified
14. No GPU/RDMA/KV-cache claims
15. Measured profiles used (not synthetic)
16. Scenario decisions generated

## State Verification

```
Starting HEAD: 9b1b4b1
Git status: clean (uncommitted prototype files)
Service: LibrarianRunTimeNode — Stopped / Manual
Router processes: 0
Orphan processes: 0
```

## Result Classification

### **Promote**

Prototype decisions are coherent, measured-cost-backed, contract-compliant, and useful enough to justify a runtime-adjacent design sprint.

## Recommended Next Sprint

```
MAC/WIN-ROUTER-CONTEXT-RUNTIME-DESIGN-1
```
