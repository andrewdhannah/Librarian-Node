# WIN-HARNESS-RECEIPT-TEMPLATE-1

**Status:** ACTIVE -- IN PROGRESS
**Previous sprint:** WIN-HARNESS-POSTFLIGHT-1 (SEALED)
**Repo:** `librarian-runtime-node`
**Platform:** Windows (PowerShell 5.1)
**Date:** 2026-06-29

---

## Sprint Summary

Build a standardized Windows harness receipt generation tool under `scripts/harness/`.
The tool generates deterministic sprint receipt Markdown from explicit inputs and/or
`postflight-check.ps1` JSON output. With pre-mutation and post-flight checks now sealed,
this sprint standardizes closeout receipt generation so future Windows harness/runtime
sprints produce consistent evidence without manual markdown writing.

---

## Scope

### In Scope
- `scripts/harness/new-sprint-receipt.ps1` -- Standardized receipt generator
- `docs/sprints/WIN-HARNESS-RECEIPT-TEMPLATE-1.md` -- This sprint doc
- `docs/receipts/WIN-HARNESS-RECEIPT-TEMPLATE-1-RECEIPT.md` -- Closeout receipt
- `SESSION-HANDOFF.md` -- Update if repo convention requires handoff update

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
| HEAD | `4f84852` -- `WIN-HARNESS-POSTFLIGHT-1 post-flight verification` |
| Working tree | Clean |
| Origin | Up to date |

---

## Durable State Verification (pre-work)

1. HEAD matches `4f84852`
2. git status is clean
3. origin/main is in sync (0 ahead, 0 behind)
4. SESSION-HANDOFF.md reads correctly
5. `scripts/harness/pre-mutation-check.ps1` exists and passes (11/11)
6. `scripts/harness/postflight-check.ps1` exists and parses cleanly
7. `docs/receipts/WIN-HARNESS-POSTFLIGHT-1-RECEIPT.md` exists
8. `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md` exists

---

## Receipt Generator Specification

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `-SprintId` | string | Yes | Sprint identifier |
| `-Status` | string | Yes | PASS, FAIL, or PARTIAL |
| `-StartingHead` | string | Yes | HEAD before sprint began |
| `-EndingHead` | string | Yes | HEAD after sprint sealed |
| `-PreviousSprint` | string | Yes | Previous sprint ID |
| `-OutputPath` | string | Yes | Output path for receipt (typically `docs/receipts/<ID>-RECEIPT.md`) |
| `-NextSprint` | string | Yes | Recommended next sprint |
| `-NextSprintRationale` | string | Yes | Rationale for next sprint |
| `-Summary` | string | No | Free-text summary; auto-generated if omitted |
| `-ChangedFiles` | string[] | No | Explicit file list; auto-detected from `git diff` if omitted |
| `-DeliverableScripts` | hashtable[] | No | Script deliverables with File/Description/Size |
| `-DeliverableDocs` | string[] | No | Doc deliverables (file paths) |
| `-AcceptanceGates` | hashtable[] | No | Gate results with Gate/Description/Result |
| `-BoundaryCompliance` | hashtable[] | No | Boundary checks with Boundary/Status |
| `-Findings` | string[] | No | Sprint findings entries |
| `-PostflightJsonPath` | string | No | Path to postflight JSON output to auto-fill state section |
| `-RepoRoot` | string | No | Auto-detected from script location |
| `-Force` | switch | No | Overwrite existing output path |
| `-Quiet` | switch | No | Suppress informational output |

### Receipt Sections

The generated Markdown contains these sections:

1. **Header** -- Sprint ID, status, date, previous sprint
2. **Summary** -- Free-text summary + result
3. **Pre-Work Baseline** -- Starting/ending HEAD, commits, changed files count
4. **Deliverables** -- Scripts created (table with File/Description/Size) and docs created
5. **Changed Files** -- List of all changed files
6. **Acceptance Gates** -- Gate ID, description, result
7. **Boundary Compliance** -- Boundary rules and their status
8. **Findings** (optional) -- Any sprint findings
9. **Closeout State** -- Service, ports, orphans, disk (from postflight JSON if available)
10. **Recommended Next Sprint** -- Next sprint ID and rationale
11. **Footer** -- Generation date, HEADs, file count

### Postflight JSON Integration

When `-PostflightJsonPath` points to a JSON file produced by `postflight-check.ps1 -ReceiptOutputPath`,
the script populates the Closeout State section with live data:
- Service status and start type
- Port 9130 state
- Ports 9120-9125 state
- Orphan process count
- C: drive free space

This enables fully automated closeout:

```powershell
# Step 1: Run postflight with JSON output
.\scripts\harness\postflight-check.ps1 `
  -SprintId "WIN-MY-SPRINT" `
  -StartingHead "abc1234" `
  -ReceiptOutputPath "G:\temp\postflight.json"

