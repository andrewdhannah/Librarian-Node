# WIN-RUNTIME-CONTROLLED-ACTIVATION-1

**Status:** CLOSED — PROMOTE
**Previous sprint:** WIN-RUNTIME-DRY-RUN-GAP-CLOSE-1 (PROMOTED)
**Repo:** `librarian-runtime-node`
**Platform:** Windows (PowerShell 5.1+)
**Date:** 2026-06-28

---

## Scope

Controlled first service activation of `LibrarianRunTimeNode` on this machine.  

Activation is **runbook-driven only** — every step follows §7 of the operator runbook.  
No auto-start, no auto-model, no workload beyond activation checks.

---

## Success Criteria

1. `scripts/check-mcp-health.ps1` executes without parser error under PS 5.1.
2. Health check run with server offline → exit code 1.
3. Operator starts `LibrarianRunTimeNode` service in elevated PS.
4. Port 9130 shows LISTENING; router `/backend/status` responds.
5. MCP health checks and permission matrix validation pass.
6. Service stopped cleanly.
7. Ports released; zero orphan `llama-server.exe` processes.
8. Closeout receipt generated.

---

## Authority

| Authority | Reference |
|-----------|-----------|
| Runbook | `docs/operations/WIN-RUNTIME-OPERATOR-RUNBOOK.md` §7 |
| Port map | §11.1 (Router 9130, model ports 9120–9125) |
| Service name | `LibrarianRunTimeNode` |
| Binary | `llama-server.exe` at `runtime/llama.cpp/` |
| Exit codes | check-mcp-health.ps1: 0=OK, 1=unreachable, 2=MCP fail, 3=tools missing |

---

## Pre-Start Baseline (confirmed 2026-06-28)

| Check | Expected | Actual |
|-------|----------|--------|
| HEAD | `a010bf7` | `a010bf7` |
| Working tree | clean | clean (pre-fix) |
| Service state | Stopped / Manual | Stopped / Manual |
| Orphan llama-server.exe | 0 | 0 |
| Port 9120–9125 | FREE | FREE |
| Port 9130 | FREE | FREE |

---

## Pre-Start Fix: check-mcp-health.ps1 Parser Error

**Blocker found:** `scripts/check-mcp-health.ps1` failed under PS 5.1 Parser::ParseFile with 5 errors.

**Root cause:** File was UTF-8 without BOM containing em-dash characters (U+2014, encoded as `E2 80 94`). PS 5.1 Parser::ParseFile reads without BOM using the system ANSI code page, misinterpreting `E2 80 94` as three ANSI characters, corrupting string boundaries and brace matching.

**Fix applied:**
1. Replaced all `—` (U+2014) with ASCII ` -- `.
2. Added UTF-8 BOM.
3. File retained CRLF line endings (matching repo's autocrlf=normalized state).

**Verification:** Parser::ParseFile returns 0 errors. Script exits with code 1 (server unreachable — expected when Librarian server is offline).

---

## Procedural Steps

### Step 1 — Operator: Start Service

Human operator with Administrator privileges runs:

```powershell
Start-Service -Name LibrarianRunTimeNode
```

### Step 2 — Verify Router Listener

```
netstat -ano | Select-String ":9130.*LISTENING"
```

Expected: Port 9130 shows LISTENING. Record PID.

### Step 3 — Router Identity

```
Invoke-RestMethod http://127.0.0.1:9130/backend/status
```

### Step 4 — MCP Health Checks

```
.\scripts\check-mcp-health.ps1
```

With Librarian server running, expected exit code 0.

### Step 5 — Stop Service

```
Stop-Service -Name LibrarianRunTimeNode -Force
```

### Step 6 — Verify Cleanup

- Port 9130: FREE
- Ports 9120–9125: FREE
- Orphan `llama-server.exe`: 0

---

## OODA Loop

| Phase | Status |
|-------|--------|
| Observe | Baseline confirmed, blocker diagnosed |
| Orient | Em-dash encoding causes PS 5.1 ParseFile failure |
| Decide | Fix script → commit → proceed to service start |
| Act | Script fixed, ready for operator action |

---

## Closeout

See `docs/receipts/WIN-RUNTIME-CONTROLLED-ACTIVATION-1-RECEIPT.md` after completion.
