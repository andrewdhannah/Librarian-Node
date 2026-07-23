# WIN-HARNESS-CONTRACT-RUNNER-1

**Status:** SEALED
**Previous sprint:** WIN-HARNESS-RECEIPT-TEMPLATE-1 (SEALED)
**Repo:** `librarian-runtime-node`
**Platform:** Windows (PowerShell 5.1)
**Date:** 2026-06-30

---

## Sprint Summary

Build a unified Windows harness contract-test runner under `scripts/harness/` that wraps
existing repository test/check scripts and produces structured pass/fail output without
changing runtime, service, model, or environment state.

The three-tool harness core is now complete (pre-mutation gate, post-flight verification,
receipt generation). This sprint adds a single contract-runner entry point so that future
sprints can execute known validation scripts consistently and capture deterministic evidence.

---

## Scope

### In Scope
- `scripts/harness/run-contract-checks.ps1` -- Unified contract test runner
- `docs/sprints/WIN-HARNESS-CONTRACT-RUNNER-1.md` -- This sprint doc
- `docs/receipts/WIN-HARNESS-CONTRACT-RUNNER-1-RECEIPT.md` -- Closeout receipt
- `SESSION-HANDOFF.md` -- Update sprint table

### Out of Scope (Do Not)
- No service start or stop
- No model workload
- No runtime/router/model code change
- No environment repair
- No firewall change
- No auto-start change
- No app work
- No broad agent autonomy

---

## Starting Baseline

| Check | Value |
|-------|-------|
| Repo | `G:\OpenWork\librarian-runtime-node` |
| Branch | `main` |
| HEAD | `85060f8` -- `WIN-HARNESS-RECEIPT-TEMPLATE-1 receipt generator` |
| Working tree | Clean |
| Origin | Up to date |

---

## Durable State Verification (pre-work)

1. HEAD matches `85060f8`
2. git status is clean
3. origin/main is in sync
4. SESSION-HANDOFF.md reads correctly
5. `scripts/harness/pre-mutation-check.ps1` passes (11/11)
6. `scripts/harness/postflight-check.ps1` parses cleanly
7. `scripts/harness/new-sprint-receipt.ps1` parses cleanly
8. `docs/receipts/WIN-HARNESS-RECEIPT-TEMPLATE-1-RECEIPT.md` exists
9. `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md` exists

---

## Contract Runner Specification

### Purpose

`run-contract-checks.ps1` is a single entry point for executing known repository
validation scripts with consistent pass/fail/skip reporting. It is **not** a test
framework — it wraps existing scripts and reports their exit codes.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `-List` | switch | No * | List available checks without running them |
| `-CheckName` | string[] | No * | Run specific check(s) by exact name |
| `-AllSafe` | switch | No * | Run all `safe_readonly` checks |
| `-Json` | switch | No | Emit deterministic JSON result object to stdout |
| `-RepoRoot` | string | No | Auto-detected from script location |
| `-Quiet` | switch | No | Suppress human-readable output |
| | | | *One of `-List`, `-CheckName`, or `-AllSafe` is required |

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All selected checks pass (or `-List` mode) |
| 1 | Any selected check fails, or unknown check name |

### Check Registry

The runner registers 44 known checks across 7 categories:

#### SAFE -- READ-ONLY (22 checks)

These can run without service, model, or admin. The runner invokes each and reports
pass/fail from exit code.

| Name | Display | Command |
|------|---------|---------|
| `pre-mutation-check` | Pre-Mutation Custody Gate | `pre-mutation-check.ps1` |
| `runtime-status` | Runtime Operator Status | `operations/runtime-status.ps1` |
| `runtime-clean-check` | Runtime Clean State Check | `operations/runtime-clean-check.ps1` |
| `runtime-logs` | Runtime Log File Inventory | `operations/runtime-logs.ps1` |
| `list-models` | Model File Inventory | `list-models.ps1` |
| `check-model-registry` | Model Registry Validation | `check-model-registry.ps1` |
| `check-mcp-health` | MCP Connection Health Check | `check-mcp-health.ps1` |
| `health-check` | Backend Health Endpoint | `health-check.ps1` |
| `test-runtime-artifact-identity` | Qualification D1: Artifact Identity | `test-runtime-artifact-identity.ps1` |
| `test-runtime-profiles` | Qualification D5: Model Profile Envelope | `test-runtime-profiles.ps1` |
| `test-runtime-cleanup` | Qualification D7: Cleanup/Orphan Proof | `test-runtime-cleanup.ps1` |
| `test-operator-runbook` | Operator Runbook Validation | `tests/test-win-runtime-operator-runbook.py` |
| `test-dry-run-readiness` | Dry-Run Readiness Validation | `tests/test-win-runtime-dry-run-readiness.py` |
| `test-dry-run-gap-close` | Dry-Run Gap Close Validation | `tests/test-win-runtime-dry-run-gap-close.py` |
| `test-startup-custody-inventory` | Startup Inventory Custody Validation | `tests/test-startup-files-custody-inventory.py` |
| `test-custody-normalization` | Custody Normalization Regression | `tests/test-custody-normalization.py` |
| `test-context-route-contract` | Context-Route Contract Validation | `tests/test-context-route-contract.py` |
| `test-advisory-stub` | Advisory Stub Engine Tests | `tests/test-advisory-stub.py` |
| `test-router-context-design` | Router Context Design Validation | `tests/test-router-context-runtime-design.py` |
| `test-router-context-contract` | Router Context Contract Tests | `tests/test-router-context-runtime-contract.py` |
| `test-router-context-prototype` | Router Context Prototype Tests | `tests/test-router-context-prototype.py` |
| `test-mcp-template-reconciliation` | MCP Template Reconciliation | `tests/test-mcp-template-reconciliation.py` |

