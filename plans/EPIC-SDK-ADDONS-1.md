# EPIC-SDK-ADDONS-1

**Status:** Planning — not yet authorized
**Prerequisites:** ENTITY-001 ✅, DECISIONS-001 ✅, PERMISSIONS-001 ✅, UF-001 ⏳

---

## Objective

Implement the add-on SDK: the capability declaration interface, execution context, governance client, storage client, lifecycle management, health reporting, and migration contract that allow third parties to contribute governed capabilities.

## Scope

| Component | Description |
|-----------|-------------|
| SDK crate (`librarian-sdk`) | Shared add-on development library |
| Capability declaration | Register capabilities with CapabilityRegistry |
| Add-on manifest | Identity document for discovery (version, storage, permissions) |
| Add-on lifecycle | Installed → Initializing → Ready → Degraded → Disabled → Removed |
| Health reporting | Per-capability health status for the router |
| Execution context | Entity identity, decision, permission, parameters |
| Governance client | Custody, residency, evidence, receipts (middleware) |
| Storage client | Private per-add-on SQLite database, backup, health |
| Migration contract | Schema migration trait for add-on storage ownership |
| Provenance contract | Source tracking for ingested data |

## Non-Scope

```
New governance primitives:        0
New receipt types:              0
New evidence categories:        0 (pending InformationProcessing)
New CapabilityCategory:         1 (InformationProcessing) — contracts change
Core governance changes:        0
Platform adapter changes:       0
```

## Acceptance Gates

| Gate | Description |
|------|-------------|
| SDK-1 | SDK crate exists, compiles, depends only on librarian-contracts |
| SDK-2 | Add-on manifest declares identity, version, capabilities, storage |
| SDK-3 | Add-on lifecycle tracked (Installed → Initializing → Ready → Degraded → Disabled → Removed) |
| SDK-4 | Health reporting per capability |
| SDK-5 | Governance client provides custody, evidence, receipt middleware |
| SDK-6 | Storage client provisions private per-add-on database |
| SDK-7 | Migration contract allows add-on schema evolution |
| SDK-8 | Provenance contract records source, hash, timestamp |
| SDK-9 | No new governance primitives introduced |

## Reference Implementation

The conversation ingestion add-on (Claude/ChatGPT archive converter)
is the reference implementation for this epic. It exercises:
capability declaration, manifest, lifecycle, health, storage,
migrations, provenance, evidence, and multi-capability routing.

## Dependencies

- UF-001 (MCP server) for external invocation — or add-on can work locally
- ENTITY-001, DECISIONS-001, PERMISSIONS-001 for identity/auth chain
