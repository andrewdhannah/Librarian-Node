# WIN-HARNESS-POSTFLIGHT-1

**Status:** ACTIVE -- IN PROGRESS
**Previous sprint:** WIN-PACKET-VALIDATION-HOOK-1 (SEALED)
**Repo:** `librarian-runtime-node`
**Platform:** Windows (PowerShell 5.1)
**Date:** 2026-06-29

---

## Sprint Summary

Build the Windows harness post-flight verification counterpart to
`scripts/harness/pre-mutation-check.ps1`. This script verifies state after a mutation
sprint and emits deterministic post-flight evidence/receipt output. It completes the
pre/post-flight custody loop defined in `docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md` SS7.

---

## Scope

### In Scope
- `scripts/harness/postflight-check.ps1` -- Post-flight verification script
- `docs/sprints/WIN-HARNESS-POSTFLIGHT-1.md` -- This sprint doc
- `docs/receipts/WIN-HARNESS-POSTFLIGHT-1-RECEIPT.md` -- Closeout receipt
- `SESSION-HANDOFF.md` -- Update if repo convention requires handoff update

### Out of Scope (Do Not)
- No service start or stop
- No model workload
- No runtime/router/model code change
- No environment repair
- No firewall change
- No auto-start change
- No native app work
- No broad agent autonomy

---

## Starting Baseline

| Check | Value |
|-------|-------|
| Repo | `G:\OpenWork\librarian-runtime-node` |
| Branch | `main` |
| HEAD | `6b1abf2` -- `docs(plan): create WIN-PC-REMAINING-SPRINTS-PLAN-1 remaining sprint map` |
| Working tree | Clean |
| Origin | Up to date |

---

## Durable State Verification (pre-work)

1. HEAD matches `6b1abf2`
2. git status is clean
3. origin/main is in sync (0 ahead, 0 behind)
4. SESSION-HANDOFF.md reads correctly
5. `scripts/harness/pre-mutation-check.ps1` exists and passes with all 11 checks
6. `docs/receipts/WIN-PACKET-VALIDATION-HOOK-1-RECEIPT.md` exists
7. `docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md` exists

---

## Post-Flight Hook Specification

### Required Checks

| # | Check | Description |
|---|-------|-------------|
| 1 | Repo root accessible | `Test-Path` on repo root |
| 2 | Git HEAD | `git rev-parse --short HEAD`; validate against optional `-ExpectedHead` |
| 3 | Working tree state | `git status --short`; report dirty files |
| 4 | Git branch is main | `git rev-parse --abbrev-ref HEAD`; must be `main` |
| 5 | Service `LibrarianRunTimeNode` | `Get-Service`; expect Stopped + Manual |
| 6 | Ports 9120-9125 free | `netstat -ano`; fail if any in LISTENING |
| 7 | Port 9130 free | `netstat -ano`; fail if in LISTENING |
| 8 | Orphan processes | `Get-CimInstance Win32_Process`; check for `llama-server.exe`, `rust-router.exe`, `python.exe` (router) |
| 9 | C: drive free space | `Get-CimInstance Win32_LogicalDisk`; default threshold 5 GB |
| 10 | Origin/main in sync | `git rev-parse HEAD` vs `origin/main` |
| 11 | Changed file summary | `git diff --name-only` from `-StartingHead` to HEAD |
| 12 | Changed file allowlist | Validate all changed files match `-ExpectedChangedFiles` patterns |
| 13 | Required sprint doc exists | Verify `-RequiredSprintDoc` path exists |
| 14 | Required sprint receipt exists | Verify `-RequiredSprintReceipt` path exists |

### Behavior

- Exit code 0 = ALL CHECKS PASSED -- sprint closeout valid
- Exit code 1 = ONE OR MORE CHECKS FAILED -- review before sealing
- Emits structured JSON receipt to console (and optionally to file via `-ReceiptOutputPath`)
- Receipt contains: HEAD, status, service, ports, orphans, disk, file summary
- Allowlist uses glob patterns (`*` matches any path segment)
- Script is read-only: no service start/stop, no process kill, no repair

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `-RepoRoot` | string | auto-detect | Path to repo root |
| `-SprintId` | string | UNKNOWN | Sprint identifier for receipt |
| `-StartingHead` | string | (required) | HEAD before sprint work began |
| `-ExpectedHead` | string | (none) | Expected ending HEAD |
| `-ExpectedChangedFiles` | string[] | empty | Allowlist of expected changed file paths (glob patterns) |
| `-RequiredSprintDoc` | string | (none) | Repo-relative path to sprint doc |
| `-RequiredSprintReceipt` | string | (none) | Repo-relative path to receipt |
| `-ReceiptOutputPath` | string | (none) | Write receipt JSON to this file |
| `-MinCdriveFreeGB` | double | 5.0 | Minimum C: free space in GB |
| `-Quiet` | switch | false | Suppress informational output |

