# EPIC-ENTITY-REGISTRY-1

**Status:** Planning — not yet authorized
**Prerequisite:** STORAGE-001 (numbered migrations) ✅

---

## Objective

Add persistent entity, actor, and capability ownership tracking to the governance database. This is the missing primitive that enables multi-user identity, capability ownership, permission assignment, and audit attribution.

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

Without entity storage, the governance database knows what happened but not who authorized it, who performed it, or who owns the capability.

## Scope

| Component | Description |
|-----------|-------------|
| Entity table | Durable storage for actors, nodes, capabilities, and resources |
| Entity type classification | Distinguish human, agent, node, capability, resource |
| Entity ownership | Track parent-child relationships between entities |
| Entity lifecycle | Active/suspended/retired states for entities |
| Entity evidence | Every entity registration/revision produces evidence |

## Non-Scope

```
New governance concepts:        0
New receipt types:              0
New evidence categories:        0
New lifecycle states:           0
New residency states:           0
Permission enforcement:         0 (next sprint)
Decision records:               0 (next sprint)
MCP server:                     0 (future epic)
Authentication protocol:        0 (future epic)
```

## Data Model

```sql
-- Migration NNN: entity_registry
CREATE TABLE IF NOT EXISTS entities (
    entity_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL CHECK (entity_type IN (
        'human', 'agent', 'node', 'capability', 'resource', 'organization'
    )),
    display_name TEXT NOT NULL,
    external_id TEXT,                           -- identity provider reference
    parent_entity_id TEXT REFERENCES entities(entity_id),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'retired')),
    metadata TEXT DEFAULT '{}',                 -- JSON metadata
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    registered_by TEXT NOT NULL
);

CREATE INDEX idx_entities_type ON entities(entity_type);
CREATE INDEX idx_entities_parent ON entities(parent_entity_id);
```

## Architecture

The entity registry integrates with existing contracts:

```
NodeIdentity (contract type)
    ↓
Entity record (storage)
    └── type = 'node'
    └── external_id = NodeId
    └── parent = owning organization

Capability (contract type)
    ↓
Entity record (storage)
    └── type = 'capability'
    └── external_id = capability_id
    └── parent = owning node or human
```

Usage pattern:

```rust
// Register a node identity in the entity registry
let entity = EntityRecord {
    entity_id: "node-windows-1",
    entity_type: EntityType::Node,
    display_name: "Windows Runtime Node 1",
    external_id: Some(node_id.as_str()),
    parent_entity_id: Some("org-librarian"),
    status: EntityStatus::Active,
    metadata: serde_json::json!({"platform": "windows", "version": "0.1.0"}),
    registered_at: now,
    registered_by: "migration-runner",
};
registry.register_entity(&entity)?;

// Later: associate a capability with that node
let cap = CapabilityOwnership {
    capability_id: "model-phi-4",
    owner_entity_id: "node-windows-1",
    granted_at: now,
    granted_by: "owner",
};
registry.grant_capability(&cap)?;
```

## Acceptance Gates

| Gate | Description |
|------|-------------|
| ER-1 | Entity table created via numbered migration |
| ER-2 | Entity types cover human, agent, node, capability, resource, organization |
| ER-3 | Entity registration produces evidence using existing EvidenceCategory |
| ER-4 | Entity lifecycle (active/suspended/retired) tracked |
| ER-5 | Parent-child relationships supported |
| ER-6 | All existing tests still pass |
| ER-7 | No new governance concepts introduced |

## Dependencies

- STORAGE-001: ✅ Complete — migration framework exists for adding the entity table

## Follow-on Sprint

After ENTITY-001, the next natural sprint is DECISIONS-001 (persistent decision records for owner authorizations), then PERMISSIONS-001 (capability-to-entity access mapping).

```
ENTITY-001 ──► DECISIONS-001 ──► PERMISSIONS-001 ──► MCP Auth
```