# Step 2: Generate receipt from postflight data
.\scripts\harness\new-sprint-receipt.ps1 `
  -SprintId "WIN-MY-SPRINT" `
  -Status "PASS" `
  -StartingHead "abc1234" `
  -EndingHead "def5678" `
  -PreviousSprint "WIN-PRIOR" `
  -OutputPath "docs/receipts/WIN-MY-SPRINT-RECEIPT.md" `
  -PostflightJsonPath "G:\temp\postflight.json" `
  -NextSprint "WIN-NEXT" `
  -NextSprintRationale "Continue the sequence."
```

---

## Acceptance Gates

| Gate | Description |
|------|-------------|
| RT-001 | `scripts/harness/new-sprint-receipt.ps1` exists |
| RT-002 | Script parses cleanly under PowerShell 5.1 |
| RT-003 | Generates receipt to a temp path with exit 0 |
| RT-004 | Rejects missing required fields with exit 1 |
| RT-005 | Can ingest postflight JSON when provided (via `-ReceiptOutputPath` path) |
| RT-006 | Output is deterministic across repeated runs with same inputs |
| RT-007 | No service start/stop performed |
| RT-008 | No model workload performed |
| RT-009 | No runtime/router/model code changed |
| RT-010 | `pre-mutation-check.ps1` still passes on final sealed tree |
| RT-011 | `postflight-check.ps1` still parses and can be used against final changed-file allowlist |
| RT-012 | Recommended next sprint documented |

---

## Boundary Adherence

| Boundary | Status |
|----------|--------|
| Only `scripts/harness/` mutated | `new-sprint-receipt.ps1` |
| Only sprint/receipt docs mutated | sprint doc, receipt, SESSION-HANDOFF.md |
| No service mutation | Enforced by design |
| No runtime/model code change | Zero runtime, router, or model files touched |
| No environment repair | Script is read-only content generator |

---

## Usage Notes

### Minimal receipt (most fields auto-populated)

```powershell
.\scripts\harness\new-sprint-receipt.ps1 `
  -SprintId "WIN-MY-SPRINT" `
  -Status "PASS" `
  -StartingHead "abc1234" `
  -EndingHead "def5678" `
  -PreviousSprint "WIN-PRIOR" `
  -OutputPath "docs/receipts/WIN-MY-SPRINT-RECEIPT.md" `
  -NextSprint "WIN-NEXT" `
  -NextSprintRationale "Rationale for next sprint."
```

Changed files are auto-detected from `git diff abc1234..def5678`.

### Full receipt with all sections

```powershell
.\scripts\harness\new-sprint-receipt.ps1 `
  -SprintId "WIN-MY-SPRINT" `
  -Status "PASS" `
  -StartingHead "abc1234" `
  -EndingHead "def5678" `
  -PreviousSprint "WIN-PRIOR" `
  -OutputPath "docs/receipts/WIN-MY-SPRINT-RECEIPT.md" `
  -Summary "Implemented feature X." `
  -DeliverableScripts @(@{File="scripts/my-script.ps1";Description="Does X";Size="~5 KB"}) `
  -DeliverableDocs @("docs/sprints/WIN-MY-SPRINT.md") `
  -AcceptanceGates @(@{Gate="G-01";Description="Gate one";Result="PASS"}) `
  -BoundaryCompliance @(@{Boundary="No service start";Status="PASS"}) `
  -Findings @("F-001: Minor observation") `
  -NextSprint "WIN-NEXT" `
  -NextSprintRationale "Rationale here."
```

### Postflight-integrated closeout

```powershell
# Postflight writes JSON
.\scripts\harness\postflight-check.ps1 -SprintId "WIN-MY-SPRINT" `
  -StartingHead "abc1234" -ReceiptOutputPath "G:\temp\pf.json"

# Receipt generator reads JSON for state section
.\scripts\harness\new-sprint-receipt.ps1 -SprintId "WIN-MY-SPRINT" `
  -Status "PASS" -StartingHead "abc1234" -EndingHead "def5678" `
  -PreviousSprint "WIN-PRIOR" `
  -OutputPath "docs/receipts/WIN-MY-SPRINT-RECEIPT.md" `
  -PostflightJsonPath "G:\temp\pf.json" `
  -NextSprint "WIN-NEXT" -NextSprintRationale "Continue."
```

---

## Next-Sprint Suggestion

After this sprint, the recommended next sprint is:

**WIN-HARNESS-CONTRACT-RUNNER-1** -- Unified contract test runner wrapping existing test
scripts. With pre-flight, post-flight, and receipt automation complete, the harness now
needs a unified way to run and report on router/runtime contract tests.

Alternative: **WIN-SPRINT-LEDGER-1** if sprint ledger is preferred over contract runner.

See `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md` for the full remaining sprint map.