### Receipt JSON Structure

```json
{
  "meta":       { "sprint_id", "starting_head", "repo_root", "timestamp" },
  "state":      { "current_head", "branch", "service_status", "ports_*",
                  "orphan_count", "c_drive_free_gb", "dirty_count", ... },
  "files":      { "changed_count", "changed_files", "unexpected_changes",
                  "sprint_doc", "sprint_receipt", ... },
  "checks":     { },
  "summary":    { "total", "passed", "failed" },
  "overall":    "PASS" | "FAIL"
}
```

---

## Acceptance Gates

| Gate | Description |
|------|-------------|
| PF-001 | `scripts/harness/postflight-check.ps1` exists |
| PF-002 | Script parses cleanly under PowerShell 5.1 |
| PF-003 | PASS case (clean tree, matching HEAD, valid allowlist) exits 0 |
| PF-004 | Wrong expected HEAD exits 1 |
| PF-005 | Dirty/unexpected state exits 1 |
| PF-006 | Missing required sprint doc/receipt exits 1 |
| PF-007 | Allowlist mismatch exits 1 |
| PF-008 | Receipt output is deterministic JSON with required fields |
| PF-009 | No service start/stop performed |
| PF-010 | No model workload performed |
| PF-011 | No runtime/router/model code changed |
| PF-012 | `pre-mutation-check.ps1` still passes on final sealed tree |
| PF-013 | Recommended next sprint documented |

---

## Boundary Adherence

| Boundary | Status |
|----------|--------|
| Only `scripts/harness/` mutated | `postflight-check.ps1` |
| Only sprint/receipt docs mutated | sprint doc, receipt, SESSION-HANDOFF.md |
| No service mutation | Enforced by script design |
| No runtime/model code change | Zero runtime, router, or model files touched |
| No environment repair | Script is read-only by design |

---

## Usage Notes

### Basic sprint closeout

```powershell
.\scripts\harness\postflight-check.ps1 -SprintId "WIN-MY-SPRINT" -StartingHead "abcdef1"
```

### Full closeout with validation

```powershell
.\scripts\harness\postflight-check.ps1 `
  -SprintId "WIN-MY-SPRINT" `
  -StartingHead "abcdef1" `
  -ExpectedHead "abcdef2" `
  -ExpectedChangedFiles @("scripts/harness/*", "docs/*.md") `
  -RequiredSprintDoc "docs/sprints/WIN-MY-SPRINT.md" `
  -RequiredSprintReceipt "docs/receipts/WIN-MY-SPRINT-RECEIPT.md" `
  -ReceiptOutputPath "G:\temp\sprint-receipt.json"
```

### Pre-flight / Post-flight workflow

```powershell
# Step 1: Before work
.\scripts\harness\pre-mutation-check.ps1 -ExpectedHead "abcdef1"

# Step 2: Do sprint work (mutate files)

# Step 3: Commit and get ending HEAD
git add <files>
git commit -m "feat(...): ..."
$endingHead = git rev-parse --short HEAD

# Step 4: After work
.\scripts\harness\postflight-check.ps1 `
  -SprintId "WIN-MY-SPRINT" `
  -StartingHead "abcdef1" `
  -ExpectedHead $endingHead
```

---

## Next-Sprint Suggestion

After this sprint, the recommended next sprint is:

**WIN-HARNESS-RECEIPT-TEMPLATE-1** -- Standardized sprint receipt generation tool.
With pre-flight and post-flight hooks complete, the next step is to automate receipt
generation so that every sprint produces a consistent, machine-readable closeout record.

Alternative: **WIN-HARNESS-CONTRACT-RUNNER-1** if unified contract test runner is
preferred over receipt automation.

See `docs/planning/WIN-SPRINT-SEQUENCE.md` SS4 Track A for the full harness implementation
sequence.
