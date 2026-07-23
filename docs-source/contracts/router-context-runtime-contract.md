# Router Context Runtime Contract — v0.1

> **Contract sprint only.** No production router behavior changed.
> No advisory stub implemented. No live routing.
> No cache engine. No GPU/RDMA/KV-cache acceleration claims.

**Contract version:** `0.1`
**Contract key:** `runtime_context_decision_contract_version`
**Sprint:** MAC/WIN-ROUTER-CONTEXT-RUNTIME-CONTRACT-1
**Date:** 2026-06-28
**Starting HEAD:** bf337a8
**Prior design:** MAC/WIN-ROUTER-CONTEXT-RUNTIME-DESIGN-1

---

## 1. Purpose

This contract defines the formal, versioned interface for a future advisory-only context decision layer that may emit `model_route + context_route + receipt_consumption` objects without enabling live routing behavior.

It hardens the interface sketches from MAC/WIN-ROUTER-CONTEXT-RUNTIME-DESIGN-1 into a tested, versioned contract.

---

## 2. Contract Version

Every conforming object must carry:

| Field | Value | Required |
|-------|-------|----------|
| `runtime_context_decision_contract_version` | `"0.1"` | Yes |

---

## 3. Enumerations

### 3.1 `workload_type`

```
sprint_planning
sprint_closeout
receipt_generation
validation
code_patch_preparation
agent_handoff
long_session_continuation
runtime_node_qualification
ui_review_or_design_planning
```

### 3.2 `task_risk_level`

```
low
medium
high
critical
```

### 3.3 `node_health`

```
not_checked
available
degraded
unreachable
stopped
timeout
unknown
```

### 3.4 `degraded_node_action`

```
avoid_remote_route
use_local_fallback
require_recheck
mark_warning
block_for_task
```

### 3.5 `governance_outcome`

```
safe
warning
requires_revalidation
blocked
```

### 3.6 `selected_context_route`

```
ram_cache
ssd_cache
remote_windows_runtime_cache
recomputation_from_source
compressed_recall_packet
canonical_evidence_read
hybrid_recall_plus_fresh_evidence
recent_turn_window
```

### 3.7 `selected_runtime_profile`

```
mac_coordinator
windows_runtime_node
weak_lan_runtime_node
degraded_node
future_stronger_gpu_node
```

> `future_stronger_gpu_node` is allowed only as a profile identifier. It must not include claims of GPU/RDMA/KV-cache acceleration.

---

## 4. Context Decision Input Object

### 4.1 Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `runtime_context_decision_contract_version` | string | Yes | Must be `"0.1"` |
| `request_id` | string | Yes | Unique identifier for this request |
| `workload_type` | string | Yes | One of the `workload_type` enum values |
| `task_risk_level` | string | Yes | One of the `task_risk_level` enum values |
| `requested_operation` | string | Yes | Brief description of the requested operation |
| `available_runtime_profiles` | string[] | Yes | List of available runtime profile names |
| `measured_hardware_profile_refs` | string[] | Yes | References to measured profile names |
| `context_candidates` | object[] | Yes | List of candidate context routes with metadata |
| `freshness_requirements` | object | Yes | Freshness requirements for this workload |
| `provenance_requirements` | object | Yes | Provenance requirements for this workload |
| `evidence_requirements` | object | Yes | Evidence requirements for this workload |
| `advisory` | boolean | Yes | **Must be `true`** |
| `production_effects_allowed` | boolean | Yes | **Must be `false`** |

### 4.2 Hard Invariants

```
advisory == true
production_effects_allowed == false
```

---

## 5. Context Decision Output Object

### 5.1 Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `runtime_context_decision_contract_version` | string | Yes | Must be `"0.1"` |
| `request_id` | string | Yes | Matches the input request_id |
| `advisory` | boolean | Yes | **Must be `true`** |
| `production_effects_allowed` | boolean | Yes | **Must be `false`** |
| `model_route` | object | Yes | Model route selection (advisory) |
| `context_route` | object | Yes | Context route selection (advisory) |
| `degraded_node_state` | object | Yes | Current degraded node state |
| `decision_summary` | object | Yes | Summary of the decision |
| `receipt_consumption` | object | Yes | Receipt consumption metadata |
| `forbidden_actions_checked` | string[] | Yes | List of forbidden actions verified as not taken |

