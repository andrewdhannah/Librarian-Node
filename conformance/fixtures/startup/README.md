# Startup Fixtures — M0A Migration Anchor

These fixtures are the deterministic target for `RUST-MIGRATION-M0A` (Deterministic Startup Engine Foundation).

## Files

| File | Purpose |
|------|---------|
| `canonical-startup-input.json` | The canonical input the Rust startup engine receives: node directory (identity, governance sync, capabilities), SQLite intent, and the expected startup contract outcome. Self-contained — no external reads needed to run the engine test. |
| `expected-startup-receipt.json` | The expected canonical startup receipt (per `schemas/startup-receipt.schema.json`). Deterministic fields are the equivalence target. |

## Test Form

The first Rust M0A test is fixture-driven:

```
Given canonical startup fixture          (canonical-startup-input.json)
When  Rust startup engine executes       (6-phase STARTUP-PROTOCOL)
Then  emitted receipt matches expected   (expected-startup-receipt.json)
```

## Field Policy

- **Deterministic fields (must match exactly):** `node_id`, `platform`, `governance_commit`, `startup_phase`, `identity_loaded`, `governance_verified`, `capabilities_loaded`, `environment_validated`, `checks_passed`, `checks_failed`, `status`
- **Variable fields (must be present and valid, value may differ):** `receipt_id` (pattern `^[A-Z0-9-]+$`), `timestamp` (ISO-8601)

Per `docs/architecture/THREE-WAY-EQUIVALENCE-PROTOCOL.md` and `docs/architecture/RUST-CORE-CONFORMANCE-SPECIFICATION.md` §4 (equivalence is semantic, not bit-identical).

## Provenance

- Input fixture derived from: `platform/windows/node-identity.json`, `platform/windows/capabilities.json`, `governance-sync.json` (runtime node).
- Expected receipt derived from the reference receipt: `evidence/phase0/reference-architecture/startup-receipt-windows.json`.
- Swift equivalence mapping: `conformance/divergences/STARTUP-RECEIPT-SHAPE-001.md` (SH-005 / EQ-004).
- Canonical SQL schema source: `capability-registry/schema/capability-registry-schema.sql` (`TheLibrarian@15c5ef2`).
