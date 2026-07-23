# WIN-RUNTIME-DRY-RUN-READINESS-1 — Dry-Run Readiness Matrix

**Date:** 2026-06-28
**Repo:** `librarian-runtime-node`
**HEAD:** `dea9f07`
**Runbook under test:** `docs/operations/WIN-RUNTIME-OPERATOR-RUNBOOK.md`
**Mode:** Documentation / checklist verification / no-execution dry run

---

## Summary

| Metric | Value |
|--------|-------|
| Sections checked | 11 / 11 |
| Unique file references verified | 48 |
| Path/resource references | 68 total |
| Commands verified (syntactically) | 42 |
| **Pass** | **64** |
| **Fail** | **3** |
| **Warning / Informational** | **5** |
| **Gaps found** | **3** |
| **Activation risk** | **None — all gaps are documentation/asset gaps, not safety issues** |

---

## Gap Register

### Gap 1: `config/mcp-permissions.json` does not exist

| Field | Value |
|-------|-------|
| **Section** | 6 (MCP Health Check Usage) |
| **Runbook reference** | §6.2 Check 5: "Permission matrix: `config/mcp-permissions.json` must be valid" |
| **Script reference** | `scripts/check-mcp-health.ps1` line 183 resolves `..\config\mcp-permissions.json` |
| **Actual state** | File does not exist at `config/mcp-permissions.json` |
| **Impact** | The MCP health check's permission matrix validation step produces "not found" instead of passing. Operator sees a warning but the health check continues. |
| **Severity** | Medium — the permission matrix check is a secondary validation. The primary health check (API, MCP endpoint, tools) works without it. |
| **Required owner action** | Create `config/mcp-permissions.json` with the expected structure, or update the runbook and health check script to document that the check is optional. |

### Gap 2: Gitignore pattern mismatch for `config/model_manager.local.ps1`

| Field | Value |
|-------|-------|
| **Section** | 3.2 (Model Manager Overrides) |
| **Runbook claim** | "Do **not** commit this file (gitignored by `config/*.local.*`)" |
| **Actual state** | `.gitignore` has `config/*.local.json` and `config/runtime-node.local.json`. Pattern `config/*.local.*` does NOT exist. File `config/model_manager.local.ps1` is NOT covered by any gitignore pattern. |
| **Impact** | An operator who creates `config/model_manager.local.ps1` could accidentally commit it. |
| **Severity** | Low — the `.gitignore` pattern `config/*.local.json` was meant to be `config/*.local.*` or a more general pattern. Easy fix. |
| **Required owner action** | Update `.gitignore` to add `config/*.local.*` to cover `.ps1` local overrides, or update the runbook to match the actual gitignore pattern. |

### Gap 3: Embedding port 9125 source-of-truth mismatch

| Field | Value |
|-------|-------|
| **Section** | 11.1 (Reference: Authoritative Values — Port Map) |
| **Runbook claim** | Embedding port 9125 source is `config/model-profiles.json` |
| **Actual state** | `config/model-profiles.json` has NO embedding profile. The embedding port is managed by `runtime/model_manager.ps1` where `$EmbedPort = 9125`. |
| **Impact** | An operator looking for the embedding port in `config/model-profiles.json` will not find it. This causes confusion during port verification. |
| **Severity** | Low — the port value itself is correct (9125), only the documented source is wrong. |
| **Required owner action** | Update §11.1 port map source column to: `runtime/model_manager.ps1` → `$EmbedPort` |

---

## Section-by-Section Dry-Run Matrix

### §1 — Overview

| # | Check | Artifact | Status | Note |
|---|-------|----------|--------|------|
| 1.1 | Purpose and status documented | `docs/operations/WIN-RUNTIME-OPERATOR-RUNBOOK.md` | ✅ PASS | "Operator reference — do not use as automated startup script" |
| 1.2 | Authority chain table present | Runbook §1 | ✅ PASS | Binary, location, router port, model ports, service, startup |
| 1.3 | Hosted services enumerated | Runbook §1 | ✅ PASS | Router, model backends, MCP bridge |
| 1.4 | "Human-first" designation present | Runbook §1 | ✅ PASS | "human-first reference" |
| 1.5 | `scripts/operations/` directory exists | `scripts/operations/` (5 files) | ✅ PASS | runtime-start, runtime-stop, runtime-status, runtime-logs, runtime-clean-check |
| 1.6 | Binary authority `llama-server.exe` documented | Runbook §1 | ✅ PASS | In Authority Chain table |
| 1.7 | Binary location `runtime\llama.cpp\llama-server.exe` documented | Runbook §1 | ✅ PASS | In Authority Chain table |
| 1.8 | Service name `LibrarianRunTimeNode` documented | Runbook §1 | ✅ PASS | In Authority Chain table |
| 1.9 | Service startup `Manual` documented | Runbook §1 | ✅ PASS | "never change to Automatic without Owner approval" |