### 5.2 `model_route` Fields

| Field | Type | Description |
|-------|------|-------------|
| `selected_runtime_profile` | string | Selected runtime profile |
| `selected_model_profile` | string | Selected model alias |
| `backend_state` | string | Backend availability state |
| `fit` | string | Fit assessment |
| `estimated_runtime_cost_ms` | number | Estimated model inference cost |
| `reason_selected` | string | Human-readable reason |
| `limitations` | string | Model limitations |

### 5.3 `context_route` Fields

| Field | Type | Description |
|-------|------|-------------|
| `selected_route` | string | Selected context route |
| `estimated_latency_ms` | number | Estimated latency from measured costs |
| `all_route_latencies` | object | Latencies for all evaluated routes |
| `freshness_state` | string | Freshness state of selected route |
| `provenance_state` | string | Provenance state of selected route |
| `governance_outcome` | string | Governance evaluation result |
| `performance_sacrificed_for_evidence` | boolean | Whether performance was sacrificed |

### 5.4 Hard Invariants

```
advisory == true
production_effects_allowed == false
forbidden_actions_checked must include all actions from Forbidden Actions table
```

---

## 6. Receipt Consumption Object

### 6.1 Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `runtime_context_decision_contract_version` | string | Yes | Must be `"0.1"` |
| `request_id` | string | Yes | Matches the decision request_id |
| `advisory` | boolean | Yes | **Must be `true`** |
| `route_summary` | string | Yes | Brief summary of the route |
| `selected_model_route` | object | Yes | Model route selection |
| `selected_context_route` | object | Yes | Context route selection |
| `governance_outcome` | string | Yes | Governance evaluation result |
| `evidence_quality` | string | Yes | Quality assessment of collected evidence |
| `performance_sacrificed_for_evidence` | boolean | Yes | Whether performance was sacrificed |
| `degraded_node_summary` | object | No | Summary of degraded node state |
| `rejected_alternatives` | object[] | Yes | List of rejected alternative routes |
| `receipt_text` | string | Yes | Human-readable receipt text |

### 6.2 Hard Invariants

```
advisory == true
```

### 6.3 Downstream-Only Consumption

Receipt consumption may consume:
- `route_summary`
- `selected_model_route`
- `selected_context_route`
- `governance_outcome`
- `evidence_quality`
- `performance_sacrificed_for_evidence`
- `degraded_node_summary`
- `rejected_alternatives`
- `receipt_text`

Receipt consumption must not:
- trigger routing
- start runtime
- modify runtime
- select model
- write router state

---

## 7. Degraded Node Contract

### 7.1 Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `node_health` | string | Yes | One of the `node_health` enum values |
| `last_check_latency_ms` | number | Yes | Latency of the last health check |
| `timeout_ms` | number | Yes | Configured timeout for health checks |
| `measured_penalty_ms` | number | Yes | Measured penalty from prior measurement sprint |
| `recommended_action` | string | Yes | One of the `degraded_node_action` enum values |
| `allowed_remote_use` | boolean | Yes | Whether remote routing is allowed |
| `reason` | string | Yes | Human-readable reason for the state |

### 7.2 Invariants

```
if node_health in ["unreachable", "stopped", "timeout"]:
  allowed_remote_use must be false
  measured_penalty_ms must be >= 3000
  recommended_action must be one of:
    avoid_remote_route
    use_local_fallback
    require_recheck
    block_for_task
```

---

## 8. Advisory-Only Invariants

Every valid contract object must enforce:

| Invariant | Input | Output | Receipt |
|-----------|-------|--------|---------|
| `advisory == true` | ✓ Mandatory | ✓ Mandatory | ✓ Mandatory |
| `production_effects_allowed == false` | ✓ Mandatory | ✓ Mandatory | N/A (not required on receipt) |

### 8.1 Prohibited Authorities

No object may authorize any of the following actions:

