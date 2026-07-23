# Router Context Runtime Design

## MAC/WIN-ROUTER-CONTEXT-RUNTIME-DESIGN-1

> **Design sprint only.** No production router behavior changed.
> No live routing. No model execution. No cache engine.
> No GPU/RDMA/KV-cache acceleration claims.

**Sprint:** MAC/WIN-ROUTER-CONTEXT-RUNTIME-DESIGN-1
**Date:** 2026-06-28
**Status:** Design / architecture artifact
**Starting HEAD:** c6a26ec

---

## 1. Executive Summary

This document defines how the measured `model_route + context_route` decision system would attach to the existing router/runtime architecture in the future, without enabling production behavior. It establishes the architectural boundary, input/output contracts, and integration points for a future implementation sprint.

**Key design decisions:**

1. The context decision layer is a **separate advisory service** that runs alongside the router, not inside it.
2. Decisions are **read-only advisories** that the router can choose to consume or ignore.
3. The layer consumes **measured hardware profiles** and **workload definitions**, not live runtime state.
4. Receipts consume `model_route + context_route` as **metadata**, not as routing instructions.
5. Degraded node detection is **passive** — the decision layer checks health endpoints but never initiates runtime state changes.

---

## 2. Design Questions Answered

### Q1: Where would the future context decision layer live?

**Answer:** As a separate Python module or service alongside the existing router.

```
librarian-runtime-node/
├── router/
│   ├── router.py              # Existing production router (DO NOT TOUCH)
│   └── context_decision.py    # NEW: Advisory decision layer (FUTURE)
├── config/
│   ├── model-profiles.json    # Existing model profiles
│   ├── measured_hardware_profiles.json  # Measured costs (from MEASURE-1)
│   └── context_decision_config.json     # NEW: Decision layer config (FUTURE)
├── fixtures/
│   └── router-context-runtime-design/
│       ├── interface-context-decision-input.json
│       ├── interface-context-decision-output.json
│       └── interface-receipt-consumption.json
```

**Rationale:**
- The existing router (`router.py`, `rust-router/`) remains untouched during this sprint.
- The decision layer is a new module that can be developed, tested, and validated independently.
- The decision layer does not modify any production router behavior.
- The decision layer reads from `config/measured_hardware_profiles.json` (already exists from MEASURE-1).

### Q2: What inputs would it consume?

**Answer:** The decision layer consumes three input categories:

1. **Workload descriptor** — Identifies the type of work being performed.
2. **Hardware profile** — Measured costs for the target platform.
3. **Node health** — Current state of the runtime node (optional, advisory only).

**Input interface sketch:** See `fixtures/router-context-runtime-design/interface-context-decision-input.json`

**Detailed inputs:**

| Input | Source | Required | Purpose |
|-------|--------|----------|---------|
| `workload_type` | Caller-provided | Yes | Determines routing policy |
| `hardware_profile` | `config/measured_hardware_profiles.json` | Yes | Provides measured cost data |
| `node_health` | Optional health check | No | Adjusts degraded-node penalties |
| `governance_overrides` | Config or caller | No | Overrides for specific workloads |

### Q3: What outputs would it emit?

**Answer:** The decision layer emits a **context decision advisory** containing:

1. **Selected context route** — The recommended route for this workload.
2. **Estimated latency** — Based on measured hardware costs.
3. **Governance outcome** — Whether the route is safe, warning, or blocked.
4. **Evidence requirements** — What evidence the route requires.
5. **Receipt summary** — Human-readable summary for audit trails.

**Output interface sketch:** See `fixtures/router-context-runtime-design/interface-context-decision-output.json`

**Critical boundary:** The output is an **advisory**. The router is not required to act on it. The decision layer has no authority to change routing behavior.

### Q4: How does it relate to existing model route/profile selection?

**Answer:** The context decision layer operates **independently** of model route selection.

**Current model route selection:**
- Happens in `router.py` via `ProfileManager` and `RefusalEngine`
- Selects based on `task_classes` and `verified_status`
- Operates at the HTTP endpoint level (`/backend/select`)

**Future context decision layer:**
- Selects based on `workload_type` and `measured_costs`
- Operates as a pre-routing advisory
- Does not replace model route selection — it complements it

**Relationship diagram:**

