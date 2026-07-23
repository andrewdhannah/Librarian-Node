# Context Route Contract (Draft — Non-Production)

> Draft contract for the Librarian context-route decision object.
> Defines the structured output that a future router may produce when deciding how to supply context for a model or runtime task.
>
> Sprint: MAC/WIN-ROUTER-CONTEXT-CONTRACT-0
> Status: DRAFT — non-production, simulator and test only
> Authority: advisory_only
> Contract Version: `0.1`

---

## 1. Purpose

This contract specifies the shape and semantics of a `context_route` object that The Librarian router may eventually emit when making context-routing decisions.

It is derived from findings in:
- `MAC/WIN-CONTEXT-REUSE-SIMULATOR-0` (generic context-path scheduling)
- `MAC/WIN-ROUTER-WORKLOAD-OPTIMIZER-1` (Librarian workload-aware routing)

This contract is **not** a production router change. It defines a stable, testable interface for future integration.

## 2. Object Shape

### 2.1 Top-Level Schema

```json
{
  "route_id": "string (required, non-empty)",
  "contract_version": "0.1 (required)",
  "workload_type": "enum (required)",
  "selected_context_route": "enum (required)",
  "selected_runtime_profile": "enum (required)",
  "freshness_state": "enum (required)",
  "provenance_state": "enum (required)",
  "governance_outcome": "enum (required)",
  "estimated_latency_ms": "number >= 0 (required)",
  "performance_sacrificed_for_evidence": "boolean (required)",
  "reason_selected": "string (required, non-empty)",
  "alternatives_rejected": "array of objects (required)",
  "evidence_requirements": "object (required)",
  "receipt_summary": "object (required)"
}
```

### 2.2 Field Definitions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `route_id` | string | Yes | Unique identifier for this context-route decision instance |
| `contract_version` | string | Yes | Contract version. Must be `"0.1"` for this draft |
| `workload_type` | enum | Yes | The Librarian workload type being served |
| `selected_context_route` | enum | Yes | The context route chosen |
| `selected_runtime_profile` | enum | Yes | The hardware/runtime profile selected |
| `freshness_state` | enum | Yes | Freshness governance state of the selected route |
| `provenance_state` | enum | Yes | Provenance governance state of the selected route |
| `governance_outcome` | enum | Yes | Overall governance outcome |
| `estimated_latency_ms` | number | Yes | Estimated latency in milliseconds. Must be >= 0 |
| `performance_sacrificed_for_evidence` | boolean | Yes | True if a slower route was chosen to satisfy evidence requirements |
| `reason_selected` | string | Yes | Human-readable explanation for why this route was selected |
| `alternatives_rejected` | array | Yes | List of rejected alternative routes with reasons |
| `evidence_requirements` | object | Yes | Evidence constraints for this workload |
| `receipt_summary` | object | Yes | Receipt-like summary for governance and audit |

### 2.3 Alternatives Rejected Shape

Each element:

```json
{
  "route": "enum (required)",
  "reason": "string (required, non-empty)"
}
```

### 2.4 Evidence Requirements Shape

```json
{
  "requires_current_git_state": "boolean (required)",
  "requires_current_test_state": "boolean (required)",
  "requires_canonical_source": "boolean (required)",
  "allows_stale_context": "boolean (required)"
}
```

### 2.5 Receipt Summary Shape

```json
{
  "label": "string (required, non-empty)",
  "detail": "string (required, non-empty)",
  "risk": "enum (required): low | medium | high"
}
```

## 3. Controlled Enumerations

### 3.1 `workload_type`

| Value | Description |
|-------|-------------|
| `sprint_planning` | Roadmap + recent status + recall packet |
| `sprint_closeout` | Strict freshness, current git/test evidence |
| `receipt_generation` | Canonical evidence required |
| `validation` | Test results must be current |
| `code_patch_preparation` | Current source files + recent decisions |
| `agent_handoff` | Compressed recall packet for continuity |
| `long_session_continuation` | High reuse, short append, many turns |
| `runtime_node_qualification` | Live node status, hardware profile |
| `ui_review_or_design_planning` | Visual/design context, screenshots |

### 3.2 `selected_context_route`

| Value | Description |
|-------|-------------|
| `ram_cache` | Fast local volatile cache |
| `ssd_cache` | Persistent local storage |
| `remote_windows_runtime_cache` | LAN-accessible remote cache |
| `recomputation_from_source` | Full recomputation from source artifacts |
| `compressed_recall_packet` | Compressed state snapshot |
| `canonical_evidence_read` | Fresh source/git/test artifact read |
| `hybrid_recall_plus_fresh_evidence` | Recall packet supplemented with fresh evidence |

### 3.3 `freshness_state`