| Authority | Reason |
|-----------|--------|
| `start_process` | Contract is advisory-only |
| `stop_process` | Contract cannot control runtime lifecycle |
| `select_model` | Contract cannot change model execution |
| `execute_model` | Contract cannot execute inference |
| `modify_router_config` | Production behavior forbidden |
| `modify_model_profiles` | Profile mutation forbidden |
| `open_network_listener` | Network exposure forbidden |
| `change_bind_host` | Network boundary forbidden |
| `write_runtime_state` | State mutation forbidden |
| `apply_context_route` | Context route application forbidden |

---

## 9. Forbidden Live-Routing Behaviors

The following actions are explicitly forbidden in this contract:

| Forbidden Action | Reason |
|-----------------|--------|
| Start backend process | Contract is advisory-only |
| Stop backend process | Contract cannot control runtime lifecycle |
| Select active model | Contract cannot change model execution |
| Apply context route | Contract cannot alter live request handling |
| Modify router config | Production behavior forbidden |
| Modify model profiles | Profile mutation forbidden |
| Change bind host | Network boundary forbidden |
| Open LAN listener | Network exposure forbidden |
| Write runtime state | No state mutation in contract sprint |
| Claim GPU/RDMA/KV acceleration | Unsupported claim |

---

## 10. Measured Cost References (from MAC/WIN-ROUTER-CONTEXT-MEASURE-1)

| Operation | Measured Cost | Used In |
|-----------|---------------|---------|
| File read warm (32K tokens) | ~0.28ms | ram_cache, ssd_cache, recent_turn_window |
| JSON parse (8K tokens) | ~0.05ms | Context processing |
| JSON serialize (32K tokens) | ~0.41ms | Context serialization |
| Git status | ~70.90ms | canonical_evidence_read |
| Git rev-parse | ~55.47ms | canonical_evidence_read |
| Recall packet (32K, local total) | ~0.50ms | compressed_recall_packet |
| Small append pipeline (429 tokens) | ~0.80ms | Context append |
| Large context pipeline (32K) | ~1.91ms | Context movement |
| Large context pipeline (64K) | ~3.80ms | Context movement |
| Degraded node timeout | ~4016ms | degraded_node |

---

## 11. Prototype Decision References (from MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1)

### 11.1 Governance-Mandated Routes

| Workload | Mandated Route | Reason |
|----------|----------------|--------|
| receipt_generation | canonical_evidence_read | Requires canonical provenance |
| sprint_closeout | canonical_evidence_read | Requires fresh git/test state |
| validation | canonical_evidence_read | Requires current test output |
| agent_handoff | compressed_recall_packet | Requires complete state snapshot |
| runtime_node_qualification | canonical_evidence_read | Requires live node health |

### 11.2 Workload Classification

| Category | Workloads | Preferred Routes |
|----------|-----------|------------------|
| Evidence-heavy | receipt_generation, sprint_closeout, validation, runtime_node_qualification | canonical_evidence_read |
| State-transfer | agent_handoff, code_patch_preparation | compressed_recall_packet |
| Continuation | long_session_continuation, ui_review_or_design_planning, sprint_planning | recent_turn_window |

---

## 12. Fixture Specifications

### 12.1 Valid Fixtures

| Fixture | Contract Object | Validates |
|---------|-----------------|-----------|
| `context-decision-input-valid.json` | Input | All required fields, advisory=true, production_effects_allowed=false |
| `context-decision-output-valid.json` | Output | All required fields, advisory=true, production_effects_allowed=false |
| `receipt-consumption-valid.json` | Receipt | All required fields, advisory=true |
| `degraded-node-valid.json` | Degraded Node | Stopped node with invariant constraints |

### 12.2 Invalid Fixtures

| Fixture | Contract Object | Invalid Because |
|---------|-----------------|----------------|
| `forbidden-live-routing-invalid.json` | Output | Contains an authorized forbidden action |
| `advisory-false-invalid.json` | Input | advisory=false |
| `weak-provenance-receipt-invalid.json` | Receipt | Weak provenance marked as safe for receipt_generation |

---

## 13. Version History

| Version | Date | Changes |
|---------|------|---------|
| 0.1 | 2026-06-28 | Initial contract from MAC/WIN-ROUTER-CONTEXT-RUNTIME-DESIGN-1 |
