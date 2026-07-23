# STORAGE-001: GOVERNANCE-PERSISTENCE-MATURITY-1

**Status:** Authorized
**Repository:** Librarian-Node

---

## Purpose

Enable controlled schema evolution for the governance database. The Rust side changes from a validated governance substrate into an operationally durable governance substrate.

## Allowed Scope

- Migration framework (`schema_version` + `migration_log` tables)
- Deterministic migration execution (up/down)
- Migration evidence and receipt generation
- Convert existing batch schema creation to migration 001

## Non-Scope

```
New governance concepts:        0
New receipt types:              0
New evidence categories:        0
New authorization models:       0
Identity enforcement:           0
MCP functionality:              0
UI migration:                   0
Application database:           0
Multi-user workflows:           0
```

## Acceptance Gates

| Gate | Description |
|------|-------------|
| SM-1 | Migration framework creates schema_version + migration_log tables |
| SM-2 | Existing schema creation converted to migration 001 |
| SM-3 | Migration runner applies pending migrations on open |
| SM-4 | Each migration produces evidence and a receipt |
| SM-5 | Migration history is queryable from migration_log |
| SM-6 | All existing 67 tests still pass |
| SM-7 | No new governance concepts introduced |