| Value | Description |
|-------|-------------|
| `verified_current` | Cache entry is fresh and within threshold |
| `recent_but_unverified` | Recently produced but not yet fully verified |
| `stale_low_risk` | Expired freshness but task tolerates it |
| `stale_requires_revalidation` | Stale and must be revalidated before use |
| `provenance_weak` | Provenance not confirmed |
| `blocked_for_task` | Hard blocked — task cannot use this state |

### 3.4 `provenance_state`

| Value | Description |
|-------|-------------|
| `verified` | Full provenance chain confirmed |
| `partially_verified` | Some provenance confirmed, some uncertain |
| `weak` | Provenance not confirmed; governance risk |
| `unknown` | Provenance status not determined |
| `blocked` | Provenance check explicitly failed |

### 3.5 `governance_outcome`

| Value | Description |
|-------|-------------|
| `safe` | No governance concerns |
| `warning` | Governance concern noted but acceptable |
| `requires_revalidation` | Must revalidate before using this route |
| `blocked` | Route blocked by governance rules |

### 3.6 `selected_runtime_profile`

| Value | Description |
|-------|-------------|
| `mac_coordinator` | Current Mac development machine |
| `windows_runtime_node` | Current Windows runtime node |
| `weak_lan_runtime_node` | Unstable/weak network conditions |
| `future_stronger_gpu_node` | Scalability test only — no GPU/RDMA/KV claims |

## 4. Contract Invariants

1. `route_id` must be unique within a decision session.
2. `contract_version` must be `"0.1"` for this draft.
3. `estimated_latency_ms` must be numeric and >= 0.
4. `alternatives_rejected` must contain at least one entry.
5. `reason_selected` must be non-empty.
6. `receipt_summary.risk` must be one of: `low`, `medium`, `high`.
7. `governance_outcome=blocked` implies `freshness_state=blocked_for_task` OR `provenance_state=blocked`.
8. `governance_outcome=safe` implies `freshness_state` is NOT `blocked_for_task` AND `provenance_state` is NOT `blocked`.
9. `receipt_generation` with `provenance_state=weak` and `governance_outcome=safe` is INVALID.
10. `sprint_closeout` with `allows_stale_context=true` in `evidence_requirements` is INVALID.
11. `runtime_node_qualification` with `freshness_state=stale_requires_revalidation` and `governance_outcome=safe` is INVALID.
12. `performance_sacrificed_for_evidence=true` requires a non-empty `reason_selected` that explains the sacrifice.
13. `future_stronger_gpu_node` fixtures must NOT contain GPU, RDMA, or KV-cache acceleration claims in any text field.

## 5. Non-Production Statement

This contract defines a **future interface only**. It does not:
- Change production router behavior
- Wire into live runtime execution
- Add a production cache
- Implement GPU/RDMA/KV-cache behavior
- Claim DualPath implementation
- Alter model execution

## 6. Fixture Reference

See `fixtures/context-route/` for machine-readable test fixtures.

| Fixture | Workload | Expected Route | Expected Governance |
|---------|----------|---------------|-------------------|
| `sprint-planning.json` | sprint_planning | compressed_recall_packet | safe or warning |
| `sprint-closeout.json` | sprint_closeout | canonical_evidence_read | safe |
| `receipt-generation.json` | receipt_generation | canonical_evidence_read | safe |
| `agent-handoff.json` | agent_handoff | compressed_recall_packet | safe |
| `long-session-continuation.json` | long_session_continuation | recent_turn_window | safe or warning |
| `runtime-node-qualification.json` | runtime_node_qualification | canonical_evidence_read | safe |
| `ui-review-design.json` | ui_review_or_design_planning | ram_cache | safe or warning |
| `blocked-route.json` | receipt_generation | ram_cache (weak provenance) | blocked |
| `performance-sacrificed.json` | sprint_closeout | canonical_evidence_read | safe (perf sacrificed) |
| `future-stronger-node.json` | long_session_continuation | ram_cache | safe |

## 7. Test Reference

Contract tests: `scripts/tests/test-context-route-contract.py`

Run with:
```bash
python scripts/tests/test-context-route-contract.py
```

Tests validate:
1. Required fields exist
2. Controlled enum values enforced
3. `route_id` non-empty
4. `contract_version` exists
5. `estimated_latency_ms` numeric and >= 0
6. `alternatives_rejected` structured
7. `reason_selected` non-empty
8. `receipt_summary` present
9. `blocked` outcome consistency
10. `receipt_generation` provenance rules
11. `sprint_closeout` freshness rules
12. `runtime_node_qualification` staleness rules
13. Performance sacrifice explanation
14. Future node GPU/RDMA/KV-claim avoidance
15. All fixtures validate successfully
