# Canonical Capability Registry Schema (embedded)

Byte-identical copies of the frozen canonical SQLite schema, embedded into
`librarian-core` via `include_str!` so the M0A runtime is self-contained and
deterministic (no external file lookup at runtime).

## Provenance

| Property | Value |
|---|---|
| Source repository | TheLibrarian (Swift canonical reference) |
| Frozen commit | `15c5ef2c1d4d4ba72af2a765f0bc4ce8bd4b16cc` (sprint-3) |
| Source paths | `capability-registry/schema/capability-registry-schema.sql` (Phase 2, 9 tables) |
| | `capability-registry/schema/capability-registry-schema-phase3.sql` (Phase 3, 2 tables) |
| Copied into workspace | 2026-08-06 (RUST-MIGRATION-M0A) |
| Modification | None — files are byte-identical to the frozen sources |

## Integrity

Expected SHA-256 hashes (canonical, recorded in the M0-0 contract surface
manifest and verified by the drift-guard test):

- `capability-registry-schema.sql`: computed at snapshot time, recorded in
  `conformance/contract-surface/contract-surface-manifest.json`
- `capability-registry-schema-phase3.sql`: same

Total tables: 11 (9 Phase-2 + 2 Phase-3). All DDL uses `CREATE TABLE IF NOT
EXISTS` with no transaction wrappers, so `execute_batch` application is
idempotent.

## Drift policy

These files are frozen by conformance policy. Any change must be recorded as a
contract clarification per the divergence protocol
(`conformance/divergences/STARTUP-RECEIPT-SHAPE-001.md` precedent) and re-baselined
against TheLibrarian `@15c5ef2`.
