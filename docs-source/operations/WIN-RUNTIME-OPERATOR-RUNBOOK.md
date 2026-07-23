# WIN-RUNTIME-OPERATOR-RUNBOOK

**Purpose:** Safe human/operator procedure for using the Windows Librarian runtime node.
**Status:** Operator reference — do not use as automated startup script.
**Repo:** `librarian-runtime-node`
**Platform:** Windows (PowerShell 5.1+)
**Last reviewed:** 2026-06-28

---

## Table of Contents

1. [Overview](#1-overview)
2. [Prerequisites](#2-prerequisites)
3. [Local Config Setup Checklist](#3-local-config-setup-checklist)
4. [Port Verification Checklist](#4-port-verification-checklist)
5. [Backend Binary Verification Checklist](#5-backend-binary-verification-checklist)
6. [MCP Health Check Usage](#6-mcp-health-check-usage)
7. [Service Start/Stop Procedure](#7-service-startstop-procedure)
8. [Log and Evidence Capture](#8-log-and-evidence-capture)
9. [Failure Triage](#9-failure-triage)
10. [Do Not Proceed Conditions](#10-do-not-proceed-conditions)
11. [Reference: Authoritative Values](#11-reference-authoritative-values)

---

## 1. Overview

The Windows runtime node runs the Librarian backend services on a Windows PC.
It hosts:

- **Router** (Rust or Python) — manages model selection and request dispatch.
- **Model backends** (`llama-server.exe`) — one per model profile.
- **MCP bridge** — optional stdio bridge for OpenWork MCP connectivity.

This runbook is a **human-first reference**. Operators follow these steps
manually. Scripts exist in `scripts/operations/` as helpers, but the operator
must verify each step — no automation replaces human judgment.

### Authority Chain

| Authority | Source |
|-----------|--------|
| Backend binary | `llama-server.exe` |
| Binary location | `runtime\llama.cpp\llama-server.exe` (in-repo default) |
| Router port | `9130` (or `$env:ROUTER_PORT`) |
| Model ports | See [section 11](#11-reference-authoritative-values) |
| Service name | `LibrarianRunTimeNode` |
| Service startup | **Manual** — never change to Automatic without Owner approval |

---

## 2. Prerequisites

### Elevation

Some operations require **Administrator** privileges. Check before starting:

```powershell
([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator
)
```

| Operation | Elevation Required |
|-----------|-------------------|
| Start/stop service | Yes |
| Port checks | No |
| Binary verification | No |
| Config file edits | No |
| Log reading | No |
| MCP health check | No |

If the check returns `False`, do not attempt service start or stop.

### Working Directory

All paths in this runbook are relative to the repo root:

```powershell
Set-Location G:\OpenWork\librarian-runtime-node
```

Verify you are in the correct repo:

```powershell
git rev-parse --show-toplevel
# Expected: G:\OpenWork\librarian-runtime-node
```

---

## 3. Local Config Setup Checklist

The repo ships with example config files. Before operating, copy each to its
local override and edit for your machine.

### 3.1 Model Profiles

```powershell
Copy-Item config\model-profiles.local.example.json config\model-profiles.local.json
```

Edit `config\model-profiles.local.json`:
- Set `defaults.binary` to the **absolute path** of `llama-server.exe`.
- Set `defaults.gguf_root` to your model directory.
- For each profile, verify `model_path` and `launch_command` use your local paths.
- Do **not** commit this file (gitignored by `config/*.local.json`).

Expected binary authority: **`llama-server.exe`**.
Do not use `llama-server-mini.exe` or any other binary without explicit Owner approval.

### 3.2 Model Manager Overrides (Optional)

```powershell
Copy-Item config\model_manager.local.example.ps1 config\model_manager.local.ps1
```

Edit `config\model_manager.local.ps1`:
- Uncomment and set `$ServerPath` if your backend binary is not the in-repo default.
- Uncomment and set `$ModelsDir` if your models are not at the default location.
- Do **not** commit this file (gitignored by `config/*.local.*`).

### 3.3 Runtime Node Config (Optional)

```powershell
Copy-Item config\runtime-node.example.json config\runtime-node.local.json
```

Edit `config\runtime-node.local.json`:
- Set `llama.binary_path` and `llama.model_path` for your local environment.
- Do **not** commit this file (gitignored).

### 3.4 Startup Custody Manifest (Optional Reference)

The example custody manifest at:
`fixtures/startup-files-custody\startup-custody-manifest.example.json`

documents the expected service configuration, ports, binaries, and paths. It is
a **reference only** — do not edit it for machine-local values.

### 3.5 Verification: Config Loads Without Error

After editing, verify the config files parse correctly:

```powershell
# Verify model-profiles.local.json is valid JSON
Get-Content config\model-profiles.local.json | ConvertFrom-Json
```

Expected: JSON parsed without error. If any config fails to parse, fix before
proceeding.

---

## 4. Port Verification Checklist

### 4.1 Authoritative Port Map

| Profile | Port | Purpose |
|---------|------|---------|
| Router | 9130 | Request dispatch |
| phi-4 | 9120 | General advisory model |
| qwen-coder | 9121 | Code advisory model |
| llama-3.2 | 9122 | General advisory model |
| qwen3 | 9123 | General advisory + reasoning |
| gemma-3 | 9124 | General advisory model |
| Embedding | 9125 | Embedding server |

**Source of truth:** `config/model-profiles.json` profile entries.

### 4.2 Pre-Start Check: No Unexpected Listeners

Before starting any service, verify no stale listeners occupy these ports:

```powershell
netstat -ano | Select-String ":9130"
netstat -ano | Select-String ":912[0-5]"
```

**Rules:**
- `LISTENING` — a process owns this port. Record its PID.
- `TIME_WAIT` — normal TCP cleanup, not a concern.
- If an unexpected `LISTENING` entry exists for a port you do not own, stop and
  investigate before starting the service.
- If the port belongs to a known orphan process, see [Failure Triage](#9-failure-triage).

### 4.3 Post-Start Verification

After starting the service (see [section 7](#7-service-startstop-procedure)):

```powershell
netstat -ano | Select-String ":9130.*LISTENING"
```

Expected: Port 9130 shows `LISTENING`. Record the PID.

---

## 5. Backend Binary Verification Checklist

### 5.1 Authoritative Binary

The authoritative backend binary is **`llama-server.exe`**.

Default location (in-repo):
```
runtime\llama.cpp\llama-server.exe
```

The binary must exist and be executable:

```powershell
# Check default binary exists
Test-Path runtime\llama.cpp\llama-server.exe

# Check version (may fail on older builds)
& .\runtime\llama.cpp\llama-server.exe --version 2>$null
```

### 5.2 Verify Against model-profiles.json

```powershell
python -c "
import json
d = json.load(open('config/model-profiles.json'))
print('Default binary:', d['defaults']['binary'])
for p in d['profiles']:
    cmd = p.get('launch_command', '')
    if cmd:
        print(f'{p[\"alias\"]}: {cmd[:80]}...')
"
```

**Rules:**
- The default binary in `model-profiles.json` must reference `llama-server.exe`.
- Each profile's `launch_command` must start with `llama-server.exe` (not
  `main.exe`, not `llama-server-mini.exe`).
- If you have a local override in `model-profiles.local.json`, verify the same.

### 5.3 Verify Machine-Local Example Exists

```powershell
Test-Path config\model-profiles.local.example.json
Test-Path config\model_manager.local.example.ps1
Test-Path config\runtime-node.example.json
```

All should return `True`.

---

## 6. MCP Health Check Usage

### 6.1 Prerequisites

- The Librarian server (Swift) must be running and serving the MCP endpoint at
  `http://127.0.0.1:3456/mcp`.
- The MCP bridge script at `scripts/mcp-bridge.ps1` provides a stdio bridge for
  OpenWork MCP connectivity.

### 6.2 Running the Health Check

```powershell
.\scripts\check-mcp-health.ps1
```

**What it checks:**

| Check | Description |
|-------|-------------|
| Server health | `GET /api/health` — server must respond with `status: ok` |
| MCP endpoint | `POST /mcp` with `tools/list` |
| JSON-RPC initialize | `initialize` method must return a result |
| Tool inventory | All expected Librarian MCP tools must be present |
| Permission matrix | `config/mcp-permissions.json` must be valid |

### 6.3 Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | Server unreachable — start Librarian server first |
| 2 | MCP endpoint unreachable — check Librarian server config |
| 3 | Tools missing — runbook may proceed but MCP is incomplete |

### 6.4 Status Output

The health check writes a status file to `SessionStartup/MCP-STATUS.md`
containing the full report.

### 6.5 MCP Bridge Usage (if needed)

The stdio bridge script at `scripts/mcp-bridge.ps1` reads JSON-RPC lines from
stdin and forwards them to the Librarian MCP endpoint. It is used by OpenWork
when configuring an MCP server of type `local`.

```powershell
# Test the bridge (standalone):
echo '{"jsonrpc":"2.0","id":"1","method":"tools/list"}' | .\scripts\mcp-bridge.ps1
```

Do **not** modify `scripts/mcp-bridge.ps1` or `scripts/check-mcp-health.ps1`
for machine-local values.

---

## 7. Service Start/Stop Procedure

**Do not automate this sequence.** Each step requires human verification.

**Helper scripts available:**
- `scripts/operations/runtime-start.ps1` — assists with steps 3–5.
- `scripts/operations/runtime-stop.ps1` — handles full stop with orphan cleanup.
- `scripts/operations/runtime-status.ps1` — read-only status inspection (no elevation needed).

Manual step-by-step instructions below.

**Elevation required:** Yes (Administrator).

```powershell
# Step 1: Verify pre-conditions (see Section 10)
# Step 2: Check no stale listeners on port 9130
netstat -ano | Select-String ":9130"

# Step 3: Start the service
Start-Service -Name LibrarianRunTimeNode

# Step 4: Wait for Running state (up to 30s)
$svc = Get-Service LibrarianRunTimeNode
while ($svc.Status -ne 'Running') {
    Start-Sleep -Seconds 1
    $svc = Get-Service LibrarianRunTimeNode
}

# Step 5: Verify port 9130 listener appears
netstat -ano | Select-String ":9130.*LISTENING"

# Step 6: Verify router responds
Invoke-RestMethod http://127.0.0.1:9130/backend/status -ErrorAction SilentlyContinue

# Step 7: Record the start time and router PID
```

**Alternative:** The helper script `scripts/operations/runtime-start.ps1`
performs steps 3–5 with timeouts and reporting.

**After start:** The router is running but has **no model selected**.
Use `POST /backend/select` to activate a profile, or run
`.\scripts\operations\runtime-status.ps1` to inspect.

### 7.2 Stop Procedure

**Elevation required:** Yes (Administrator).

```powershell
# Step 1: Stop the service
Stop-Service -Name LibrarianRunTimeNode -Force

# Step 2: Wait for Stopped state (up to 30s)
$svc = Get-Service LibrarianRunTimeNode
while ($svc.Status -ne 'Stopped') {
    Start-Sleep -Seconds 1
    $svc = Get-Service LibrarianRunTimeNode
}

# Step 3: Verify port 9130 is free
netstat -ano | Select-String ":9130.*LISTENING"
# Expected: no output

# Step 4: Check for orphan processes
Get-Process -Name "rust-router","llama-server" -ErrorAction SilentlyContinue
# Expected: no output. If processes remain, see Failure Triage.

# Step 5: Record the stop time
```

**Alternative:** The helper script `scripts/operations/runtime-stop.ps1`
performs all steps with orphan cleanup and port verification.

### 7.3 Status Inspection (No Elevation)

```powershell
Get-Service LibrarianRunTimeNode
.\scripts\operations\runtime-status.ps1
```

---

## 8. Log and Evidence Capture

### 8.1 Log Locations

| Source | Path(s) |
|--------|---------|
| Service startup | `logs\service-router-startup.log` |
| Router (Rust) | `logs\rust-router-service.log` |
| Router (stderr) | `logs\router-stderr.log` |
| Router (stdout) | `logs\router-stdout.log` |
| Backend model | `logs\service-stderr.log`, `logs\service-stdout.log` |
| MCP status | `SessionStartup\MCP-STATUS.md` |

### 8.2 Capturing Evidence Before/After

**Before start:**
```powershell
# Record pre-start state
Get-Service LibrarianRunTimeNode | Export-Clixml logs\pre-start-service-state.xml
netstat -ano > logs\pre-start-ports.txt
Get-Process -Name "rust-router","llama-server" -ErrorAction SilentlyContinue |
    Export-Clixml logs\pre-start-processes.xml
```

**After stop:**
```powershell
# Record post-stop state
Get-Service LibrarianRunTimeNode | Export-Clixml logs\post-stop-service-state.xml
netstat -ano > logs\post-stop-ports.txt
Get-Process -Name "rust-router","llama-server" -ErrorAction SilentlyContinue |
    Export-Clixml logs\post-stop-processes.xml
```

**Evidence for receipt generation:**
- Service state before and after.
- Process list before and after.
- Port table before and after.
- Router endpoint response (if service was running).
- MCP health check status (if Librarian server was running).

### 8.3 Do Not Commit Logs

All files under `logs/` are gitignored. Do not commit log files, evidence
exports, or machine-local state snapshots to the repo.

---

## 9. Failure Triage

### 9.1 Service Fails to Start

| Symptom | Likely Cause | Check |
|---------|-------------|-------|
| `Start-Service` timeout | Backend port collision | Run `netstat -ano \| findstr ":9130"` — if LISTENING, kill the owning process |
| `Start-Service` access denied | Missing elevation | Run `whoami` and verify Admin |
| Port 9130 not listening after start | Router crashed on launch | Check `logs\router-stderr.log` and `logs\service-stderr.log` |
| Router responds 500 | Missing or invalid config | Verify `config\model-profiles.json` exists and parses as valid JSON |

### 9.2 Service Fails to Stop

| Symptom | Likely Cause | Check |
|---------|-------------|-------|
| Service stuck in `StopPending` | Orphan process not exiting | Run `Get-Process` for `rust-router*`, `llama-server*`, force-kill with `Stop-Process -Force` |
| Port 9130 still LISTENING | Orphan router process | Run `netstat -ano \| findstr ":9130"`, note PID, `Stop-Process -Id <PID> -Force` |
| `Stop-Service` access denied | Missing elevation | Run as Administrator |

### 9.3 Orphan Process Cleanup

```powershell
# Find orphan processes
$orphans = @()
$orphans += Get-Process -Name "rust-router" -ErrorAction SilentlyContinue
$orphans += Get-Process -Name "llama-server" -ErrorAction SilentlyContinue

# Kill each orphan
foreach ($p in $orphans) {
    Write-Host "Killing orphan: $($p.Name) PID $($p.Id)"
    Stop-Process -Id $p.Id -Force
}

# Verify cleanup
Start-Sleep -Seconds 2
Get-Process -Name "rust-router","llama-server" -ErrorAction SilentlyContinue
# Expected: no output
```

**Do not kill unrelated processes** — verify process ownership before killing.
Check `ParentProcessId` to confirm the process is a runtime-node child.

### 9.4 MCP Health Check Fails

| Failure | Check |
|---------|-------|
| Server unreachable (exit 1) | Is the Librarian Swift server running? Verify at `http://127.0.0.1:3456/api/health` |
| MCP endpoint unreachable (exit 2) | Is the Librarian server configured to serve MCP? Check `config/mcp-permissions.json` |
| Tools missing (exit 3) | Some expected MCP tools are not registered. Check version compatibility. |

### 9.5 Config Parse Error

```powershell
# Validate JSON configs
Get-Content config\model-profiles.json | ConvertFrom-Json | Out-Null
Get-Content config\model-profiles.local.json -ErrorAction SilentlyContinue |
    ConvertFrom-Json | Out-Null
```

If either fails, fix the config file before proceeding.

---

## 10. Do Not Proceed Conditions

If **any** of the following is true, stop and resolve before proceeding:

### 10.1 Hard Stops

| Condition | Action |
|-----------|--------|
| Service is already `Running` and you did not start it | Investigate who started it. Do not assume it is safe. |
| Port 9130 has an unexpected `LISTENING` entry | Record PID, investigate ownership before proceeding. |
| An orphan `llama-server.exe` or `rust-router.exe` is running from a prior session | Clean up via [section 9.3](#93-orphan-process-cleanup) before starting. |
| `config\model-profiles.json` does not parse as valid JSON | Fix the config file. Do not proceed with a broken profile map. |
| The backend binary is missing or is not `llama-server.exe` | Restore or rebuild `llama-server.exe`. Do not substitute with another binary. |
| You are not running as Administrator (for service operations) | Elevate. Do not mix unauth checks with auth operations. |
| An earlier step in this runbook failed and was not resolved | Do not skip failure resolution. Address the root cause first. |

### 10.2 Policy Stops

| Condition | Action |
|-----------|--------|
| The service startup type is not `Manual` | Do not change it without Owner approval. Record the current value. |
| The sprint scope does not include service lifecycle | Do not start the service. Document-only sprints must not mutate service state. |
| You are an agent (not a human operator) and the sprint says "do not automate" | Follow human-first instructions. Do not execute `Start-Service` or `Stop-Service` without explicit Owner instruction. |
| The runbook instructs you to execute steps you do not understand | Stop and ask. Do not run commands whose purpose you cannot explain. |

### 10.3 Boundary Stops

| Condition | Action |
|-----------|--------|
| The sprint scope is documentation/validation only | Do not start the service. Do not run models. Do not mutate router behavior. |
| You modified `router/`, `rust-router/`, `runtime/`, or `models/` | This is out of scope for a runbook sprint. Revert or get approval. |
| You are about to commit a machine-local path | Stop. Use a documented placeholder (e.g., `<repo-root>`) or local example pattern. |
| Service state validation requires a running service | If the service is not running and the sprint scope forbids starting it, mark the check as `SKIPPED (service not running)` — do not start it. |

---

## 11. Reference: Authoritative Values

### 11.1 Port Map

| Profile | Port | Source |
|---------|------|--------|
| Router | 9130 | `config/model-profiles.json`, `$env:ROUTER_PORT` fallback |
| phi-4 | 9120 | `config/model-profiles.json` |
| qwen-coder | 9121 | `config/model-profiles.json` |
| llama-3.2 | 9122 | `config/model-profiles.json` |
| qwen3 | 9123 | `config/model-profiles.json` |
| gemma-3 | 9124 | `config/model-profiles.json` |
| Embedding | 9125 | `runtime/model_manager.ps1` → `$EmbedPort` |

### 11.2 Binary Authority

| Field | Value |
|-------|-------|
| Authoritative backend binary | `llama-server.exe` |
| In-repo default location | `runtime/llama.cpp/llama-server.exe` |
| Historical alternate | `llama-server-mini.exe` (deprecated — do not use) |
| Config source | `config/model-profiles.json` → `defaults.binary` |

### 11.3 Service Identity

| Field | Value |
|-------|-------|
| Service name | `LibrarianRunTimeNode` |
| Startup type | **Manual** |
| NSSM binary | `runtime/bin/nssm.exe` (gitignored, do not commit) |
| Launcher script | `scripts/start-librarian-runtime-node.ps1` |

### 11.4 MCP Endpoints

| Field | Value |
|-------|-------|
| MCP endpoint URL | `http://127.0.0.1:3456/mcp` |
| Env var | `LIBRARIAN_MCP_URL` |
| Health check script | `scripts/check-mcp-health.ps1` |
| Stdio bridge script | `scripts/mcp-bridge.ps1` |
| Permission matrix | `config/mcp-permissions.json` |

### 11.5 Config File Sources

| File | Purpose | Gitignored |
|------|---------|-----------|
| `config/model-profiles.json` | Authoritative model profiles | No |
| `config/model-profiles.local.example.json` | Example local overrides | No |
| `config/model-profiles.local.json` | Machine-local overrides | Yes |
| `config/model_manager.local.example.ps1` | Example model manager overrides | No |
| `config/model_manager.local.ps1` | Machine-local manager overrides | Yes |
| `config/runtime-node.example.json` | Example runtime node config | No |
| `config/runtime-node.local.json` | Machine-local runtime config | Yes |
| `fixtures/startup-files-custody/machine-local-config.example.json` | Example custody config | No |
| `fixtures/startup-files-custody/startup-custody-manifest.example.json` | Reference manifest | No |
| `config/mcp-permissions.json` | MCP tool permission matrix | No |
| `mcp/templates/mcp-server.example.json` | MCP server config templates | No |

---

## Runbook Validation

After completing any operator procedure, verify:

- [ ] Working tree is clean (`git status`).
- [ ] No unexpected files modified.
- [ ] Service state matches expectation (Running / Stopped).
- [ ] Port map matches authoritative values.
- [ ] No orphan processes remain.
- [ ] Logs captured (if applicable).
- [ ] No machine-local values committed.
- [ ] Router, runtime, model files untouched.
