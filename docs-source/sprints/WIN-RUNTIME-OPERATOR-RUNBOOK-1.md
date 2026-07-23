# WIN-RUNTIME-OPERATOR-RUNBOOK-1 — Windows Runtime Operator Runbook

**Status**: CLOSED — PROMOTE  
**Date**: 2026-06-28  
**Repo**: `librarian-runtime-node`  
**Branch**: `main`

---

## 1. Objective

Create a safe, human-first operator runbook for using the Windows runtime node
without weakening the custody baseline. Documentation and validation checks only.
No services started, no models run, no runtime behavior mutated.

## 2. Starting HEAD

```
310e999 docs(sprint): close WIN-MCP-TEMPLATE-RECONCILE-1 — PROMOTE
```

## 3. Final HEAD

```
ed1940a docs(operations): add Windows runtime operator runbook
```

## 4. Files Changed

| File | Action | Description |
|------|--------|-------------|
| `docs/operations/WIN-RUNTIME-OPERATOR-RUNBOOK.md` | Created | Full operator runbook (11 sections, ~550 lines) |
| `scripts/tests/test-win-runtime-operator-runbook.py` | Created | 74 validation tests covering runbook completeness |
| `scripts/tests/test-mcp-template-reconciliation.py` | Modified | Added `docs/operations/` to allowed file prefixes |

## 5. Runbook Path

```
docs/operations/WIN-RUNTIME-OPERATOR-RUNBOOK.md
```

### Runbook Sections

| # | Section | Content |
|---|---------|---------|
| 1 | Overview | Purpose, authority chain (binary, ports, service) |
| 2 | Prerequisites | Elevation check, working directory |
| 3 | Local Config Setup Checklist | model-profiles, model manager, runtime-node, custody manifest |
| 4 | Port Verification Checklist | Authoritative port map (9130, 9120-9125), pre/post-start checks |
| 5 | Backend Binary Verification Checklist | `llama-server.exe` authority, config validation |
| 6 | MCP Health Check Usage | `scripts/check-mcp-health.ps1`, exit codes, status output, bridge usage |
| 7 | Service Start/Stop Procedure | Human-first step-by-step, helper scripts, do-not-automate |
| 8 | Log and Evidence Capture | Log paths, pre/post-state capture |
| 9 | Failure Triage | Service start/stop failure, orphan cleanup, MCP failure, config parse error |
| 10 | Do Not Proceed Conditions | Hard stops, policy stops, boundary stops |
| 11 | Reference: Authoritative Values | Ports, binary, service ID, MCP endpoints, config files |

## 6. Validation Results

### Operator Runbook Tests (new)

```
Test Results: 74 passed, 0 failed, 74 total
All tests passed!
```

Test categories:
1. Runbook file exists
2. All 10 required sections present
3. Authoritative port map (7 ports)
4. Authorized backend binary (`llama-server.exe`)
5. MCP health check script referenced
6. All 12 required reference strings present
7. "Do not proceed" conditions present (8 patterns)
8. No auto-service or auto-model instructions (8 patterns)
9. No machine-local paths (5 patterns)
10. 7 authoritative config sources referenced
11. Start/stop are human instructions (5 checks)
12. Failure triage covers 5 failure modes

### MCP Template Reconciliation Tests (existing)

```
Test Results: 147 passed, 0 failed, 147 total
All tests passed!
```

### Custody Normalization Tests (existing)

```
Test Results: 55 passed, 0 failed, 55 total
All tests passed!
```

### Combined Test Summary

| Suite | Passed | Failed |
|-------|--------|--------|
| Runbook validation | 74 | 0 |
| MCP template reconciliation | 147 | 0 |
| Custody normalization | 55 | 0 |
| **Total** | **276** | **0** |

## 7. Service State

```
Name                  Status StartType
----                  ------ ---------
LibrarianRunTimeNode Stopped    Manual
```

**Confirmed**: Service remains Stopped / Manual. No service was started or modified.

## 8. Router / Runtime / Models Untouched

- `router/` — No files modified.
- `rust-router/` — No files modified.
- `runtime/` — No files modified.
- `runtime/llama.cpp/` — No files modified.
- `models/` — No files modified.
- **Confirmed**: No production router code, runtime code, or model files were touched.

## 9. Runtime HTTP Untouched

- No HTTP endpoint definitions were changed.
- No port assignments were changed.
- No HTTP semantics were altered.
- **Confirmed**: Runtime HTTP untouched.

## 10. Model Execution Untouched

- No model was loaded, started, or queried.
- No model profile was changed.
- Runbook explicitly forbids auto-model execution.
- **Confirmed**: Model execution untouched.

## 11. Orphan Process Check

```
(No orphan llama-server or rust-router processes found)
```

**Confirmed**: 0 orphan processes.

## 12. Working Tree

```
On branch main
Your branch is ahead of 'origin/main' by 15 commits.
nothing to commit, working tree clean
```

**Confirmed**: Working tree clean after commit.

## 13. Hard Boundaries Verified

| Boundary | Status |
|----------|--------|
| Service not started | ✅ Not started |
| Models not run | ✅ Not run |
| `router/` untouched | ✅ Not modified |
| `rust-router/` untouched | ✅ Not modified |
| `runtime/` untouched | ✅ Not modified |
| `models/` untouched | ✅ Not modified |
| Runtime HTTP unchanged | ✅ Not changed |
| No machine-local values committed | ✅ Placeholders only |
| No operator steps converted to automation | ✅ Human-first instructions |
| Service `Stopped / Manual` preserved | ✅ Verified |
| 0 orphan processes | ✅ Verified |

## 14. Classification

**PROMOTE**

All objectives met:
1. Operator runbook created at `docs/operations/WIN-RUNTIME-OPERATOR-RUNBOOK.md`
   with all required sections: config setup, port verification, binary verification,
   MCP health check, start/stop procedure, log capture, failure triage, do-not-proceed.
2. 74 validation tests cover runbook completeness, correctness, and boundary compliance.
3. 276/276 total tests pass across all three suites (runbook, MCP, custody).
4. Service remains Stopped/Manual. Router, runtime, HTTP, model execution untouched.
5. No machine-local values committed. No auto-service/auto-model instructions.
6. 0 orphan processes.
7. Working tree clean.

## 15. PC-Side Status (After This Sprint)

| Area | Status |
|------|--------|
| Startup custody | ✅ Closed |
| Backend binary authority | ✅ Closed |
| MCP template parity | ✅ Closed |
| Windows native MCP bridge | ✅ Added |
| MCP health check | ✅ Added |
| Operator runbook | ✅ Added |
| Production router/runtime/models | ✅ Untouched |
| Service state | ✅ Stopped / Manual |
| Tests | ✅ 276/276 combined passing |
