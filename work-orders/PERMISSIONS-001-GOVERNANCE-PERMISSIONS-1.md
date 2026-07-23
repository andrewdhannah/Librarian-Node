# PERMISSIONS-001: GOVERNANCE-PERMISSIONS-1

**Status:** Authorized
**Repository:** Librarian-Node
**Prerequisites:** STORAGE-001 ✅, ENTITY-001 ✅, DECISIONS-001 ✅

---

## Purpose

Add capability access mapping to the governance database. Permissions reference recorded decisions — they do not create authority.

## Adds

- Permissions table (migration 004)
- Entity → capability access mapping
- Permission lifecycle (active, suspended, revoked)
- Decision references
- Evidence generation

## Does Not Add

- Authentication provider
- MCP protocol logic
- Runtime enforcement code
- Platform account mapping
- Role-based access control
- Policy evaluation engine

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