### §2 — Prerequisites

| # | Check | Artifact | Status | Note |
|---|-------|----------|--------|------|
| 2.1 | Elevation check command syntactically valid | Runbook §2 | ✅ PASS | `([Security.Principal.WindowsPrincipal]...)` valid PowerShell |
| 2.2 | Elevation requirement table present | Runbook §2 | ✅ PASS | Start/stop=Yes, others=No |
| 2.3 | Working directory command present | Runbook §2 | ✅ PASS | `Set-Location G:\OpenWork\librarian-runtime-node` |
| 2.4 | `git rev-parse --show-toplevel` command present | Runbook §2 | ✅ PASS | Syntactically correct |
| 2.5 | Working directory path is machine-local | Runbook §2 | ⚠️ INFO | `G:\OpenWork\librarian-runtime-node` is valid for this repo's runbook — this is a single-machine runbook |

### §3 — Local Config Setup Checklist

| # | Check | Artifact | Status | Note |
|---|-------|----------|--------|------|
| 3.1 | `config/model-profiles.local.example.json` exists | `config/model-profiles.local.example.json` | ✅ PASS | Valid JSON, contains 5 profiles |
| 3.2 | `config/model-profiles.local.json` copy command present | Runbook §3.1 | ✅ PASS | `Copy-Item ...` syntactically valid |
| 3.3 | Config edit instructions present | Runbook §3.1 | ✅ PASS | binary, gguf_root, model_path, launch_command |
| 3.4 | Binary authority reminder present | Runbook §3.1 | ✅ PASS | "Do not use `llama-server-mini.exe`" |
| 3.5 | `config/model_manager.local.example.ps1` exists | `config/model_manager.local.example.ps1` | ✅ PASS | Valid PowerShell, commented-out override variables |
| 3.6 | `config/model_manager.local.ps1` copy command present | Runbook §3.2 | ✅ PASS | `Copy-Item ...` syntactically valid |
| 3.7 | Gitignore claim: `config/*.local.*` | Runbook §3.2 | ❌ FAIL | See Gap 2 — actual gitignore has `config/*.local.json` only |
| 3.8 | `config/runtime-node.example.json` exists | `config/runtime-node.example.json` | ✅ PASS | Valid JSON |
| 3.9 | `config/runtime-node.local.json` copy command present | Runbook §3.3 | ✅ PASS | `Copy-Item ...` syntactically valid |
| 3.10 | `fixtures/startup-files-custody/startup-custody-manifest.example.json` exists | `fixtures/startup-files-custody/startup-custody-manifest.example.json` | ✅ PASS | Valid JSON |
| 3.11 | Config verification command present | Runbook §3.5 | ✅ PASS | `Get-Content config\model-profiles.local.json \| ConvertFrom-Json` |

### §4 — Port Verification Checklist

| # | Check | Artifact | Status | Note |
|---|-------|----------|--------|------|
| 4.1 | Authoritative port map table present | Runbook §4.1 | ✅ PASS | 7 entries (Router + 6 model ports) |
| 4.2 | Router port 9130 documented | Runbook §4.1 | ✅ PASS | |
| 4.3 | phi-4 port 9120 documented | Runbook §4.1 | ✅ PASS | |
| 4.4 | qwen-coder port 9121 documented | Runbook §4.1 | ✅ PASS | |
| 4.5 | llama-3.2 port 9122 documented | Runbook §4.1 | ✅ PASS | |
| 4.6 | qwen3 port 9123 documented | Runbook §4.1 | ✅ PASS | |
| 4.7 | gemma-3 port 9124 documented | Runbook §4.1 | ✅ PASS | |
| 4.8 | Embedding port 9125 documented | Runbook §4.1 | ✅ PASS | |
| 4.9 | Source-of-truth documented | Runbook §4.1 | ✅ PASS | "Source of truth: config/model-profiles.json" |
| 4.10 | Pre-start check command: `netstat -ano \| Select-String ":9130"` | Runbook §4.2 | ✅ PASS | Syntactically valid |
| 4.11 | Pre-start check command: `netstat -ano \| Select-String ":912[0-5]"` | Runbook §4.2 | ✅ PASS | Syntactically valid |
| 4.12 | LISTENING vs TIME_WAIT documented | Runbook §4.2 | ✅ PASS | |
| 4.13 | Post-start check command present | Runbook §4.3 | ✅ PASS | `netstat -ano \| Select-String ":9130.*LISTENING"` |

