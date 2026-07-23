# MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1 — Sprint Report

## Router Context Decision Prototype

> **Prototype / non-production routing decisions only.**
> No production router behavior changed. No live routing.
> No model execution changed. No cache engine added.
> No GPU/RDMA/KV-cache acceleration claims.

**Sprint:** MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1
**Date:** 2026-06-28
**Starting HEAD:** 9b1b4b1
**Platform:** Windows 10 (AMD64), Python 3.14.3

---

## 1. Executive Summary

Built a non-production prototype that generates mock router decisions containing both `model_route` and `context_route` objects, using measured hardware/context costs from MAC/WIN-ROUTER-CONTEXT-MEASURE-1.

**Key results:**

1. **9 workload types** each produce a complete `model_route + context_route` decision.
2. **7 scenario cases** demonstrate specific routing behaviors (receipt generation, degraded nodes, agent handoff, etc.).
3. **176 tests pass** — all contract invariants, governance rules, and measured-cost requirements verified.
4. **Measured costs are used** — git subprocess overhead (~71ms), degraded-node penalty (~4016ms), recall packet local processing (~0.5ms).
5. **Governance mandates enforce contract** — receipt_generation and sprint_closeout always use canonical_evidence_read regardless of latency.

---

## 2. Generated Decisions

### 2.1 All 9 Workload Types

| Workload | Selected Route | Latency (ms) | Governance | Model |
|----------|---------------|-------------|------------|-------|
| sprint_planning | recent_turn_window | 0.30 | safe | phi-4 |
| sprint_closeout | canonical_evidence_read | 70.90 | safe | phi-4 |
| receipt_generation | canonical_evidence_read | 70.90 | safe | phi-4 |
| validation | canonical_evidence_read | 70.90 | safe | phi-4 |
| code_patch_preparation | compressed_recall_packet | 4.18 | safe | qwen-coder |
| agent_handoff | compressed_recall_packet | 4.17 | safe | phi-4 |
| long_session_continuation | recent_turn_window | 0.30 | warning | phi-4 |
| runtime_node_qualification | canonical_evidence_read | 70.90 | safe | phi-4 |
| ui_review_or_design_planning | recent_turn_window | 0.30 | warning | phi-4 |

### 2.2 Scenario Results

| Scenario | Route | Latency (ms) | Notes |
|----------|-------|-------------|-------|
| A — Receipt Generation | canonical_evidence_read | 70.90 | Measured git subprocess cost |
| B — Long Session Continuation | recent_turn_window | 0.30 | Cheap local context |
| C — Degraded Runtime Node | canonical_evidence_read | 70.90 | Avoids 4016ms remote penalty |
| D — Agent Handoff | compressed_recall_packet | 4.17 | Measured local recall cost |
| E — Sprint Closeout | canonical_evidence_read | 70.90 | Fresh evidence required |
| F — UI Review / Design | recent_turn_window | 0.30 | Warning governance |
| G — Parallel Mixed | 3 unique routes | 0.30–70.90 | Different routes per workload |

---

## 3. How Measured Costs Changed Route Choices

### 3.1 Local context movement is cheap

Measured costs show local context handling is extremely fast:
- `recent_turn_window`: ~0.30ms
- `ram_cache`: ~0.50ms
- `compressed_recall_packet` (local): ~4.17ms
- `ssd_cache`: ~0.52ms

This means the optimizer should **not** over-optimize local SSD/RAM paths. They are already fast enough.

### 3.2 Canonical evidence is expensive but justified

Git subprocess overhead dominates:
- `git status`: ~71ms
- `git rev-parse`: ~55ms

This cost is **2-3x higher** than the simulator assumed (25ms). But for receipt_generation and sprint_closeout, evidence quality dominates cost consideration.

### 3.3 Degraded remote nodes are devastating

The measured ~4016ms TCP timeout for stopped/unreachable nodes means:
- Remote routing should only be used when node health is confirmed
- The optimizer must strongly prefer local paths when remote health is uncertain
- The 4-second penalty is ~80x higher than the simulator's 50ms base assumption

