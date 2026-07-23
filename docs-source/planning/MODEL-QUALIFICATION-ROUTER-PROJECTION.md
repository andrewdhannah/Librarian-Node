# MODEL-QUALIFICATION-ROUTER-PROJECTION

**Sprint:** MODEL-QUALIFICATION-ROUTER-BASELINE-INTEGRATION-PLAN-1
**Gate:** MQR-BI-10, MQR-BI-11

---

## Purpose

Define the approved model→role projection that the packet router consumes. The router never reads raw qualification evidence, capability manifests, or benchmark scores. It reads only the approved projection.

---

## Router Projection Table

**Owner:** Mac (Owner-approved only)
**Consumer:** Packet router
**Mutator:** Only via owner_decision → promotion flow

### Schema

```
router_projection (
    projection_id TEXT PRIMARY KEY,
    manifest_id TEXT NOT NULL,         -- references capability_manifest.manifest_id
    identity_id TEXT NOT NULL,         -- references model_identity_record.identity_id
    roles_json TEXT NOT NULL,           -- JSON array of role assignments with constraints
    constraints_json TEXT,              -- JSON object of system/execution constraints
    priority INTEGER DEFAULT 0,        -- higher = preferred when multiple models qualify
    approved_at TEXT NOT NULL,          -- UTC timestamp of Owner approval
    expires_at TEXT,                    -- optional expiry (NULL = indefinite)
    superseded_by TEXT,                 -- NULL unless replaced by newer projection
    FOREIGN KEY (manifest_id) REFERENCES capability_manifest(manifest_id)
    FOREIGN KEY (identity_id) REFERENCES model_identity_record(identity_id)
)
```

### roles_json Structure

```json
[
  {
    "role": "implementer",
    "constraints": {
      "max_context_tokens": 8192,
      "preferred_quant": ["Q4_K_M", "Q8_0"],
      "min_tokens_per_sec": 10.0,
      "requires_gpu": true
    },
    "known_failures": ["long-context-extraction"],
    "approved_at": "2026-07-11T12:00:00Z"
  }
]
```

### constraints_json Structure

```json
{
  "requires_gpu": true,
  "min_vram_mb": 2048,
  "max_vram_mb": 4096,
  "requires_release_proof": true,
  "compatible_backends": ["vulkan"],
  "max_load_time_ms": 30000
}
```

---

## Router Selection Algorithm

When a work packet arrives with a required role:

```
1. Query router_projection WHERE superseded_by IS NULL
2. Filter by roles_json contains required role
3. Filter by expires_at IS NULL OR expires_at > now()
4. Filter by constraints_json matches system capabilities
5. Filter by execution_profile matches hardware constraints
6. Sort by priority DESC
7. Return top match (or null if no match)
```

The router does NOT:
- Query capability_manifest directly
- Read qualification_run results
- Access Windows runtime_runs
- Interpret benchmark scores
- Evaluate Stage 2-7 evidence

---

## Projection Lifecycle

### Creation

```
1. capability_manifest reaches status "approved" or "conditional"
2. Owner creates owner_decision (approve)
3. System promotes to router_projection
4. Router begins selecting this model for the approved role(s)
```

### Supersession

```
1. New qualification run produces better capability_manifest
2. Owner approves new manifest
3. New router_projection created
4. Old projection marked superseded_by = new projection_id
5. Router stops using old projection for new packets
6. In-flight packets using old projection continue to completion
```

### Expiry

```
1. projection.expires_at reached
2. Router stops selecting this projection
3. projection.superseded_by remains NULL (expired, not superseded)
4. Owner can re-approve with new expiry
```

### Revocation

```
1. Owner creates owner_decision (revoke)
2. projection.superseded_by = NULL
3. projection.expires_at = now() (immediate expiry)
4. Router stops selecting immediately
```

---

## Multi-Model Routing

When multiple models qualify for the same role:

| Scenario | Router Behavior |
|----------|----------------|
| Single model qualifies | Select that model |
| Multiple models qualify, different priorities | Select highest priority |
| Multiple models qualify, same priority | Select most recently approved |
| Multiple models, same priority, same time | Select lowest VRAM usage (efficiency) |
| No model qualifies | Return error; packet rejected |

---

## Router Projection Constraints

| Rule | Rationale |
|------|-----------|
| Only Owner-approved projections enter router | Prevent premature routing |
| Projections are immutable once created | No silent mutations |
| Supersession is explicit | Old projection must be identified |
| Expiry is optional but recommended | Prevent stale routing |
| Router never reads Windows DB | Authority boundary |
| Router never interprets raw evidence | Q8 canary preservation |

---

## Router Integration Points

| Integration | From | To | Mechanism |
|-------------|------|----|-----------|
| Packet routing | Router | Mac DB | SELECT from router_projection |
| Model loading | Router | Windows | POST /backend/select via residency |
| Generation | Router | Windows | POST /v1/chat/completions |
| Status feedback | Router | Mac | routing_log INSERT |