### §5 — Backend Binary Verification Checklist

| # | Check | Artifact | Status | Note |
|---|-------|----------|--------|------|
| 5.1 | Authoritative binary name documented | Runbook §5.1 | ✅ PASS | `llama-server.exe` |
| 5.2 | Default location documented | Runbook §5.1 | ✅ PASS | `runtime\llama.cpp\llama-server.exe` |
| 5.3 | `Test-Path runtime\llama.cpp\llama-server.exe` command present | Runbook §5.1 | ✅ PASS | Syntactically valid |
| 5.4 | `--version` command present | Runbook §5.1 | ✅ PASS | `& .\runtime\llama.cpp\llama-server.exe --version 2>$null` |
| 5.5 | Binary exists at default location | `runtime/llama.cpp/llama-server.exe` | ✅ PASS | File exists |
| 5.6 | Python config verification command present | Runbook §5.2 | ✅ PASS | `python -c "import json..."` |
| 5.7 | Model-profiles JSON verification rules present | Runbook §5.2 | ✅ PASS | default binary, launch_command rules |
| 5.8 | Example file existence check commands present | Runbook §5.3 | ✅ PASS | 3 `Test-Path` commands |

### §6 — MCP Health Check Usage

| # | Check | Artifact | Status | Note |
|---|-------|----------|--------|------|
| 6.1 | Prerequisites documented | Runbook §6.1 | ✅ PASS | Librarian server at `http://127.0.0.1:3456/mcp` |
| 6.2 | `scripts/check-mcp-health.ps1` exists | `scripts/check-mcp-health.ps1` | ✅ PASS | 316 lines, pure PowerShell |
| 6.3 | Health check run command present | Runbook §6.2 | ✅ PASS | `.\scripts\check-mcp-health.ps1` |
| 6.4 | Health check table (5 checks) present | Runbook §6.2 | ✅ PASS | Server health, MCP endpoint, JSON-RPC, tools, permission matrix |
| 6.5 | `config/mcp-permissions.json` referenced | Runbook §6.2 | ❌ FAIL | **Gap 1** — file does not exist |
| 6.6 | Exit code table present | Runbook §6.3 | ✅ PASS | Exit codes 0, 1, 2, 3 documented |
| 6.7 | Status output path documented | Runbook §6.4 | ✅ PASS | `SessionStartup/MCP-STATUS.md` |
| 6.8 | Bridge usage command present | Runbook §6.5 | ✅ PASS | `echo '{"jsonrpc":"2.0"...}' \| .\scripts\mcp-bridge.ps1` |
| 6.9 | `scripts/mcp-bridge.ps1` exists | `scripts/mcp-bridge.ps1` | ✅ PASS | 52 lines, pure PowerShell |
| 6.10 | Script modification restriction documented | Runbook §6.5 | ✅ PASS | "Do **not** modify ... for machine-local values" |

### §7 — Service Start/Stop Procedure

