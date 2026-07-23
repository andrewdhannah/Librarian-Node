# Sprint Specification: WIN-SPRINT-LEDGER-1

**Status:** Active
**Date:** 2026-06-30
**Phase:** Phase 0a — Harness Core
**Dependencies:** WIN-HARNESS-BASELINE-DIFF-1, WIN-RUST-TOOLCHAIN-DRIFT-TRIAGE-1

---

## 1. Purpose

Create a durable, machine-parseable sprint-ledger surface so future agents can verify sprint continuity without relying only on SESSION-HANDOFF.md prose.

Addresses planning document S-05 (WIN-SPRINT-LEDGER-1) from `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md`.

---

## 2. Allowed Mutation Scope

| Path | Action |
|------|--------|
| `project-state/sprint-ledger.json` | **Create** — machine-readable sprint ledger |
| `scripts/harness/validate-sprint-ledger.ps1` | **Create** — ledger validation script |
| `docs/sprints/WIN-SPRINT-LEDGER-1.md` | **Create** — this sprint specification |
| `docs/receipts/WIN-SPRINT-LEDGER-1-RECEIPT.md` | **Create** — closeout receipt |
| `SESSION-HANDOFF.md` | Update if required by repo convention |
| `project-state/` | **Create** directory if absent |

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
| LG-01 | `project-state/sprint-ledger.json` exists and is valid JSON | PASS |
| LG-02 | Ledger records all 22 sealed Windows PC sprints from Phase 0 chain | PASS |
| LG-03 | Each sprint entry has all required fields | PASS |
| LG-04 | Receipt paths reference existing files | PASS |
| LG-05 | Sprint doc paths reference existing files | PASS |
| LG-06 | `scripts/harness/validate-sprint-ledger.ps1` parses cleanly under PowerShell 5.1 | PASS |
| LG-07 | Validator exits 0 on the created ledger | PASS |
| LG-08 | Validator exits 1 on deliberately invalid temp copy | PASS |
| LG-09 | `pre-mutation-check.ps1` still passes on final tree | PASS |
| LG-10 | No service/model/runtime/environment files changed | PASS |

---

## 5. Ledger Schema

The ledger is a JSON file at `project-state/sprint-ledger.json` with this structure:

### Top-level fields

| Field | Type | Description |
|-------|------|-------------|
| `schema_version` | string | Schema version identifier (e.g. "1.0.0") |
| `generated_at` | string | ISO 8601 timestamp of ledger generation |
| `generated_by` | string | Sprint ID that generated this ledger |
| `current_head` | object | `{ full, short, branch }` — current git HEAD |
| `origin_sync_state` | string | `"in_sync"` or `"ahead"` or `"behind"` or `"diverged"` |
| `active_sprint` | string or null | Currently active sprint ID, or null |
| `next_authorized_sprint` | string | Next sprint authorized for work |
| `sprints` | array | Ordered array of sprint entries |

### Sprint entry fields

| Field | Type | Description |
|-------|------|-------------|
| `sprint_id` | string | Unique sprint identifier (e.g. "WIN-HARNESS-...") |
| `status` | string | One of: `sealed`, `ready_for_review`, `active`, `planned` |
| `commit` | string | Git commit SHA (short) that sealed the sprint |
| `pushed` | bool | Whether the commit has been pushed to origin |
| `branch` | string | Git branch name |
| `receipt_path` | string or null | Repo-relative path to closeout receipt, or null |
| `sprint_doc_path` | string or null | Repo-relative path to sprint spec, or null |
| `primary_files` | array | Key files delivered by the sprint |
| `category` | string | Sprint category (harness, runtime, planning, maintenance, init) |
| `phase` | string | Sprint phase identifier |
| `owner_review_required` | bool | Whether Owner review is needed before sealing |
| `next_sprint` | string | Intended next sprint after this one |
| `notes` | string | Free-text notes about the sprint |

---

## 6. Required Preflight Checks

1. HEAD matches `59f4cba`
2. Working tree is clean
3. `origin/main` is in sync
4. `pre-mutation-check.ps1` passes
5. All existing harness scripts parse cleanly under PowerShell 5.1
6. `docs/receipts/WIN-RUST-TOOLCHAIN-DRIFT-TRIAGE-1-RECEIPT.md` exists
7. `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md` exists

---

## 7. Closeout Requirements

1. `project-state/sprint-ledger.json` — valid JSON, all sprints recorded
2. `scripts/harness/validate-sprint-ledger.ps1` — parses, validates, exits 0/1
3. Ledger validator passes on the created ledger (exit 0)
4. Ledger validator fails on deliberately invalid ledger (exit 1)
5. `pre-mutation-check.ps1` still passes (exit 0)
6. Receipt documenting PASS/FAIL for each acceptance gate
7. Working tree clean (or classified) after creation
8. HEAD unchanged from starting state
9. Origin sync status recorded
10. No unexpected changed files outside allowed mutation scope

---

## 8. Recommended Next Sprint

**WIN-RUST-PATH-RESTORE-1** — Recreate the rustup proxy shim directory (%USERPROFILE%\.cargo\bin\) to restore rustc/cargo PATH access. This was the recommended sprint before WIN-SPRINT-LEDGER-1 was interleaved and remains the highest-priority repair sprint.

---

## 9. References

- `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md` § S-05 (WIN-SPRINT-LEDGER-1)
- `docs/sprints/WIN-HARNESS-BASELINE-DIFF-1.md` (predecessor tooling)
- `docs/sprints/WIN-RUST-TOOLCHAIN-DRIFT-TRIAGE-1.md` (predecessor sprint)
- `project-state/sprint-ledger.json` (the ledger itself)
- `scripts/harness/validate-sprint-ledger.ps1` (ledger validator)
