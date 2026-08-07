# DIVERGENCE STARTUP-RECEIPT-SHAPE-001 — Receipt Shape Mismatch

**Status:** Resolved (contract clarification)
**Date:** 2026-08-06
**Protocol:** `docs/architecture/RUST-CORE-CONFORMANCE-SPECIFICATION.md` §9 (Divergence Protocol)
**Recorded by:** RUST-MIGRATION-M0A checkpoint

---

## Summary

**DIVERGENCE:** Receipt shape mismatch

**Classification:** Contract clarification

**Resolution:** Canonical receipt contract supersedes platform harness receipt.

**Evidence:**
- `STARTUP-HARNESS-CANONICALIZATION-1` (`TheLibrarian@15c5ef2`, `docs/planning/`)
- `SH-001` — Canonical Contract Extraction
- `SH-005` — Cross-Platform Equivalence Validation
- `EQ-004` — Receipt equivalence check

---

## Detailed Record

| Field | Value |
|-------|-------|
| **Input** | Startup of a governed node (identity, governance, capabilities, environment) |
| **Expected (contract)** | `schemas/startup-receipt.schema.json` — canonical node startup receipt (13 required fields, platform-neutral) |
| **Swift behavior** | Emits `startup-execution-receipt-v1` (`receipts/startup-run-sr-carbideframe-*.json`): step sequence with durations, hooks, findings, MCP health probe over HTTP, title derivation. Pre-canonicalization harness artifact (macOS CarbideFrame `SessionStartup/`) |
| **Rust behavior** | Emits canonical receipt per `schemas/startup-receipt.schema.json`; deterministic facts equivalent to Swift harness outcome |
| **Disposition** | Canonical contract supersedes the platform harness receipt. Swift observable facts (startup phases, order, pass/fail outcomes) are preserved through the canonical mapping. The `startup-execution-receipt-v1` shape is NOT reproduced — doing so would be a regression |

**Owner decision (2026-08-06):** Do not reproduce the Swift `startup-execution-receipt-v1` shape. The migration target is "Make Rust produce the canonical startup receipt contract while preserving equivalent observable facts" — not "Make Rust emit the old Swift harness JSON."

---

## Equivalence Mapping (SH-005 / EQ-004)

```
Swift startup-execution-receipt-v1
          |
          | SH-005 / EQ-004 mapping
          v
canonical startup-receipt-v1
```

| Swift fact | Canonical field |
|------------|-----------------|
| `startup.terminal_state: READY` | `status: GOVERNED_EXECUTION`, `startup_phase: complete` |
| All `startup.sequence[]` steps `passed` | `checks_passed: 6`, `checks_failed: 0` |
| Identity resolved (project/node) | `identity_loaded: true`, `node_id`, `platform` |
| Governance verified (guard posture, ledger, drift) | `governance_verified: true` |
| Capabilities loaded | `capabilities_loaded: true` |
| Environment validated (health, storage) | `environment_validated: true` |
| `hooks` (health-check, guard-posture, drift-scan) | collapsed into `environment_validated` / `governance_verified` outcomes |
| `title`, `sources`, `manifest_version` | harness-only; out of canonical contract scope |

---

## Consequences

1. **RUST-M0-7 oracle:** The equivalence oracle is the canonical receipt contract plus the reference receipt `evidence/phase0/reference-architecture/startup-receipt-windows.json`. Swift equivalence is demonstrated through the mapping above, not byte-level receipt equality.
2. **No-networking constraint is consistent:** the canonical contract has no network probe requirement (the Swift harness's MCP health probe is a harness artifact, not a contract requirement).
3. **M0A fixture anchor:** `conformance/fixtures/startup/canonical-startup-input.json` + `expected-startup-receipt.json` encode this resolution as the deterministic migration target.
