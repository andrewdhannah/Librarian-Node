# MAC/WIN-ROUTER-CONTEXT-RUNTIME-DESIGN-1 — Sprint Report

## Router Context Runtime Design

> **Design sprint only.** No production router behavior changed.
> No live routing. No model execution. No cache engine.
> No GPU/RDMA/KV-cache acceleration claims.

**Sprint:** MAC/WIN-ROUTER-CONTEXT-RUNTIME-DESIGN-1
**Date:** 2026-06-28
**Starting HEAD:** c6a26ec
**Platform:** Design artifact only — no runtime execution

---

## 1. Executive Summary

Designed how the measured `model_route + context_route` decision system would attach to the existing router/runtime architecture in the future, without enabling production behavior.

**Key results:**

1. **10 design questions answered** — All primary questions addressed with specific architectural decisions.
2. **3 interface sketches created** — Input, output, and receipt consumption interfaces defined.
3. **72 validation tests pass** — All contract, fixture, and boundary tests verified.
4. **Production boundary preserved** — No production files modified, no behavior changed.
5. **Future implementation boundary defined** — Next sprints identified but not yet approved.

---

## 2. Design Questions Answered

| Question | Answer |
|----------|--------|
| Q1: Where would the context decision layer live? | Separate Python module alongside router |
| Q2: What inputs would it consume? | workload_type, hardware_profile, node_health |
| Q3: What outputs would it emit? | Advisory with route, latency, governance, evidence |
| Q4: How does it relate to model route/profile selection? | Independent, complementary advisory |
| Q5: How would receipts consume model_route + context_route? | As metadata fields in evidence JSON |
| Q6: How does it safely check degraded runtime state? | Passive health checks, never modifies state |
| Q7: What must remain advisory? | Everything — entire layer is advisory_only |
| Q8: What tests would gate future integration? | 72 contract/fixture/boundary tests |
| Q9: What production files must not be touched yet? | 12 files listed as FORBIDDEN |
| Q10: What is the exact next implementation boundary? | Design artifact validation only |

---

## 3. Architecture Decisions

### 3.1 Decision Layer is Separate

The context decision layer runs alongside the router, not inside it. This preserves the existing production router boundary while enabling future advisory capabilities.

### 3.2 Decisions are Advisory Only

All decisions from the context decision layer are read-only advisories. The router is not required to act on them. The decision layer has no authority to change routing behavior.

### 3.3 Receipts Consume as Metadata

Receipts include `context_decision` as metadata fields in the evidence JSON. This metadata does not affect routing decisions — it is recorded for audit and analysis purposes.

### 3.4 Degraded Node Detection is Passive

The decision layer checks health endpoints but never initiates runtime state changes. It never starts or stops processes, never modifies configuration, and never sends requests to backend model endpoints.

---

## 4. Interface Contracts

### 4.1 Input Interface

```json
{
  "workload_type": "sprint_closeout",
  "hardware_profile": "windows_runtime_node",
  "node_health": "stopped"
}
```

### 4.2 Output Interface

```json
{
  "decision_id": "ctx-decision-abc12345",
  "advisory_status": "non-production",
  "context_route": {
    "selected_route": "canonical_evidence_read",
    "estimated_latency_ms": 70.9,
    "governance_outcome": "safe"
  }
}
```

### 4.3 Receipt Consumption Interface

```json
{
  "context_decision": {
    "advisory": true,
    "selected_context_route": "canonical_evidence_read",
    "estimated_latency_ms": 70.9
  }
}
```

---

## 5. Test Results

**Total tests:** 72
**Passed:** 72
**Failed:** 0

**Test categories:**
1. Design document validation: 17 tests
2. Fixture schema validation: 23 tests
3. Measured costs reference validation: 12 tests
4. Prototype decisions reference validation: 20 tests
5. Advisory-only boundary validation: 9 tests (including 3 fixture-level checks)

---

## 6. Production Boundary Verification

