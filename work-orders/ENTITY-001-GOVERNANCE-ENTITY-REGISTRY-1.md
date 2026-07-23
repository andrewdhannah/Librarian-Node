# ENTITY-001: GOVERNANCE-ENTITY-REGISTRY-1

**Status:** Authorized
**Repository:** Librarian-Node
**Prerequisites:** STORAGE-001 (numbered migrations) ✅

---

## Purpose

Add persistent entity storage to the governance database — referenceable records for actors, nodes, capabilities, and resources that participate in governed execution.

## Adds

- Entity persistence (entities table, migration 002)
- Entity identifiers and classification
- Entity lifecycle (active/suspended/retired)
- Entity evidence generation

## Does Not Add

- Authentication
- Authorization
- Permissions
- MCP logic
- Platform identity rules
- User access policy

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