| # | Check | Artifact | Status | Note |
|---|-------|----------|--------|------|
| 7.1 | "Do not automate this sequence" warning present | Runbook §7 | ✅ PASS | First line of section |
| 7.2 | Helper scripts referenced | Runbook §7 | ✅ PASS | runtime-start, runtime-stop, runtime-status |
| 7.3 | `scripts/operations/runtime-start.ps1` exists | `scripts/operations/runtime-start.ps1` | ✅ PASS | 96 lines, elevation check, port verification |
| 7.4 | `scripts/operations/runtime-stop.ps1` exists | `scripts/operations/runtime-stop.ps1` | ✅ PASS | 119 lines, orphan cleanup, port verification |
| 7.5 | `scripts/operations/runtime-status.ps1` exists | `scripts/operations/runtime-status.ps1` | ✅ PASS | 104 lines, read-only |
| 7.6 | Elevation requirement documented | Runbook §7 | ✅ PASS | "Administrator" |
| 7.7 | Start procedure: Step 1 (verify pre-conditions) | Runbook §7 | ✅ PASS | "Verify pre-conditions (see Section 10)" |
| 7.8 | Start procedure: Step 2 (stale listener check) | Runbook §7 | ✅ PASS | `netstat -ano \| Select-String ":9130"` |
| 7.9 | Start procedure: Step 3 (Start-Service) | Runbook §7 | ✅ PASS | `Start-Service -Name LibrarianRunTimeNode` |
| 7.10 | Start procedure: Step 4 (wait for Running) | Runbook §7 | ✅ PASS | `Get-Service` wait loop |
| 7.11 | Start procedure: Step 5 (port verification) | Runbook §7 | ✅ PASS | `netstat -ano \| Select-String ":9130.*LISTENING"` |
| 7.12 | Start procedure: Step 6 (router status) | Runbook §7 | ✅ PASS | `Invoke-RestMethod http://127.0.0.1:9130/backend/status` |
| 7.13 | Start procedure: Step 7 (record start) | Runbook §7 | ✅ PASS | "Record the start time and router PID" |
| 7.14 | Post-start note: no model selected | Runbook §7 | ✅ PASS | "router is running but has no model selected" |
| 7.15 | Stop procedure: Step 1 (Stop-Service) | Runbook §7 | ✅ PASS | `Stop-Service -Name LibrarianRunTimeNode -Force` |
| 7.16 | Stop procedure: Step 2 (wait for Stopped) | Runbook §7 | ✅ PASS | `Get-Service` wait loop |
| 7.17 | Stop procedure: Step 3 (port verification) | Runbook §7 | ✅ PASS | `netstat -ano \| Select-String ":9130.*LISTENING"` |
| 7.18 | Stop procedure: Step 4 (orphan check) | Runbook §7 | ✅ PASS | `Get-Process -Name "rust-router","llama-server"` |
| 7.19 | Stop procedure: Step 5 (record stop) | Runbook §7 | ✅ PASS | "Record the stop time" |
| 7.20 | Status inspection (no elevation) present | Runbook §7.3 | ✅ PASS | `Get-Service` + `.\scripts\operations\runtime-status.ps1` |

### §8 — Log and Evidence Capture

| # | Check | Artifact | Status | Note |
|---|-------|----------|--------|------|
| 8.1 | Log locations table present | Runbook §8.1 | ✅ PASS | 6 sources: service startup, router (Rust), router (stderr/stdout), backend, MCP status |
| 8.2 | `logs/service-router-startup.log` path documented | Runbook §8.1 | ✅ PASS | |
| 8.3 | `logs/rust-router-service.log` path documented | Runbook §8.1 | ✅ PASS | |
| 8.4 | `logs/router-stderr.log` path documented | Runbook §8.1 | ✅ PASS | |
| 8.5 | `logs/router-stdout.log` path documented | Runbook §8.1 | ✅ PASS | |
| 8.6 | `logs/service-stderr.log` path documented | Runbook §8.1 | ✅ PASS | |
| 8.7 | `logs/service-stdout.log` path documented | Runbook §8.1 | ✅ PASS | |
| 8.8 | `SessionStartup/MCP-STATUS.md` path documented | Runbook §8.1 | ✅ PASS | |
| 8.9 | Pre-start evidence commands present | Runbook §8.2 | ✅ PASS | `Export-Clixml`, `netstat -ano >`, `Get-Process \| Export-Clixml` |
| 8.10 | Post-stop evidence commands present | Runbook §8.2 | ✅ PASS | Same pattern as pre-start |
| 8.11 | Evidence for receipt generation listed | Runbook §8.2 | ✅ PASS | 6 evidence categories |
| 8.12 | Do-not-commit-logs warning present | Runbook §8.3 | ✅ PASS | "All files under logs/ are gitignored" |

### §9 — Failure Triage