### 3.4 Recall packets are efficient locally

Measured local recall packet processing:
- Serialize: ~0.22ms
- Decompress: ~0.02ms
- Total local: ~0.50ms for 32K tokens

The 80ms base cost in the simulator was misleading — it included network transfer. Local processing is negligible.

---

## 4. Analysis Questions

### 4.1 Did measured costs materially change route choices?

**Yes.** The key changes:
1. Local routes (recent_turn_window, ram_cache) are now clearly preferred for low-governance workloads because they cost ~0.3ms vs ~71ms for canonical evidence.
2. Canonical evidence is now treated as a deliberate proof cost, not a cheap cache read.
3. Degraded remote routing is now strongly avoided due to the 4016ms penalty.

### 4.2 Which workloads select canonical evidence despite higher cost?

- `sprint_closeout` — requires fresh git/test evidence
- `receipt_generation` — requires canonical provenance-verified source
- `validation` — requires current test output
- `runtime_node_qualification` — requires live node health data

All four have strict governance requirements that override latency optimization.

### 4.3 Which workloads select recall packets because local processing is cheap?

- `code_patch_preparation` — compressed_recall_packet (4.18ms)
- `agent_handoff` — compressed_recall_packet (4.17ms)

Both benefit from the measured ~0.5ms local recall processing cost.

### 4.4 Which workloads avoid remote runtime because degraded-node penalty is too high?

- `runtime_node_qualification` — governance-mandated to canonical_evidence_read
- Scenario C (degraded node) — explicitly avoids remote_windows_runtime_cache

The 4016ms penalty makes remote routing unattractive for any workload that can use a local alternative.

### 4.5 Is the model_route + context_route object understandable?

**Yes.** Each decision includes:
- Clear route selection with measured latency
- Human-readable reason explaining why
- Receipt summary with governance risk level
- Rejected alternatives with specific cost comparisons
- Model profile selection with task class matching

### 4.6 What production integration risks remain?

1. **Governance mandates are hardcoded** — production router needs dynamic governance evaluation
2. **Model inference cost not measured** — estimated_runtime_cost_ms is 0 (not in scope)
3. **Freshness/provenance states are static** — production needs real-time freshness tracking
4. **No actual cache behavior** — prototype assumes ideal cache hit rates
5. **Mac measurements unavailable** — only Windows runtime node measured

### 4.7 Should this promote to a runtime-adjacent design sprint?

**Yes.** The prototype demonstrates:
- Measured costs produce coherent routing decisions
- Governance mandates correctly enforce contract invariants
- The `model_route + context_route` object is well-structured
- Receipt summaries are actionable for audit

---

## 5. Result Classification

### **Promote**

Prototype decisions are coherent, measured-cost-backed, contract-compliant, and useful enough to justify a runtime-adjacent design sprint.

**Recommended next sprint:** `MAC/WIN-ROUTER-CONTEXT-RUNTIME-DESIGN-1`

---

## 6. Files Created

| File | Purpose |
|------|---------|
| `scripts/prototypes/router_context_decision_prototype.py` | Prototype decision generator |
| `scripts/tests/test-router-context-prototype.py` | 176 contract/governance tests |
| `fixtures/router-context-prototype/decision-*.json` | 9 workload decision fixtures |
| `fixtures/router-context-prototype/scenario-*.json` | 7 scenario fixtures |
| `reports/router-context-prototype-decisions.json` | Machine-readable decisions |
| `reports/MAC-WIN-ROUTER-CONTEXT-PROTOTYPE-1.md` | This report |
| `docs/sprints/MAC-WIN-ROUTER-CONTEXT-PROTOTYPE-1.md` | Sprint closeout doc |

---

## 7. Working Tree Status

- **Starting HEAD:** 9b1b4b1
- **Production router behavior:** UNCHANGED
- **Model execution behavior:** UNCHANGED
- **Service state preserved:** YES (LibrarianRunTimeNode stopped/manual)
- **Orphan processes:** 0
- **No GPU/RDMA/KV-cache claims:** VERIFIED
