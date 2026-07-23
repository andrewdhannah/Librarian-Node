# Closeout Receipt: WIN-RUNTIME-CONTROLLED-ACTIVATION-1

**Status:** CLOSED — PROMOTE
**Date:** 2026-06-28
**Previous sprint:** WIN-RUNTIME-DRY-RUN-GAP-CLOSE-1 (PROMOTED)

---

## Summary

Controlled first service activation of `LibrarianRunTimeNode` on this machine.
Runbook-driven activation only (§7). No model workload beyond activation checks.

**Result: PASS** — all success criteria met.

---

## Pre-Start Baseline

| Check | Expected | Actual | Pass |
|-------|----------|--------|------|
| HEAD | `a010bf7` | `a010bf7` | ✅ |
| Working tree | clean | clean | ✅ |
| Service state | Stopped / Manual | Stopped / Manual | ✅ |
| Orphan llama-server.exe | 0 | 0 | ✅ |
| Port 9130 | FREE | FREE | ✅ |
| Ports 9120–9125 | FREE | FREE | ✅ |

---

## Blocker: check-mcp-health.ps1 Parser Fix

| Field | Detail |
|-------|--------|
| **Blocker** | ParseFile failed: 5 errors starting at line 74 |
| **Root cause** | UTF-8 no-BOM + em dashes (U+2014) → ANSI misinterpretation |
| **Fix** | Em dashes → ` -- `; add UTF-8 BOM |
| **Verification** | ParseFile: 0 errors; exit code 1 (server offline) |
| **Commit** | `9e7fb04` |

---

## Activation Sequence

### Step 1 — Service Start (Operator)

| Check | Result |
|-------|--------|
| Start-Service LibrarianRunTimeNode | ✅ Operator confirmed |
| Service state | **Running** |
| Startup type | Manual (unchanged) |

### Step 2 — Port 9130 Verification

| Check | Result |
|-------|--------|
| LISTENING on 9130 | ✅ PID 1932 |
| Protocol | TCP |
| Address | 127.0.0.1:9130 |

### Step 3 — Router Identity

```json
{
    "status": "degraded",
    "authority": "advisory_only",
    "active_profile": null,
    "profiles_registered": 5,
    "runtimes_alive": 0,
    "uptime_seconds": 32
}
```

Degraded status is expected — no model selected per activation scope.

### Step 4 — MCP Health Check

| Check | Result | Expected |
|-------|--------|----------|
| Exit code | **1** | 1 (Librarian Swift server offline) |
| Permission matrix | **valid** | valid |
| Status file | `SessionStartup/MCP-STATUS.md` | written |

Permission matrix validation passed:
- `agents_can_mark_verified`: false ✅
- `human_verification_is_final`: true ✅
- No tool has `can_verify: true` ✅
- All 12 expected tools have permission entries ✅

### Step 5 — Service Stop (Operator)

| Check | Result |
|-------|--------|
| Stop-Service | ✅ Operator confirmed |
| Service state | **Stopped** |
| Startup type | Manual (unchanged) |

### Step 6 — Cleanup Verification

| Check | Result | Pass |
|-------|--------|------|
| Port 9130 | **FREE** | ✅ |
| Port 9120 | FREE | ✅ |
| Port 9121 | FREE | ✅ |
| Port 9122 | FREE | ✅ |
| Port 9123 | FREE | ✅ |
| Port 9124 | FREE | ✅ |
| Port 9125 | FREE | ✅ |
| Orphan llama-server.exe | **0** | ✅ |
| Orphan rust-router.exe | **0** | ✅ |

---

## Files Changed

| File | Change | Scope |
|------|--------|-------|
| `scripts/check-mcp-health.ps1` | Fix encoding (em dash → ASCII, +BOM) | Activation blocker |
| `docs/sprints/WIN-RUNTIME-CONTROLLED-ACTIVATION-1.md` | Create (ACTIVE → now CLOSED) | Sprint doc |
| `docs/receipts/WIN-RUNTIME-CONTROLLED-ACTIVATION-1-RECEIPT.md` | Create (this file) | Closeout receipt |

---

## OODA Close

| Phase | Summary |
|-------|---------|
| Observe | Clean baseline; check-mcp-health.ps1 had PS 5.1 parser errors |
| Orient | Em dashes in UTF-8 no-BOM file corrupted under ANSI read path |
| Decide | Fix script → cycle service → validate → close |
| Act | All steps completed, all checks pass |

---

## Promoted Assets

- `scripts/check-mcp-health.ps1` — encoding-hardened for PS 5.1
- `SessionStartup/MCP-STATUS.md` — first valid status file
- Port map: Router 9130 fully verified

---

## Not in Scope

The following were NOT part of this sprint and remain unstarted:
- Model activation / `POST /backend/select`
- Librarian Swift MCP server at `:3456`
- Model profile tuning or binary swap
- Auto-start configuration

---

**Receipt generated:** 2026-06-29T04:02:00Z
**Closing HEAD:** 9e7fb04