```
┌─────────────────────────────────────────────────────┐
│  Current Production Router (DO NOT TOUCH)           │
│                                                     │
│  /backend/select ──► ProfileManager                │
│                    ──► RefusalEngine                │
│                    ──► ProcessManager               │
└─────────────────────────────────────────────────────┘
                         ▲
                         │ (future: advisory input)
                         │
┌─────────────────────────────────────────────────────┐
│  Context Decision Layer (FUTURE - NOT ACTIVE)       │
│                                                     │
│  workload_type ──► Decision Engine                  │
│  hardware_profile ──► (measured costs)              │
│  node_health ──► (optional)                         │
│                                                     │
│  Output: context_decision advisory                  │
└─────────────────────────────────────────────────────┘
```

### Q5: How would receipts consume `model_route + context_route`?

**Answer:** Receipts would include `model_route + context_route` as **metadata fields** in the evidence JSON.

**Current receipt structure** (from `evidence.rs` / `EvidenceWriter`):
```json
{
  "status": "ok",
  "profile": "phi-4",
  "authority": "advisory_only",
  "timestamp": "..."
}
```

**Future receipt structure** (advisory metadata):
```json
{
  "status": "ok",
  "profile": "phi-4",
  "authority": "advisory_only",
  "timestamp": "...",
  "context_decision": {
    "advisory": true,
    "selected_context_route": "canonical_evidence_read",
    "estimated_latency_ms": 70.9,
    "governance_outcome": "safe",
    "workload_type": "sprint_closeout",
    "measured_costs_version": "1.0.0",
    "receipt_summary": {
      "label": "canonical_evidence_read selected for Sprint Closeout",
      "detail": "Performance was sacrificed to satisfy live_git_test_state evidence requirements.",
      "risk": "high"
    }
  }
}
```

**Critical boundary:** The `context_decision` field is **metadata only**. It does not change how the router processes the request. It is recorded for audit and analysis purposes.

**Receipt consumption path:**
1. Decision layer generates advisory (non-production).
2. Router receives advisory as optional input (future).
3. Router records advisory in evidence JSON (future).
4. Receipt analysis tools can query advisory metadata (future).
5. Advisory metadata does not affect routing decisions (current boundary).

### Q6: How does it safely check degraded runtime state?

**Answer:** Through **passive health checks** that never initiate runtime state changes.

**Degraded state detection:**
1. The decision layer queries `GET /health` on the runtime node endpoint.
2. If the endpoint returns a valid response, the node is considered healthy.
3. If the endpoint times out (~4 seconds based on MEASURE-1 measurements), the node is considered degraded.
4. If the endpoint returns an error, the node is considered unhealthy.

**Safety boundaries:**
- The decision layer **never** starts or stops runtime processes.
- The decision layer **never** modifies `model-profiles.json` or any configuration.
- The decision layer **never** sends requests to backend model endpoints.
- The decision layer **only** reads health state — it does not write or modify it.

**Measured degraded-node penalty:** ~4000ms (from MEASURE-1 measurements).

### Q7: What must remain advisory?

**Answer:** Everything. The entire context decision layer is advisory by design.

**Advisory boundaries:**
1. **Route selection** — The decision layer recommends routes, the router chooses.
2. **Latency estimates** — Based on measured costs, not live profiling.
3. **Governance outcomes** — The decision layer evaluates policies, the router enforces.
4. **Evidence requirements** — The decision layer documents requirements, the router collects evidence.
5. **Receipt metadata** — The decision layer provides context, the router records it.

**Authority chain:**
- Decision layer: **advisory_only** (no authority to change routing)
- Router: **advisory_only** (per existing contract)
- Runtime node: **managed** (per existing service lifecycle)

### Q8: What tests would gate future integration?

**Answer:** Tests that validate documentation contracts, fixture schemas, and advisory behavior without changing runtime.

**Test categories:**

1. **Contract validation tests** — Verify that decision layer input/output schemas match the design.
2. **Fixture validation tests** — Verify that generated fixtures conform to the interface contracts.
3. **Advisory-only tests** — Verify that the decision layer never modifies production state.
4. **Measured-cost tests** — Verify that decisions use measured costs from MEASURE-1.
5. **Governance tests** — Verify that governance mandates are correctly applied.

**Test command:** `python scripts/tests/test-router-context-runtime-design.py`

### Q9: What production files must not be touched yet?

**Answer:** The following files/behaviors are explicitly forbidden for this sprint:

