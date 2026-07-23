# EPIC-DECISIONS-001

**Status:** Planning — not yet authorized
**Prerequisites:** STORAGE-001 ✅, ENTITY-001 ✅

---

## Objective

Add durable owner authority records to the governance database — persistent storage for what was approved, by whom, and under what context.

## Why This Exists

The governance database can currently answer:

| Question | Answer | Source |
|----------|--------|--------|
| What happened? | ✅ Evidence records |
| What was produced? | ✅ Receipts |
| What exists? | ✅ Entity registry |
| What state is it in? | ✅ Lifecycle + Residency |
| **What was approved?** | ❌ Not persisted |

Without decision records, human authority intent exists only in conversation or external systems. The governance substrate cannot independently prove that an action had authorization.

## Scope

| Component | Description |
|-----------|-------------|
| Decision records table (migration 003) | Durable storage for owner authorizations |
| Decision status tracking | Approved / rejected / deferred / superseded |
| Decision-to-entity linkage | Which entity authorized what, for which target |
| Decision-to-evidence linkage | What evidence supported the decision |
| Decision-to-receipt linkage | What receipt recorded the decision |

## Non-Scope

```
New governance concepts:        0
New receipt types:              0
New evidence categories:        0
Permission enforcement:         0 (→ PERMISSIONS-001)
Authentication system:          0 (future)
MCP protocol:                   0 (→ UF-001)
Runtime identity:               0 (entity external_id)
Policy evaluation engine:       0 (→ PERMISSIONS-001)
```

## Acceptance Gates

| Gate | Description |
|------|-------------|
| DC-1 | Decision records table created via numbered migration (003) |
| DC-2 | Decision status tracks approved, rejected, deferred, superseded |
| DC-3 | Decision links to entity (who authorized) |
| DC-4 | Decision produces evidence using existing EvidenceCategory |
| DC-5 | Decision links to receipt using existing receipt envelope |
| DC-6 | All existing 89 tests still pass |
| DC-7 | No new governance concepts introduced |

## Sequence Position

```
ENTITY-001 ──► DECISIONS-001 ──► PERMISSIONS-001 ──► MCP UF-001
     ✅            ⏳               ⏳                   ⏳
```
