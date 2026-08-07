# Subsystem Conformance: Startup Engine

**Version:** 1.0.0
**Status:** Pending
**Subsystem #:** 4 (Milestone 0 target — first implemented)

---

## 1. Contract Source

- `contracts/startup/STARTUP-PROTOCOL.md` — the 6-phase startup sequence (authoritative, this repo)
- `contracts/startup/STARTUP-OUTPUT-CONTRACT.md` — startup receipt format
- `contracts/startup/SESSION-IDENTITY-CONTRACT.md` — node identity requirements
- `schemas/startup-receipt.schema.json` — receipt JSON Schema
- Swift reference behavior: `TheLibrarian@15c5ef2` startup implementation and its produced receipts

## 2. Expected Inputs

- Node directory containing:
  - `node-identity.json` (or `node-id.json` legacy format)
  - `governance-sync.json` (verified governance state)
  - `capabilities.json`
- SQLite database path (existing or to be created)

## 3. Expected Outputs

- Startup receipt JSON conforming to `schemas/startup-receipt.schema.json`:

```json
{
  "receipt_id": "<unique-receipt-id>",
  "node_id": "<node-identifier>",
  "platform": "<windows|linux|macos>",
  "governance_commit": "<commit-sha>",
  "startup_phase": "complete",
  "identity_loaded": true,
  "governance_verified": true,
  "capabilities_loaded": true,
  "environment_validated": true,
  "checks_passed": 6,
  "checks_failed": 0,
  "status": "GOVERNED_EXECUTION",
  "timestamp": "<iso-8601-timestamp>"
}
```

- Exit code 0 on success; 1 on startup failure

## 4. Receipt Format

- `schemas/startup-receipt.schema.json` (this repo)
- Receipt must pass JSON Schema validation
- Receipt must be produced identically for identical inputs (determinism)

## 5. Qualification Tests

| Test ID | Source | Requirement |
|---------|--------|-------------|
| RECEIPT-SCHEMA-001 | Sealed suite (TheLibrarian@15c5ef2) | Receipt passes schema validation |
| RUST-M0-5 | This spec §8 | 6-phase startup executes per protocol |
| RUST-M0-6 | This spec §8 | Receipt conforms to schema |
| RUST-M0-7 | This spec §8 | Receipt equivalent to Swift reference receipt |
| RUST-M0-8 | This spec §8 | Deterministic across repeated runs |

## 6. Golden Fixtures

- Reference receipts from Swift baseline: `TheLibrarian@15c5ef2` startup receipts (canonical)
- Cross-platform receipts: `evidence/phase0/reference-architecture/startup-receipt-{windows,linux,macos}.json` (this repo)
- Node fixture files: `platform/<os>/node-identity.json`, `capabilities.json`

## 7. Performance Expectations

- Startup completes within a bounded time (target: < 5 s cold start on reference hardware; exact bound measured and recorded)
- No network access during startup (deterministic fast path)
- Startup is single-threaded

## 8. Conformance Level Target

- Target: **C2 (Evidence Compatible)** for Milestone 0
- Must demonstrate:
  - Receipts have required fields (RECEIPT-SCHEMA-001)
  - Receipts are immutable (stored append-only)
  - Evidence chains are complete (startup receipt linked to governance verification evidence)
  - Historical receipts are interpretable (schema versioned)