| File/Behavior | Status | Reason |
|---------------|--------|--------|
| `router/router.py` | FORBIDDEN | Production router implementation |
| `rust-router/src/*.rs` | FORBIDDEN | Production Rust router |
| `config/model-profiles.json` | FORBIDDEN | Production model configuration |
| `rust-router/Cargo.toml` | FORBIDDEN | Production build configuration |
| `LibrarianRunTimeNode` service | FORBIDDEN | Production service state |
| Any llama-server process | FORBIDDEN | Production model execution |
| Any health endpoint behavior | FORBIDDEN | Production health checking |
| Any refusal condition | FORBIDDEN | Production contract enforcement |

### Q10: What is the exact next implementation boundary, if any?

**Answer:** The next implementation boundary is **design artifact validation only**.

**This sprint produces:**
1. Design document (this file)
2. Interface sketches (JSON fixtures)
3. Validation tests (Python)
4. Sprint documentation

**This sprint does NOT produce:**
1. Any working decision layer code
2. Any router modifications
3. Any runtime state changes
4. Any production behavior changes

**Future implementation boundary (for reference only):**
- **MAC/WIN-ROUTER-CONTEXT-ADVISORY-STUB-1** would create a minimal advisory stub.
- **MAC/WIN-ROUTER-CONTEXT-RUNTIME-CONTRACT-1** would formalize the integration contract.
- Neither is approved until this design sprint proves the boundary is clean.

---

## 3. Architecture Overview

### 3.1 Current Architecture (DO NOT CHANGE)

```
┌─────────────────────────────────────────────────────────────────┐
│                    Current Production Router                     │
│                                                                 │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────┐    │
│  │ ProfileManager│  │ RefusalEngine│  │ ProcessManager     │    │
│  │ (config.rs)  │  │ (refusal.rs) │  │ (process.rs)       │    │
│  └──────┬──────┘  └──────┬───────┘  └─────────┬──────────┘    │
│         │                │                     │                │
│         └────────────────┼─────────────────────┘                │
│                          │                                      │
│                    ┌─────▼─────┐                               │
│                    │ HTTP Server│                               │
│                    │ (server.rs)│                               │
│                    └─────┬─────┘                               │
│                          │                                      │
└──────────────────────────┼──────────────────────────────────────┘
                           │
                    ┌──────▼──────┐
                    │ EvidenceWriter│
                    │ (evidence.rs)│
                    └─────────────┘
```

### 3.2 Future Architecture (DESIGN ONLY — NOT ACTIVE)