| File/Behavior | Status |
|---------------|--------|
| `router/router.py` | UNCHANGED |
| `rust-router/src/*.rs` | UNCHANGED |
| `config/model-profiles.json` | UNCHANGED |
| `rust-router/Cargo.toml` | UNCHANGED |
| `LibrarianRunTimeNode` service | UNCHANGED (Stopped / Manual) |
| llama-server processes | UNCHANGED (0 orphans) |
| Health endpoints | UNCHANGED |
| Refusal conditions | UNCHANGED |

---

## 7. Measured Cost References

All cost estimates derived from MAC/WIN-ROUTER-CONTEXT-MEASURE-1:

| Operation | Measured Cost | Used In Design |
|-----------|---------------|----------------|
| git status | ~70.90ms | canonical_evidence_read |
| git rev-parse | ~55.47ms | canonical_evidence_read |
| File read (32K tokens) | ~0.28ms | ram_cache, ssd_cache |
| Recall packet (32K) | ~0.50ms | compressed_recall_packet |
| Degraded node timeout | ~4016ms | remote route penalty |

---

## 8. Prototype Decision References

Governance rules derived from MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1:

| Workload | Mandated Route | Reason |
|----------|----------------|--------|
| receipt_generation | canonical_evidence_read | Canonical provenance |
| sprint_closeout | canonical_evidence_read | Fresh git/test state |
| validation | canonical_evidence_read | Current test output |
| agent_handoff | compressed_recall_packet | Complete state snapshot |
| runtime_node_qualification | canonical_evidence_read | Live node health |

---

## 9. Files Created

| File | Purpose |
|------|---------|
| `docs/design/router-context-runtime-design.md` | Main design document |
| `docs/sprints/MAC-WIN-ROUTER-CONTEXT-RUNTIME-DESIGN-1.md` | Sprint documentation |
| `reports/MAC-WIN-ROUTER-CONTEXT-RUNTIME-DESIGN-1.md` | This report |
| `fixtures/router-context-runtime-design/interface-context-decision-input.json` | Input interface sketch |
| `fixtures/router-context-runtime-design/interface-context-decision-output.json` | Output interface sketch |
| `fixtures/router-context-runtime-design/interface-receipt-consumption.json` | Receipt consumption interface sketch |
| `scripts/tests/test-router-context-runtime-design.py` | 72 validation tests |

---

## 10. Working Tree Status

- **Starting HEAD:** c6a26ec
- **Final HEAD:** c6a26ec (no commits)
- **Production router behavior:** UNCHANGED
- **Model execution behavior:** UNCHANGED
- **Service state preserved:** YES (LibrarianRunTimeNode Stopped / Manual)
- **Orphan processes:** 0
- **No GPU/RDMA/KV-cache claims:** VERIFIED

---

## 11. Result Classification

### **Promote**

Design sprint completed successfully. All acceptance criteria met. Boundary is clean.

**Recommended next sprint:** `MAC/WIN-ROUTER-CONTEXT-RUNTIME-CONTRACT-1` or `MAC/WIN-ROUTER-CONTEXT-ADVISORY-STUB-1`

**Note:** Neither next sprint is approved until this design sprint proves the boundary is clean. This design sprint proves the boundary is clean.

---

## 12. Acceptance Criteria Check

| Criterion | Status |
|-----------|--------|
| 1. Runtime-adjacent design doc exists | PASS |
| 2. Design references measured profiles from MEASURE-1 | PASS |
| 3. Design references prototype outputs from PROTOTYPE-1 | PASS |
| 4. Design defines future attachment point without enabling it | PASS |
| 5. Design defines inputs/outputs for decision layer | PASS |
| 6. Design defines receipt consumption path | PASS |
| 7. Design defines degraded-node handling boundary | PASS |
| 8. Design defines advisory-only authority boundary | PASS |
| 9. Design lists production files/behaviors forbidden for this sprint | PASS |
| 10. Optional fixtures/tests pass | PASS (72/72) |
| 11. No production router behavior changed | PASS |
| 12. No runtime-node HTTP behavior changed | PASS |
| 13. No GPU/RDMA/KV-cache acceleration claims | PASS |
| 14. Service state preserved | PASS |
| 15. Orphan process count remains 0 | PASS |

---

## 13. Author

OpenWork Agent
Date: 2026-06-28