#### REQUIRES SERVICE (8 checks)

Skipped by the runner. Listed for discovery. Reason: "requires service running (router/backend)".

#### REQUIRES MODEL (4 checks)

Skipped by the runner. Listed for discovery. Reason: "requires model/runtime".

#### REQUIRES ADMIN (2 checks)

Skipped by the runner. Listed for discovery. Reason: "requires admin/operator".

#### REQUIRES PARAMETER (2 checks)

Skipped by the runner. These scripts need manual file-path arguments. Reason: "requires manual -Path parameter".

#### MUTATION-CAPABLE (5 checks)

Skipped by the runner. These scripts modify environment state or rebuild artifacts.
Reason: "mutation-capable — excluded from contract runner".

#### EXCLUDED (2 checks)

Skipped by the runner. These are not validation scripts. Reason: "not a validation script".

### JSON Output Schema

When `-Json` is specified, the runner emits a JSON object with this structure:

```json
{
  "runner_id": "WIN-HARNESS-CONTRACT-RUNNER-1",
  "version": "1.0.0",
  "timestamp": "2026-06-30T00:00:00-04:00",
  "repo_root": "G:\\OpenWork\\librarian-runtime-node",
  "mode": "all_safe | named_checks",
  "summary": {
    "total": 22,
    "run": 22,
    "passed": 15,
    "failed": 7,
    "errors": 0,
    "skipped": 0,
    "overall": "PASS | FAIL"
  },
  "checks": [
    {
      "name": "pre-mutation-check",
      "display": "Pre-Mutation Custody Gate",
      "command": "powershell.exe -NoProfile -File \"...\\pre-mutation-check.ps1\"",
      "status": "pass | fail | skip | error",
      "exit_code": 0,
      "duration_ms": 523,
      "category": "safe_readonly",
      "skip_reason": null
    }
  ]
}
```

---

## Acceptance Gates

| Gate | Description | Result |
|------|-------------|--------|
| CR-001 | `scripts/harness/run-contract-checks.ps1` exists | PASS |
| CR-002 | Script parses cleanly under PowerShell 5.1 | PASS |
| CR-003 | `-List` mode exits 0 and shows all 44 checks | PASS |
| CR-004 | `-CheckName` with known safe check exits 0 | PASS |
| CR-005 | `-CheckName` with unknown name exits 1 | PASS |
| CR-006 | `-AllSafe` runs all 22 safe_readonly checks and reports pass/fail correctly | PASS |
| CR-007 | JSON output is deterministic, includes name/command/status/duration/skip_reason | PASS |
| CR-008 | No service start/stop performed | PASS |
| CR-009 | No model workload performed | PASS |
| CR-010 | No runtime/router/model code changed | PASS |
| CR-011 | `pre-mutation-check.ps1` still passes on final sealed tree | PASS |
| CR-012 | `postflight-check.ps1` still parses and can be used against final changed-file allowlist | PASS |
| CR-013 | `new-sprint-receipt.ps1` can generate the sprint receipt | PASS |
| CR-014 | Recommended next sprint documented | PASS |

---

## Boundary Adherence

| Boundary | Status |
|----------|--------|
| Only `scripts/harness/` mutated | `run-contract-checks.ps1` |
| Only sprint/receipt docs mutated | sprint doc, receipt, SESSION-HANDOFF.md |
| No service mutation | Enforced by runner design — all non-safe checks are skipped |
| No runtime/model code change | Zero runtime, router, or model files touched |
| No environment repair | Runner never writes to environment — reads exit codes only |

---

## Usage Notes

### List all available checks

```powershell
.\scripts\harness\run-contract-checks.ps1 -List
```

### Run all safe read-only checks

```powershell
.\scripts\harness\run-contract-checks.ps1 -AllSafe
```

### Run specific checks by name

```powershell
.\scripts\harness\run-contract-checks.ps1 -CheckName pre-mutation-check,runtime-status
```

### Run checks with JSON output (programmatic)

```powershell
$result = .\scripts\harness\run-contract-checks.ps1 -CheckName test-operator-runbook -Json -Quiet
$result | ConvertFrom-Json | ForEach-Object { $_.summary }
```

### Run all safe checks with JSON

```powershell
.\scripts\harness\run-contract-checks.ps1 -AllSafe -Json
```

### Workflow integration with postflight-check

```powershell
# At sprint start
.\scripts\harness\pre-mutation-check.ps1

# During sprint, validate state
.\scripts\harness\run-contract-checks.ps1 -AllSafe

# At sprint end
.\scripts\harness\postflight-check.ps1 -SprintId "WIN-MY-SPRINT" -StartingHead "abc1234"
```

---

## Next-Sprint Suggestion

After this sprint, the recommended next sprint is:

**WIN-HARNESS-BASELINE-DIFF-1** — Baseline drift detection tool. The harness now has
pre-flight checks, post-flight verification, receipt generation, and a unified contract
runner. The next logical step is automated baseline drift detection — comparing current
environment state (service, ports, processes, disk, git) against a frozen baseline
snapshot and reporting deviations.

Alternative: **WIN-SPRINT-LEDGER-1** if sprint ledger convention is preferred over
baseline drift detection.

See `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md` for the full remaining sprint map.
