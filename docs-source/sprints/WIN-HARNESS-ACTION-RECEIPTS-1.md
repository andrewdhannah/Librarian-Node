# Sprint Specification: WIN-HARNESS-ACTION-RECEIPTS-1

**Status:** Sealed
**Date:** 2026-06-30
**Phase:** Phase 0b — Harness Enhancement
**Dependencies:** WIN-RUST-PATH-RESTORE-1

---

## 1. Purpose

Implement granular action receipt generation for discrete Windows harness actions. Action receipts capture individual bounded execution events inside a sprint — such as preflight run, postflight run, contract-check run, baseline-diff run, receipt generation, or ledger validation — so future agents and The Librarian can audit exactly what was run, with inputs, outputs, exit codes, timestamps, and custody classification.

Addresses planning document S-06 (WIN-HARNESS-ACTION-RECEIPTS-1) from `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md`.

---

## 2. Allowed Mutation Scope

| Path | Action |
|------|--------|
| `scripts/harness/new-action-receipt.ps1` | **Create** — action receipt generator |
| `docs/sprints/WIN-HARNESS-ACTION-RECEIPTS-1.md` | **Create** — this sprint specification |
| `docs/receipts/WIN-HARNESS-ACTION-RECEIPTS-1-RECEIPT.md` | **Create** — closeout receipt |
| `docs/receipts/actions/` | **Create** directory for action receipts |
| `project-state/sprint-ledger.json` | **Update** — add this sprint entry, update current_head |
| `SESSION-HANDOFF.md` | **Update** if required by repo convention |

---

## 3. Forbidden Actions

- No service start or stop
- No model workload
- No runtime/router/model code change
- No Rust repair
- No environment repair
- No firewall change
- No auto-start change
- No application work
- No broad agent autonomy

---

## 4. Acceptance Gates

| Gate | Description | Expected Result |
|------|-------------|-----------------|
| AR-01 | `scripts/harness/new-action-receipt.ps1` parses cleanly under PowerShell 5.1 | PASS |
| AR-02 | Valid input generates deterministic Markdown receipt | PASS |
| AR-03 | Optional JSON output is valid JSON | PASS |
| AR-04 | Missing required fields exits 1 | PASS |
| AR-05 | Invalid action type exits 1 | PASS |
| AR-06 | Invalid result exits 1 | PASS |
| AR-07 | Repeated runs with identical inputs produce identical output | PASS |
| AR-08 | Action receipt path documented (docs/receipts/actions/) | PASS |
| AR-09 | `sprint-ledger.json` remains valid | PASS |
| AR-10 | `pre-mutation-check.ps1` still passes on final tree | PASS |
| AR-11 | `postflight-check.ps1` passes with changed-file allowlist | PASS |
| AR-12 | No service/model/runtime/environment files changed | PASS |

---

## 5. Action Receipt Schema

The action receipt generator (`new-action-receipt.ps1`) supports the following fields:

### Required Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `ActionId` | string | Unique action identifier (e.g. "WIN-HARNESS-AR-001") |
| `SprintId` | string | Sprint identifier this action belongs to |
| `ActionType` | string | One of the recognized action types (see below) |
| `CommandInvoked` | string | Command or script that was invoked |
| `CustodyClass` | string | Custody classification (e.g. "read_only", "controlled_mutation", "audit") |
| `AllowedMutationScope` | string | Comma-separated list of file globs/directories allowed to change |
| `ForbiddenMutationScope` | string | Comma-separated list of file globs/directories forbidden from changing |
| `StartingHead` | string | HEAD before the action was performed |
| `EndingHead` | string | HEAD after the action was performed |
| `ExitCode` | int | Process exit code from the action |
| `Result` | string | PASS, FAIL, or PARTIAL |
| `OutputPath` | string | Output path for the receipt Markdown file |

### Optional Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `WorkingTreeBefore` | string | Working tree state before the action |
| `WorkingTreeAfter` | string | Working tree state after the action |
| `EvidencePaths` | string[] | Array of evidence file paths |
| `Notes` | string[] | Array of notes or findings strings |
| `JsonOutputPath` | string | Optional path for JSON output |
| `Timestamp` | switch | Include real timestamp (non-deterministic) |

### Recognized Action Types

| Type | Description |
|------|-------------|
| `preflight_check` | Pre-mutation custody gate execution |
| `postflight_check` | Post-flight verification execution |
| `contract_runner` | Contract check runner execution |
| `baseline_diff` | Baseline drift detection execution |
| `ledger_validation` | Sprint ledger validation execution |
| `receipt_generation` | Receipt generation itself |
| `toolchain_check` | Toolchain version/availability check |
| `manual_owner_action` | Action initiated by human Owner |
| `read_only_investigation` | Read-only investigation or audit |

### Output Path Convention

- **Markdown:** `docs/receipts/actions/<ACTION-ID>.md`
- **JSON (optional):** `docs/receipts/actions/<ACTION-ID>.json`

---

## 6. Required Preflight Checks

1. HEAD matches `44d1bcf`
2. Working tree is clean
3. `origin/main` is in sync
4. `pre-mutation-check.ps1` passes
5. All existing harness scripts parse cleanly under PowerShell 5.1
6. `baseline-diff.ps1 -Section rust_version` exits 0
7. `docs/receipts/WIN-RUST-PATH-RESTORE-1-RECEIPT.md` exists
8. `project-state/sprint-ledger.json` is valid

---

## 7. Closeout Requirements

1. `scripts/harness/new-action-receipt.ps1` — parses cleanly under PowerShell 5.1
2. Valid input generates deterministic Markdown receipt
3. Optional JSON output generates valid JSON
4. Missing required fields exits 1
5. Invalid action type exits 1
6. Invalid result exits 1
7. Repeated runs with identical inputs (no `-Timestamp`) produce identical output
8. `docs/sprints/WIN-HARNESS-ACTION-RECEIPTS-1.md` — this specification
9. `docs/receipts/WIN-HARNESS-ACTION-RECEIPTS-1-RECEIPT.md` — closeout receipt
10. `pre-mutation-check.ps1` still passes (exit 0)
11. `postflight-check.ps1` passes with changed-file allowlist
12. Working tree clean after mutation
13. `sprint-ledger.json` remains valid
14. No service/model/runtime/environment files changed

---

## 8. Recommended Next Sprint

**WIN-HARNESS-CUSTODY-LEDGER-1** — Implement a custody ledger that records every discrete action performed by the harness across sprints, providing a durable, machine-parseable audit trail. This sprint follows the action receipt infrastructure created in WIN-HARNESS-ACTION-RECEIPTS-1 and extends the sprint-ledger pattern to the action level. Note: WIN-HARNESS-CLEANUP-1 (C: drive space reclamation) should be considered only if a fresh baseline-diff or disk check confirms C: free space is critically low (below 10%).

---

## 9. References

- `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md` § S-06 (WIN-HARNESS-ACTION-RECEIPTS-1)
- `docs/sprints/WIN-RUST-PATH-RESTORE-1.md` (predecessor sprint)
- `scripts/harness/new-action-receipt.ps1` (the action receipt generator)
- `scripts/harness/new-sprint-receipt.ps1` (sprint-level receipt generator, pattern reference)
- `scripts/harness/pre-mutation-check.ps1` (pre-flight custody gate)
- `scripts/harness/postflight-check.ps1` (post-flight verification)
- `project-state/sprint-ledger.json` (sprint ledger)