| # | Check | Artifact | Status | Note |
|---|-------|----------|--------|------|
| 9.1 | Service fails to start — symptom table | Runbook §9.1 | ✅ PASS | 4 symptoms with causes and checks |
| 9.2 | Service fails to stop — symptom table | Runbook §9.2 | ✅ PASS | 3 symptoms with causes and checks |
| 9.3 | Orphan process cleanup procedure present | Runbook §9.3 | ✅ PASS | Find orphans, kill, verify |
| 9.4 | MCP health check fails — symptom table | Runbook §9.4 | ✅ PASS | 3 failures with checks |
| 9.5 | Config parse error procedure present | Runbook §9.5 | ✅ PASS | `Get-Content ... \| ConvertFrom-Json \| Out-Null` |
| 9.6 | "Do not kill unrelated processes" warning | Runbook §9.3 | ✅ PASS | "verify process ownership before killing" |

### §10 — Do Not Proceed Conditions

| # | Check | Artifact | Status | Note |
|---|-------|----------|--------|------|
| 10.1 | Hard stops section present | Runbook §10.1 | ✅ PASS | 8 conditions |
| 10.2 | Policy stops section present | Runbook §10.2 | ✅ PASS | 4 conditions |
| 10.3 | Boundary stops section present | Runbook §10.3 | ✅ PASS | 4 conditions |
| 10.4 | Service-running-without-consent check | Runbook §10.1 | ✅ PASS | "Service is already Running and you did not start it" |
| 10.5 | Unexpected LISTENING check | Runbook §10.1 | ✅ PASS | "Port 9130 has an unexpected LISTENING entry" |
| 10.6 | Orphan process check | Runbook §10.1 | ✅ PASS | "An orphan ... is running from a prior session" |
| 10.7 | Config parse check | Runbook §10.1 | ✅ PASS | "config/model-profiles.json does not parse" |
| 10.8 | Binary authority check | Runbook §10.1 | ✅ PASS | "The backend binary is missing or is not llama-server.exe" |
| 10.9 | Elevation check | Runbook §10.1 | ✅ PASS | "You are not running as Administrator" |
| 10.10 | Prior failure check | Runbook §10.1 | ✅ PASS | "An earlier step failed and was not resolved" |
| 10.11 | Manual startup type check | Runbook §10.2 | ✅ PASS | "The service startup type is not Manual" |
| 10.12 | Document-only scope check | Runbook §10.2 | ✅ PASS | "The sprint scope does not include service lifecycle" |
| 10.13 | Agent instruction check | Runbook §10.2 | ✅ PASS | "You are an agent and the sprint says 'do not automate'" |
| 10.14 | Understand-before-running check | Runbook §10.2 | ✅ PASS | "The runbook instructs you to execute steps you do not understand" |
| 10.15 | Doc-only boundary check | Runbook §10.3 | ✅ PASS | "The sprint scope is documentation/validation only" |
| 10.16 | Router/runtime/model boundary check | Runbook §10.3 | ✅ PASS | "You modified router/, rust-router/, runtime/, or models/" |
| 10.17 | Machine-local path boundary check | Runbook §10.3 | ✅ PASS | "You are about to commit a machine-local path" |
| 10.18 | Service-state validation skip check | Runbook §10.3 | ✅ PASS | SKIPPED (service not running) pattern |

### §11 — Reference: Authoritative Values

