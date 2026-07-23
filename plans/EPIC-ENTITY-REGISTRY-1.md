# EPIC-ENTITY-REGISTRY-1

**Status:** Planning — not yet authorized
**Prerequisite:** STORAGE-001 (numbered migrations) ✅

---

## Objective

Add persistent entity storage to the governance database — referenceable records for actors, nodes, capabilities, and resources that participate in governed execution.

This is the first step from a governed execution engine into a governed multi-actor system.

## Boundary

ENTITY-001 answers only:

> Can governance refer to the things participating in execution?

It does NOT answer:

- What actions may an entity perform? (→ PERMISSIONS-001)
- What decisions has the owner made? (→ DECISIONS-001)
- Is this request authorized? (→ MCP layer)

Each question belongs to a later sprint. The separation prevents authorization logic from being coupled to entity registration.

## Why This Exists

The governance database currently tracks:

| What | Table | Status |
|------|-------|--------|
| Schema versions | schema_version | ✅ |
| Migration history | migration_log | ✅ |
| Lifecycle cursors | lifecycle_cursors | ✅ |
| Custody events | custody_events | ✅ |
| Evidence records | evidence_records | ✅ |
| Receipts | receipts + receipt_parents | ✅ |
| **Entities (actors, nodes, resources)** | **(missing)** | ❌ |
| **Decisions (authorizations)** | **(missing)** | ❌ |
| **Permissions (capability access)** | **(missing)** | ❌ |

Without entity storage, the governance database knows what happened but not who participated.

## Scope

| Component | Description |
|-----------|-------------|
| Entity table (migration 002) | Durable storage for actors, nodes, capabilities, resources |
| Entity type classification | Distinguish human, agent, node, capability, resource, organization |
| Parent-child relationships | Track entity ownership hierarchy |
| Entity lifecycle status | Active / suspended / retired |
| Entity evidence | Every registration/revision produces evidence |

## Non-Scope

```
New governance concepts:        0
New receipt types:              0
New evidence categories:        0
New lifecycle states:           0
New residency states:           0
Permission enforcement:         0 (→ PERMISSIONS-001)
Decision records:               0 (→ DECISIONS-001)
Authentication providers:       0 (future)
MCP server:                     0 (→ UF-001)
Organizations/tenants:          0 (beyond parent-child)
Roles/groups:                   0 (beyond entity types)
```

## Data Model

```sql
-- Migration 002: entity_registry
-- This is additive only. Entities table is independent.
CREATE TABLE IF NOT EXISTS entities (
    entity_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL CHECK (entity_type IN (
        'human', 'agent', 'node', 'capability', 'resource', 'organization'
    )),
    display_name TEXT NOT NULL,
    external_id TEXT,                           -- reference to external identity
    parent_entity_id TEXT REFERENCES entities(entity_id),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'retired')),
    metadata TEXT DEFAULT '{}',                 -- JSON metadata (platform info, etc.)
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    registered_by TEXT NOT NULL
);

CREATE INDEX idx_entities_type ON entities(entity_type);
CREATE INDEX idx_entities_parent ON entities(parent_entity_id);
```

## Contract Integration

The entity registry maps existing contract types to storage:

```
NodeIdentity (contract)  →  Entity (type=node, external_id=NodeId)
Capability (contract)    →  Entity (type=capability, external_id=capability_id)
NodeRole (contract)      →  Entity type classification
CustodyEvent (contract)  →  References entity_id as actor
EvidenceRecord (contract)→  Produced_by references entity_id
Receipt (contract)       →  Initiated_by references entity_id
```

## Acceptance Gates

| Gate | Requirement |
|------|-------------|
| ER-1 | Entity table created via numbered migration (002) |
| ER-2 | Entity types cover human, agent, node, capability, resource, organization |
| ER-3 | Entity registration produces evidence using existing EvidenceCategory |
| ER-4 | Entity lifecycle (active/suspended/retired) tracked |
| ER-5 | Parent-child relationships supported |
| ER-6 | All existing 73 tests still pass |
| ER-7 | No new governance concepts introduced |

## What It Enables

```
ENTITY-001
   |
   ├── "Andrew's node"
   ├── "Windows node"
   ├── "Linux node"
   ├── "MCP client"
   └── "Capability provider"

       ↓

DECISIONS-001          "Owner approved capability X to entity Y"
PERMISSIONS-001        "Entity A may invoke capability B"
UF-001                 "Request enters governance with known identity"
```

## Dependencies

- STORAGE-001: ✅ Complete — migration framework exists for entity table

## Follow-on Sequence

```
ENTITY-001 ──► DECISIONS-001 ──► PERMISSIONS-001 ──► MCP Auth (UF-001)
```

Each sprint adds one table via numbered migration, zero new governance concepts, preserves all existing tests.