```
┌─────────────────────────────────────────────────────────────────┐
│                    Current Production Router                     │
│                    (DO NOT CHANGE)                               │
│                                                                 │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────┐    │
│  │ ProfileManager│  │ RefusalEngine│  │ ProcessManager     │    │
│  └──────┬──────┘  └──────┬───────┘  └─────────┬──────────┘    │
│         │                │                     │                │
│         └────────────────┼─────────────────────┘                │
│                          │                                      │
│                    ┌─────▼─────┐                               │
│                    │ HTTP Server│                               │
│                    └─────┬─────┘                               │
│                          │                                      │
│  ┌───────────────────────┼───────────────────────────────┐     │
│  │                       │ (advisory metadata)           │     │
│  │                 ┌─────▼─────┐                         │     │
│  │                 │ EvidenceWriter│                     │     │
│  │                 │ (includes context_decision)│       │     │
│  │                 └───────────┘                         │     │
│  └───────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────┘
                           ▲
                           │ (advisory input - FUTURE)
                           │
┌──────────────────────────┼──────────────────────────────────────┐
│  Context Decision Layer  │  (FUTURE - NOT ACTIVE)               │
│                          │                                      │
│  ┌───────────────────────▼───────────────────────────┐         │
│  │              DecisionEngine                       │         │
│  │                                                   │         │
│  │  Inputs:                                          │         │
│  │    - workload_type (caller-provided)              │         │
│  │    - hardware_profile (measured_hardware_profiles)│         │
│  │    - node_health (optional health check)          │         │
│  │                                                   │         │
│  │  Outputs:                                         │         │
│  │    - selected_context_route (advisory)            │         │
│  │    - estimated_latency_ms (measured)              │         │
│  │    - governance_outcome (policy evaluation)       │         │
│  │    - evidence_requirements (documentation)        │         │
│  │    - receipt_summary (audit trail)                │         │
│  └───────────────────────────────────────────────────┘         │
│                                                                 │
│  ┌───────────────────────────────────────────────────┐         │
│  │              CostEstimator                        │         │
│  │                                                   │         │
│  │  Reads: config/measured_hardware_profiles.json    │         │
│  │  Provides: route latency estimates                │         │
│  └───────────────────────────────────────────────────┘         │
│                                                                 │
│  ┌───────────────────────────────────────────────────┐         │
│  │              GovernanceEvaluator                  │         │
│  │                                                   │         │
│  │  Evaluates: workload-specific policies            │         │
│  │  Provides: governance_outcome                     │         │
│  └───────────────────────────────────────────────────┘         │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. Interface Contracts

### 4.1 Decision Layer Input Interface

```json
{
  "workload_type": "sprint_closeout",
  "hardware_profile": "windows_runtime_node",
  "node_health": "stopped",
  "governance_overrides": {}
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `workload_type` | string | Yes | One of the 9 workload types from contract |
| `hardware_profile` | string | Yes | One of the measured hardware profiles |
| `node_health` | string | No | Current node health state |
| `governance_overrides` | object | No | Override governance rules for this decision |

### 4.2 Decision Layer Output Interface

```json
{
  "decision_id": "ctx-decision-abc12345",
  "timestamp": "2026-06-28T12:00:00.000000+00:00",
  "advisory_status": "non-production",
  "workload_type": "sprint_closeout",
  "context_route": {
    "selected_route": "canonical_evidence_read",
    "estimated_latency_ms": 70.9,
    "all_route_latencies": {
      "canonical_evidence_read": 70.9,
      "compressed_recall_packet": 4.61,
      "ram_cache": 4.8,
      "ssd_cache": 18.2,
      "recomputation_from_source": 500.0
    },
    "freshness_state": "verified_current",
    "provenance_state": "verified",
    "governance_outcome": "safe",
    "performance_sacrificed_for_evidence": true
  },
  "evidence_requirements": {
    "requires_current_git_state": true,
    "requires_current_test_state": true,
    "requires_canonical_source": false,
    "allows_stale_context": false
  },
  "receipt_summary": {
    "label": "canonical_evidence_read selected for Sprint Closeout",
    "detail": "Performance was sacrificed to satisfy live_git_test_state evidence requirements.",
    "risk": "high"
  },
  "measured_costs_version": "1.0.0"
}
```

### 4.3 Receipt Consumption Interface

```json
{
  "context_decision": {
    "advisory": true,
    "selected_context_route": "canonical_evidence_read",
    "estimated_latency_ms": 70.9,
    "governance_outcome": "safe",
    "workload_type": "sprint_closeout",
    "measured_costs_version": "1.0.0",
    "decision_id": "ctx-decision-abc12345",
    "receipt_summary": {
      "label": "canonical_evidence_read selected for Sprint Closeout",
      "detail": "Performance was sacrificed to satisfy live_git_test_state evidence requirements.",
      "risk": "high"
    }
  }
}
```

---

## 5. Measured Cost References

All cost estimates in this design are derived from measured data from MAC/WIN-ROUTER-CONTEXT-MEASURE-1.

### 5.1 Key Measured Costs

| Operation | Measured Cost | Source |
|-----------|---------------|--------|
| File read (warm, 32K tokens) | ~0.28ms | MEASURE-1: file_io.large_32k.read_warm |
| JSON parse (8K tokens) | ~0.05ms | MEASURE-1: json_processing.synthetic.medium_8k.parse |
| JSON serialize (32K tokens) | ~0.41ms | MEASURE-1: large_context.large_32k.serialize |
| Git status | ~70.90ms | MEASURE-1: canonical_evidence.git_status |
| Git rev-parse | ~55.47ms | MEASURE-1: canonical_evidence.git_revparse |
| Recall packet (32K, local) | ~0.50ms | MEASURE-1: recall_packet.medium_32k (total) |
| Small append pipeline | ~0.80ms | MEASURE-1: small_append (total) |
| Large context pipeline (32K) | ~1.91ms | MEASURE-1: large_context (total) |
| Large context pipeline (64K) | ~3.80ms | MEASURE-1: large_context (total) |
| Degraded node timeout | ~4016ms | MEASURE-1: runtime_health, lan_roundtrip |

### 5.2 Cost Comparison: Prototype vs Measured

| Route | Prototype Estimate | Measured Cost | Source |
|-------|-------------------|---------------|--------|
| recent_turn_window | 0.30ms | 0.28ms | file_io.large_32k.read_warm |
| ram_cache | ~4.20ms | 0.28ms | file_io.large_32k.read_warm |
| ssd_cache | ~13.40ms | 0.28ms | file_io.large_32k.read_warm |
| compressed_recall_packet | ~4.17ms | 0.50ms | recall_packet.medium_32k |
| canonical_evidence_read | ~70.90ms | 70.90ms | canonical_evidence.git_status |
| recomputation_from_source | 500.0ms | Not measured | (conservative estimate) |

---

## 6. Prototype Decision References

All governance rules and workload definitions are derived from prototype decisions from MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1.

### 6.1 Governance-Mandated Routes

| Workload | Mandated Route | Reason |
|----------|----------------|--------|
| receipt_generation | canonical_evidence_read | Requires canonical provenance |
| sprint_closeout | canonical_evidence_read | Requires fresh git/test state |
| validation | canonical_evidence_read | Requires current test output |
| agent_handoff | compressed_recall_packet | Requires complete state snapshot |
| runtime_node_qualification | canonical_evidence_read | Requires live node health |

### 6.2 Workload Classification

| Category | Workloads | Preferred Routes |
|----------|-----------|------------------|
| Evidence-heavy | receipt_generation, sprint_closeout, validation, runtime_node_qualification | canonical_evidence_read |
| State-transfer | agent_handoff, code_patch_preparation | compressed_recall_packet |
| Continuation | long_session_continuation, ui_review_or_design_planning, sprint_planning | recent_turn_window |

---

## 7. Production Files Forbidden

This sprint must NOT modify any of the following production files:

| File | Path | Status |
|------|------|--------|
| Python router | `router/router.py` | FORBIDDEN |
| Rust router lib | `rust-router/src/lib.rs` | FORBIDDEN |
| Rust router config | `rust-router/src/config.rs` | FORBIDDEN |
| Rust router evidence | `rust-router/src/evidence.rs` | FORBIDDEN |
| Rust router process | `rust-router/src/process.rs` | FORBIDDEN |
| Rust router refusal | `rust-router/src/refusal.rs` | FORBIDDEN |
| Rust router server | `rust-router/src/server.rs` | FORBIDDEN |
| Rust router main | `rust-router/src/main.rs` | FORBIDDEN |
| Rust router Cargo.toml | `rust-router/Cargo.toml` | FORBIDDEN |
| Model profiles | `config/model-profiles.json` | FORBIDDEN |
| Runtime config | `config/runtime-node.local.json` | FORBIDDEN |
| Service configuration | `LibrarianRunTimeNode` | FORBIDDEN |

---

## 8. Future Implementation Boundary

### 8.1 What This Sprint Produces

1. This design document
2. Interface sketches (JSON fixtures)
3. Validation tests (Python)
4. Sprint documentation

### 8.2 What This Sprint Does NOT Produce

1. Working decision layer code
2. Router modifications
3. Runtime state changes
4. Production behavior changes
5. Cache engine implementation
6. GPU/RDMA/KV-cache acceleration

### 8.3 Recommended Next Sprints (NOT APPROVED)

| Sprint | Purpose | Depends On |
|--------|---------|------------|
| MAC/WIN-ROUTER-CONTEXT-ADVISORY-STUB-1 | Create minimal advisory stub | This design sprint |
| MAC/WIN-ROUTER-CONTEXT-RUNTIME-CONTRACT-1 | Formalize integration contract | This design sprint |

**Neither is approved until this design sprint proves the boundary is clean.**

---

## 9. Acceptance Criteria

This design sprint is accepted when:

1. Design document exists and covers all 10 design questions
2. Design references measured profiles from MEASURE-1
3. Design references prototype decisions from PROTOTYPE-1
4. Design defines future attachment point without enabling it
5. Design defines inputs/outputs for decision layer
6. Design defines receipt consumption path
7. Design defines degraded-node handling boundary
8. Design defines advisory-only authority boundary
9. Design lists forbidden production files/behaviors
10. Validation tests pass
11. No production router behavior changed
12. No runtime-node HTTP behavior changed
13. No GPU/RDMA/KV-cache acceleration claims
14. Service state preserved (LibrarianRunTimeNode Stopped/Manual)
15. Orphan process count remains 0

---

## 10. Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-06-28 | Initial design document |