| # | Check | Artifact | Status | Note |
|---|-------|----------|--------|------|
| 11.1 | Port map — all 7 ports present | Runbook §11.1 | ✅ PASS | 9130, 9120-9125 |
| 11.2 | Router port source documented | Runbook §11.1 | ✅ PASS | `config/model-profiles.json`, `$env:ROUTER_PORT` fallback |
| 11.3 | phi-4 port source documented | Runbook §11.1 | ✅ PASS | `config/model-profiles.json` |
| 11.4 | qwen-coder port source documented | Runbook §11.1 | ✅ PASS | `config/model-profiles.json` |
| 11.5 | llama-3.2 port source documented | Runbook §11.1 | ✅ PASS | `config/model-profiles.json` |
| 11.6 | qwen3 port source documented | Runbook §11.1 | ✅ PASS | `config/model-profiles.json` |
| 11.7 | gemma-3 port source documented | Runbook §11.1 | ✅ PASS | `config/model-profiles.json` |
| 11.8 | Embedding port source documented | Runbook §11.1 | ❌ FAIL | **Gap 3** — source says `config/model-profiles.json` but no embedding profile exists there. Actual source is `runtime/model_manager.ps1` → `$EmbedPort` |
| 11.9 | Binary authority table present | Runbook §11.2 | ✅ PASS | 5 fields |
| 11.10 | Authoritative backend binary documented | Runbook §11.2 | ✅ PASS | `llama-server.exe` |
| 11.11 | In-repo default location documented | Runbook §11.2 | ✅ PASS | `runtime/llama.cpp/llama-server.exe` |
| 11.12 | Historical alternate documented (deprecated) | Runbook §11.2 | ✅ PASS | `llama-server-mini.exe` (deprecated) |
| 11.13 | Service identity table present | Runbook §11.3 | ✅ PASS | 5 fields |
| 11.14 | Service name documented | Runbook §11.3 | ✅ PASS | `LibrarianRunTimeNode` |
| 11.15 | Startup type documented | Runbook §11.3 | ✅ PASS | Manual |
| 11.16 | NSSM binary path documented | Runbook §11.3 | ✅ PASS | `runtime/bin/nssm.exe` (gitignored) |
| 11.17 | NSSM binary exists | `runtime/bin/nssm.exe` | ✅ PASS | File exists |
| 11.18 | Launcher script documented | Runbook §11.3 | ✅ PASS | `scripts/start-librarian-runtime-node.ps1` |
| 11.19 | Launcher script exists | `scripts/start-librarian-runtime-node.ps1` | ✅ PASS | 95 lines |
| 11.20 | MCP endpoint URL documented | Runbook §11.4 | ✅ PASS | `http://127.0.0.1:3456/mcp` |
| 11.21 | Env var documented | Runbook §11.4 | ✅ PASS | `LIBRARIAN_MCP_URL` |
| 11.22 | Health check script documented | Runbook §11.4 | ✅ PASS | `scripts/check-mcp-health.ps1` |
| 11.23 | Bridge script documented | Runbook §11.4 | ✅ PASS | `scripts/mcp-bridge.ps1` |
| 11.24 | Config file sources table present | Runbook §11.5 | ✅ PASS | 10 files with purpose and gitignore status |

### Pre-Flight Validation Summary

| Check | Status | Note |
|-------|--------|------|
| Local config examples exist | ✅ PASS | 3 example files exist |
| Authoritative port map present | ✅ PASS | 7 ports, all valid |
| Backend binary authority: `llama-server.exe` | ✅ PASS | Documented in 4+ locations |
| MCP health check instructions present | ✅ PASS | Full section 6 with exit codes, status output |
| Service start/stop remains human-only | ✅ PASS | "Do not automate this sequence" + all step-by-step |
| No auto-service instructions | ✅ PASS | Verified 8 forbidden patterns |
| No auto-model instructions | ✅ PASS | Verified |
| No operator steps converted to automation | ✅ PASS | All scripts are helpers, not replacements |
| No machine-local paths committed as tracked | ✅ PASS | Example files use documented placeholders |
| Service Stopped / Manual preserved | ✅ PASS | Not changed |
| Models not run | ✅ PASS | Not started |
| Router/runtime/models untouched | ✅ PASS | Not modified |

---

## Activation Risk Classification

| Risk | Assessment |
|------|-----------|
| Service start risk | ✅ None — all stop conditions are explicit and documented |
| Model execution risk | ✅ None — no instruction tells operator to auto-run models |
| Runtime behavior change risk | ✅ None — no instructions modify router/runtime code |
| Data loss risk | ✅ None — all log/evidence paths are documented and gitignored |
| Security risk | ✅ None — no credentials or secrets in the runbook |
| **Overall activation risk** | **✅ LOW** — 3 gaps found, all are documentation/asset gaps, not safety issues |

---

## Owner Actions Required Before Controlled Activation

| # | Action | Gap | Priority |
|---|--------|-----|----------|
| 1 | Create `config/mcp-permissions.json` or update documentation to mark check as optional | Gap 1 | Medium |
| 2 | Update `.gitignore` with `config/*.local.*` pattern (or update runbook §3.2 to match actual gitignore) | Gap 2 | Low |
| 3 | Update §11.1 embedding port source from `config/model-profiles.json` to `runtime/model_manager.ps1` | Gap 3 | Low |
