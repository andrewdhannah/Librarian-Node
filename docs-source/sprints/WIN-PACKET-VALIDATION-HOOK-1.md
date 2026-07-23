# WIN-PACKET-VALIDATION-HOOK-1

**Status:** ACTIVE -- IN PROGRESS
**Previous sprint:** WIN-AGENT-HARNESS-PLAN-1 (SEALED)
**Repo:** `librarian-runtime-node`
**Platform:** Windows (PowerShell 5.1)
**Date:** 2026-06-29

---

## Sprint Summary

Implement the first Windows harness pre-mutation custody gate under `scripts/harness/`.
This is a pre-mutation safety hook -- a read-only environment verifier that gates on
preconditions before any agent or human mutation of the workspace. It is NOT a runtime
health probe.

---

## Scope

### In Scope
- `scripts/harness/pre-mutation-check.ps1` -- Pre-mutation verification script
- `docs/sprints/WIN-PACKET-VALIDATION-HOOK-1.md` -- This sprint doc
- `docs/receipts/WIN-PACKET-VALIDATION-HOOK-1-RECEIPT.md` -- Closeout receipt
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
- Do not modify planning docs from WIN-AGENT-HARNESS-PLAN-1
- Do not modify existing scripts outside `scripts/harness/`

---

## Starting Baseline

| Check | Value |
|-------|-------|
| Repo | `G:\OpenWork\librarian-runtime-node` |
| Branch | `main` |
| HEAD | `7cc7d10` -- `WIN-AGENT-HARNESS-PLAN-1 planning docs` |
| Working tree | Clean |
| Origin | Up to date |

---

## Durable State Verification (pre-work)

1. HEAD matches `7cc7d10`
2. git status is clean
3. origin/main is in sync (0 ahead, 0 behind)
4. SESSION-HANDOFF.md reads correctly
5. All 5 planning docs exist:
   - `docs/planning/WIN-AGENT-HARNESS-PLAN.md`
   - `docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md`
   - `docs/planning/WIN-HARNESS-PARITY-ROADMAP.md`
   - `docs/planning/WIN-LIBRARIAN-HOST-OPTIONS.md`
   - `docs/planning/WIN-SPRINT-SEQUENCE.md`
6. `docs/receipts/WIN-AGENT-HARNESS-PLAN-1-RECEIPT.md` exists

---

## Pre-Mutation Hook Specification

### Required Checks

The hook shall verify the following dimensions and emit deterministic pass/fail output:

| # | Check | Implementation |
|---|-------|---------------|
| 1 | Repo root accessible | `Test-Path` on repo root |
| 2 | Git HEAD | `git rev-parse --short HEAD`; optionally validate against `-ExpectedHead` |
| 3 | Working tree clean/dirty | `git status --short`; fail if any dirty file found |
| 4 | Git branch is main | `git rev-parse --abbrev-ref HEAD`; must be `main` |
| 5 | `LibrarianRunTimeNode` service state | `Get-Service`; expect Stopped + Manual |
| 6 | Ports 9120-9125 free | `netstat -ano`; fail if any in LISTENING |
| 7 | Port 9130 free | `netstat -ano`; fail if in LISTENING |
| 8 | Orphan runtime/router/model processes | `Get-CimInstance Win32_Process`; check for `llama-server.exe`, `rust-router.exe`, `python.exe` (router) |
| 9 | C: drive free space >= threshold | `Get-CimInstance Win32_LogicalDisk`; default threshold 5 GB |
| 10 | Origin/main in sync | `git rev-parse HEAD` vs `origin/main` |
| 11 | Required planning/baseline/receipt files | File existence check for standard docs |

### Behavior

- Exit code 0 = ALL CHECKS PASSED -- safe to proceed
- Exit code 1 = ONE OR MORE CHECKS FAILED -- do not mutate
- Output is deterministic: same machine state produces same result
- Script is read-only: no service start/stop, no process kill, no repair

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `-RepoRoot` | string | auto-detect | Path to repo root |
| `-ExpectedHead` | string | (none) | Expected short SHA at HEAD |
| `-MinCdriveFreeGB` | double | 5.0 | Minimum C: free space in GB |
| `-RequiredFiles` | string[] | standard set | Repo-relative paths to check |
| `-Quiet` | switch | false | Suppress informational output |

---

## Acceptance Gates

| Gate | Description |
|------|-------------|
| VH-001 | `scripts/harness/pre-mutation-check.ps1` exists |
| VH-002 | Script executes without parse errors |
| VH-003 | Script returns PASS on clean repo with correct HEAD |
| VH-004 | Script returns FAIL when working tree is dirty (detects known untracked) |
| VH-005 | Script returns FAIL when HEAD does not match expected |
| VH-006 | All 11 required checks are implemented |
| VH-007 | Script exits 0 on all-pass, 1 on any-fail |
| VH-008 | No service start/stop performed |
| VH-009 | No model workload performed |
| VH-010 | No runtime/router/model code changed |
| VH-011 | Receipt/evidence file emitted |
| VH-012 | Recommended next sprint documented |

---

## Boundary Adherence

| Boundary | Status |
|----------|--------|
| Only `scripts/harness/` mutated | scripts/harness/pre-mutation-check.ps1 |
| Only sprint/receipt docs mutated | docs/sprints/WIN-PACKET-VALIDATION-HOOK-1.md, docs/receipts/WIN-PACKET-VALIDATION-HOOK-1-RECEIPT.md |
| No service mutation | Enforced by pre-flight check itself |
| No runtime/model code change | Zero runtime, router, or model files touched |
| No environment repair | Script is read-only by design |
| No broad agent autonomy | Script is a gate, not an action executor |

---

## Next-Sprint Suggestion

After this sprint, the recommended next sprint is:

**WIN-HARNESS-POSTFLIGHT-1** -- Build post-flight state verification and receipt generation
for the harness. This completes the pre/post-flight loop defined in the custody sandbox model.

Alternative: **WIN-HARNESS-CONTRACT-RUNNER-1** if post-flight is deferred.

See `docs/planning/WIN-SPRINT-SEQUENCE.md` SS4 Track A for the full sequence.
