# EPIC-PERMISSIONS-001

**Status:** Planning — not yet authorized
**Prerequisites:** STORAGE-001 ✅, ENTITY-001 ✅, DECISIONS-001 ✅

---

## Objective

Add capability access mapping to the governance database — persistent storage for which entities may access which capabilities, backed by recorded decisions.

## Why This Exists

The governance database can currently answer:

| Question | Answer | Source |
|----------|--------|--------|
| What exists? | ✅ Entity registry |
| What was approved? | ✅ Decision records |
| **Who may do what?** | ❌ Not persisted |

Permissions do not become the source of authority. They reference decisions. Every permission maps back through: Permission → Decision → Entity → Evidence → Receipt.

## Scope

| Component | Description |
|-----------|-------------|
| Permissions table (migration 004) | Entity → capability access mapping |
| Permission lifecycle state | Active, suspended, revoked |
| Decision reference | Every permission references the authorizing decision |
| Scope constraints | Optional capability-level or entity-level scope |

## Non-Scope

```
Authentication provider:        0 (future)
MCP protocol logic:             0 (→ UF-001)
Runtime enforcement code:       0 (→ UF-001)
Platform account mapping:       0 (future)
Role-based access control:      0 (beyond entity-type)
Policy evaluation engine:       0 (future)
```

## Acceptance Gates

| Gate | Description |
|------|-------------|
| PM-1 | Permissions table created via numbered migration (004) |
| PM-2 | Permission links entity to capability |
| PM-3 | Permission references a recorded decision |
| PM-4 | Permission lifecycle: active, suspended, revoked |
| PM-5 | Permission produces evidence using existing EvidenceCategory |
| PM-6 | All existing tests still pass |
| PM-7 | No new governance concepts introduced |

## Sequence Position

```
ENTITY-001 ──► DECISIONS-001 ──► PERMISSIONS-001 ──► MCP UF-001
     ✅              ✅               ⏳                  ⏳
```
