# Windows Agent-Host Environment Baseline

**Sprint:** WIN-AGENT-HARNESS-ENV-BASELINE-1
**Date:** 2026-06-29
**Host:** DESKTOP-ISNJ51B
**Repo:** `G:\OpenWork\librarian-runtime-node`
**HEAD:** `08a8602` — `docs(sprint): close WIN-RUNTIME-CONTROLLED-ACTIVATION-1 — PROMOTE`

---

## Table of Contents

1. [Durable State Verification](#1-durable-state-verification)
2. [Windows Version / Build / Edition](#2-windows-version--build--edition)
3. [Machine Identity](#3-machine-identity)
4. [CPU Topology](#4-cpu-topology)
5. [RAM](#5-ram)
6. [GPU / VRAM](#6-gpu--vram)
7. [Disks and Free Space](#7-disks-and-free-space)
8. [Network Profile and Local IP](#8-network-profile-and-local-ip)
9. [PowerShell Version and Execution Policy](#9-powershell-version-and-execution-policy)
10. [Git Version and Config](#10-git-version-and-config)
11. [Python Version](#11-python-version)
12. [Node / npm Version](#12-node--npm-version)
13. [Rust / Cargo Version](#13-rust--cargo-version)
14. [Visual Studio / MSVC / Build Tools](#14-visual-studio--msvc--build-tools)
15. [PATH Summary](#15-path-summary)
16. [Key Environment Variables (non-secret)](#16-key-environment-variables-non-secret)
17. [Repo Locations](#17-repo-locations)
18. [Allowed Writable Workspace Paths](#18-allowed-writable-workspace-paths)
19. [Forbidden / Secret-Risk Paths](#19-forbidden--secret-risk-paths)
20. [Service State](#20-service-state)
21. [Port State (9120–9125, 9130)](#21-port-state-91209125-9130)
22. [Orphan Process State](#22-orphan-process-state)
23. [Existing Harness / Check Scripts](#23-existing-harness--check-scripts)
24. [Findings Register](#24-findings-register)
25. [Recommended Next Sprint](#25-recommended-next-sprint)

---

## 1. Durable State Verification

### Current HEAD
```
08a8602 — docs(sprint): close WIN-RUNTIME-CONTROLLED-ACTIVATION-1 — PROMOTE
```
Full hash: `08a8602def6b8134b466b5046b7bed0e74c822eb`

### git status
```
On branch main
Your branch is ahead of 'origin/main' by 20 commits.
nothing to commit, working tree clean
```

### SESSION-HANDOFF.md
**Status:** EXISTS ✅ — Last updated 2026-06-23. HEAD `e7cfe33` recorded at that time (stale — current HEAD is `08a8602`).

### FEATURE-STATUS.md
**Status:** NOT FOUND ❌ — No `FEATURE-STATUS.md` file exists at the repo root.

### sprint-ledger.json
**Status:** NOT FOUND ❌ — No `sprint-ledger.json` file exists at the repo root.

### docs/planning/WIN-AGENT-HARNESS-PLAN.md
**Status:** NOT FOUND ❌ — Not yet created.

### docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md
**Status:** NOT FOUND ❌ — Not yet created.

### docs/planning/WIN-HARNESS-PARITY-ROADMAP.md
**Status:** NOT FOUND ❌ — Not yet created.

### docs/planning/WIN-LIBRARIAN-HOST-OPTIONS.md
**Status:** NOT FOUND ❌ — Not yet created.

### docs/planning/WIN-SPRINT-SEQUENCE.md
**Status:** NOT FOUND ❌ — Not yet created.

### docs/sprints/WIN-RUNTIME-CONTROLLED-ACTIVATION-1.md
**Status:** EXISTS ✅ — CLOSED — PROMOTE status.

### docs/receipts/WIN-RUNTIME-CONTROLLED-ACTIVATION-1-RECEIPT.md
**Status:** EXISTS ✅ — Closeout receipt for controlled activation sprint.

---

## 2. Windows Version / Build / Edition

| Property | Value |
|----------|-------|
| Caption | Microsoft Windows 10 Pro for Workstations |
| Version | 10.0.19045 |
| BuildNumber | 19045 |
| OSArchitecture | 64-bit |
| LastBootUpTime | 2026-06-27 20:48:49 UTC |

**Edition:** Windows 10 Pro for Workstations
**Build:** 19045 (22H2)

---

## 3. Machine Identity

| Property | Value |
|----------|-------|
| Name | DESKTOP-ISNJ51B |
| Manufacturer | MSI |
| Model | MS-7751 |
| SystemType | x64-based PC |
| TotalPhysicalMemory | 25,441,173,504 bytes (~24.3 GB) |

---

## 4. CPU Topology

| Property | Value |
|----------|-------|
| Name | Intel(R) Core(TM) i5-3570K CPU @ 3.40GHz |
| Cores | 4 |
| Logical Processors | 4 |
| Max Clock Speed | 3.40 GHz (3401 MHz reported) |
| L2 Cache | 1024 KB |
| L3 Cache | 6144 KB |

**Architecture:** Intel Ivy Bridge (3rd Gen)
**SMT/Hyper-Threading:** Not available (4C/4T)

---

## 5. RAM

| Property | Value |
|----------|-------|
| Total Physical Memory | 24.3 GB (25,441,173,504 bytes) |
| Usable | ~23.7 GB (after hardware reserved) |

*(Actual usable may vary depending on GPU shared memory allocation)*

---

## 6. GPU / VRAM

### GPU 1 — Integrated (Intel HD Graphics 4000)

| Property | Value |
|----------|-------|
| Name | Intel(R) HD Graphics 4000 |
| AdapterRAM | 2.25 GB (2,415,919,104 bytes) |
| DriverVersion | 10.18.10.4252 |
| PNPDeviceID | PCI\VEN_8086&DEV_0162 |

### GPU 2 — Discrete (AMD Radeon RX 570)

| Property | Value |
|----------|-------|
| Name | Radeon RX 570 Series |
| AdapterRAM | 4.0 GB (4,293,918,720 bytes) |
| DriverVersion | 31.0.21925.1001 |
| Current Mode | 1920 x 1080 x 4294967296 colors |
| PNPDeviceID | PCI\VEN_1002&DEV_67DF |

**Primary display GPU:** Radeon RX 570 4 GB
**VRAM available for model offload:** ~4 GB (3840 MB usable)

---

## 7. Disks and Free Space

| Drive | VolumeName | Size (GB) | Free (GB) | Free % |
|-------|------------|-----------|-----------|--------|
| C: | *(none)* | 111.16 | 10.2 | 9.2% |
| D: | SYSTEM RESERVED | 0.1 | 0.07 | 67.6% |
| F: | Acer | 430.66 | 348.07 | 80.8% |
| G: | *(none)* | 465.13 | 132.34 | 28.5% |

**Critical:** C: drive has only 10.2 GB free (9.2%) — very low. This may cause issues with build tools, Windows updates, and temp files.

---

## 8. Network Profile and Local IP

| Property | Value |
|----------|-------|
| Adapter | Broadcom 802.11ac Network Adapter |
| DHCP Enabled | Yes |
| IPv4 Address | 192.168.0.158 |
| IPv4 Subnet | 255.255.255.0 (fe80::/64 for IPv6) |
| IPv6 Addresses | `2607:f2c0:e4e1:fab0:38e8:ed2b:7ad:3b12`, `fe80::311d:1d56:7528:dfd2` |
| Default Gateway | (DHCP-assigned, not shown) |
| DNS Servers | (DHCP-assigned, not shown) |

**Profile type:** Private LAN (192.168.0.0/24)
**Connectivity:** Wi-Fi only

---

## 9. PowerShell Version and Execution Policy

| Property | Value |
|----------|-------|
| PSVersion | 5.1.19041.7417 |
| PSEdition | Desktop |
| PSCompatibleVersions | 1.0, 2.0, 3.0, 4.0, 5.0, 5.1 |
| BuildVersion | 10.0.19041.7417 |
| CLRVersion | 4.0.30319.42000 |
| ExecutionPolicy | **RemoteSigned** |

**Note:** PowerShell 5.1 only. No PowerShell Core (pwsh) detected in PATH. This is relevant for the existing PS5.1 parser encoding issue (em-dash fix in WIN-RUNTIME-CONTROLLED-ACTIVATION-1).

---

## 10. Git Version and Config

| Property | Value |
|----------|-------|
| Version | 2.54.0.windows.1 |
| core.autocrlf | `true` |
| core.hooksPath | *(not set)* |
| user.name | Andrew Hannah |
| user.email | andrewdhannah@users.noreply.github.com |
| Remote | `https://github.com/andrewdhannah/librarian-runtime-node.git` |
| Branch | `main` |

**Status:** Ahead of origin/main by 20 commits. Pending push.

---

## 11. Python Version

| Property | Value |
|----------|-------|
| Version | Python 3.14.3 |
| Primary path | `C:\Python314\python.exe` |
| Windows Store stub | `C:\Users\andre\AppData\Local\Microsoft\WindowsApps\python.exe` |
| python3 alias | Not available (Windows Store shortcut disabled) |

**Note:** The WindowsApps `python.exe` is a Microsoft Store stub. It appears in PATH before `C:\Python314\` only by App Execution Alias precedence. In practice, `C:\Python314\` is first in the system PATH entries so actual resolution should be correct.

---

## 12. Node / npm Version

| Property | Value |
|----------|-------|
| Node.js | v24.14.0 |
| npm | 11.9.0 |
| Path | `C:\Program Files\nodejs\` |

---

## 13. Rust / Cargo Version

| Property | Value |
|----------|-------|
| rustc | 1.96.0 (ac68faa20 2026-05-25) |
| cargo | 1.96.0 (30a34c682 2026-05-25) |
| Default host | x86_64-pc-windows-msvc |
| Toolchain | stable-x86_64-pc-windows-msvc (active, default) |
| rustup home | `C:\Users\andre\.rustup` |
| Binary paths | `C:\Users\andre\.cargo\bin\rustc.exe`, `C:\Users\andre\.cargo\bin\cargo.exe` |

### Installed Rust Targets
- `aarch64-linux-android`
- `armv7-linux-androideabi`
- `i686-linux-android`
- `x86_64-linux-android`
- `x86_64-pc-windows-msvc`

**Note:** Cross-compilation targets for Android are installed.

---

## 14. Visual Studio / MSVC / Build Tools

| Tool | Status |
|------|--------|
| Visual Studio 2022 BuildTools | Installed at `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools` |
| VS 18 BuildTools | Installed at `C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools` |
| MSBuild | Available at `C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\MSBuild\Current\bin\MSBuild.exe` |
| cl.exe (MSVC compiler) | **Not in PATH** — developer command prompt required |
| CMake | 4.3.3 (at `C:\Program Files\CMake\bin`) |
| dotnet | **Not installed** — SDK not found (only runtime? 'dotnet' at `C:\Program Files\dotnet\` but no SDK) |

**Note:** MSVC tools (`cl.exe`) are not accessible from this non-elevated PowerShell session. A Developer Command Prompt or `vcvarsall.bat` would be required for compilation, though Rust's MSVC toolchain may locate them automatically via the VS Installer.

---

## 15. PATH Summary

PATH entries (order as in environment, grouped by category):

### System / Windows
- `C:\WINDOWS\system32`
- `C:\WINDOWS`
- `C:\WINDOWS\System32\Wbem`
- `C:\WINDOWS\System32\WindowsPowerShell\v1.0\`
- `C:\WINDOWS\System32\OpenSSH\`

### Programming Languages
- `C:\Python314\`
- `C:\Python314\Scripts\`
- `C:\Program Files\nodejs\`
- `C:\Users\andre\.cargo\bin`
- `C:\Users\andre\.cargo\bin` (also from OpenWork sidecars entry)

### Java
- `C:\Program Files\Zulu\zulu-17\bin\`
- `C:\Program Files\BellSoft\LibericaJDK-17-Full\bin\`

### Build Tools
- `C:\Program Files\CMake\bin`
- `C:\Program Files\dotnet\`
- `C:\VulkanSDK\1.3.296.0\Bin`

### Version Control
- `C:\Program Files\Git\cmd`

### Package Managers
- `C:\ProgramData\chocolatey\bin`

### GPU / Graphics
- `C:\Program Files\NVIDIA Corporation\NVIDIA NvDLISR`
- `F:\Oculus\Support\oculus-runtime`
- `C:\VulkanSDK\1.3.296.0\Bin`

### Model Runtimes
- `C:\Users\andre\AppData\Local\Programs\Ollama`
- `C:\Users\andre\.lmstudio\bin`
- `G:\Downloads\g\Ollama`

### Editors / IDEs
- `G:\Microsoft VS Code\bin`

### OpenWork / Agent
- `C:\Users\andre\AppData\Local\Programs\@openworkdesktop\resources\sidecars`
- `C:\Users\andre\AppData\Roaming\npm`

### WindowsApps / WinGet
- `C:\Users\andre\AppData\Local\Microsoft\WindowsApps`
- `C:\Users\andre\AppData\Local\Microsoft\WinGet\Packages\Google.AndroidCLI_Microsoft.Winget.Source_8wekyb3d8bbwe`

### Other
- `C:\Program Files\WorldPainter`

---

## 16. Key Environment Variables (Non-Secret)

| Variable | Value |
|----------|-------|
| COMPUTERNAME | DESKTOP-ISNJ51B |
| USERNAME | andre |
| USERPROFILE | C:\Users\andre |
| HOMEDRIVE | C: |
| HOMEPATH | \Users\andre |
| SYSTEMDRIVE | C: |
| SYSTEMROOT | C:\WINDOWS |
| TEMP | C:\Users\andre\AppData\Local\Temp |
| TMP | C:\Users\andre\AppData\Local\Temp |
| OS | Windows_NT |
| PROCESSOR_ARCHITECTURE | AMD64 |
| NUMBER_OF_PROCESSORS | 4 |
| JAVA_HOME | C:\Program Files\Zulu\zulu-17\ |
| VK_SDK_PATH | C:\VulkanSDK\1.3.296.0 |
| VULKAN_SDK | C:\VulkanSDK\1.3.296.0 |
| OPENCODE_CONFIG | C:\Users\andre\AppData\Roaming\openwork\runtime-opencode-config.json |
| OPENWORK_SERVER_URL | http://127.0.0.1:50634 |
| OPENWORK_ELECTRON_REMOTE_DEBUG_PORT | 9223 |
| ChocolateyInstall | C:\ProgramData\chocolatey |
| AGENT | 1 |
| OPENCODE | 1 |
| OneDrive | C:\Users\andre\OneDrive |

---

## 17. Repo Locations

| Repo | Path |
|------|------|
| **librarian-runtime-node** | `G:\OpenWork\librarian-runtime-node\` |
| TheLibrarian-main (companion) | `G:\OpenWork\TheLibrarian-main\` |

---

## 18. Allowed Writable Workspace Paths

| Path | Owner | Notes |
|------|-------|-------|
| `G:\OpenWork\librarian-runtime-node` | DESKTOP-ISNJ51B\andre | Repo root |
| `G:\OpenWork\librarian-runtime-node\logs` | DESKTOP-ISNJ51B\andre | Gitignored |
| `G:\OpenWork\librarian-runtime-node\temp` | DESKTOP-ISNJ51B\andre | Gitignored |
| `G:\OpenWork\librarian-runtime-node\reports` | DESKTOP-ISNJ51B\andre | Reports output |
| `G:\OpenWork\librarian-runtime-node\docs` | DESKTOP-ISNJ51B\andre | Documentation |
| `C:\Users\andre\AppData\Local\Temp` | DESKTOP-ISNJ51B\andre | System temp |

---

## 19. Forbidden / Secret-Risk Paths

| Path | Risk |
|------|------|
| `config/runtime-node.local.json` | Local config — gitignored, may contain secrets |
| `*.env`, `.env` | Environment files — gitignored |
| `secrets/` | Secret directory — gitignored |
| `*.secret` | Secret files — gitignored |
| `models/` | Model binaries — gitignored |
| `runtime/llama.cpp/llama-server.exe` | Binary — gitignored by `*.exe` pattern + explicit `runtime/llama.cpp/` |
| `runtime/bin/nssm.exe` | Binary — gitignored by `*.exe` pattern + explicit `runtime/bin/` |
| `rust-router/target/` | Build artifacts — gitignored |
| `logs/` | Log directory — gitignored |

**Gitignored directly:**
- `*.gguf`, `*.safetensors`, `*.bin`, `*.pt`, `*.pth`, `*.onnx`
- `*.log`
- `__pycache__/`, `*.pyc`, `*.pyo`
- `.cache/`, `.zvec/`, `indexes/`, `vector-indexes/`, `*.zvec`, `*.zvecdb`
- `target/`
- `*.exe`, `*.dll`, `*.pdb`, `*.obj`, `*.exp`, `*.lib`

---

## 20. Service State

| Service | Status | StartType | ServiceType |
|---------|--------|-----------|-------------|
| `LibrarianRunTimeNode` | **Stopped** ✅ | **Manual** ✅ | Win32OwnProcess |

### NSSM Service Configuration

| Parameter | Value |
|-----------|-------|
| ImagePath | `G:\openwork\librarian-runtime-node\runtime\bin\nssm.exe` |
| DisplayName | Librarian Runtime Node |
| Start | 3 (SERVICE_DEMAND_START = Manual) |
| Type | 16 (SERVICE_WIN32_OWN_PROCESS) |
| ObjectName | LocalSystem |
| Application | `C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe` |
| AppDirectory | `G:\openwork\librarian-runtime-node` |
| AppParameters | `-NoProfile -ExecutionPolicy Bypass -File "G:\openwork\librarian-runtime-node\scripts\start-librarian-runtime-node.ps1"` |

**Note:** AppDirectory path uses lowercase `G:\openwork\librarian-runtime-node` in NSSM config vs capitalized `G:\OpenWork\librarian-runtime-node` on disk. This works because Windows paths are case-insensitive.

---

## 21. Port State (9120–9125, 9130)

| Port | Status |
|------|--------|
| 9120 (phi-4) | **FREE** ✅ |
| 9121 (qwen-coder) | **FREE** ✅ |
| 9122 (llama-3.2) | **FREE** ✅ |
| 9123 (qwen3) | **FREE** ✅ |
| 9124 (gemma-3) | **FREE** ✅ |
| 9125 *(unassigned)* | **FREE** ✅ |
| 9130 (Router) | **FREE** ✅ |

All 7 ports are free. No stale listeners or TIME_WAIT residue.

---

## 22. Orphan Process State

| Process | Running? |
|---------|----------|
| `llama-server.exe` | **0 running** ✅ |
| `rust-router.exe` | **0 running** ✅ |
| `python.exe` (router) | **0 running** ✅ |
| `LibrarianRunTimeNode` service | Stopped ✅ |

**No orphan processes detected.** The single `powershell.exe` child process is the current agent shell session.

---

## 23. Existing Harness / Check Scripts

### Harness / Integration Scripts
- `scripts/run-integration-proof-v2.ps1` — Automated lifecycle proof (v2 receipts)
- `scripts/run-runtime-qualification.ps1` — Governed rebuild + qualification
- `scripts/verify-runtime-qualification.ps1` — Qualification gate verifier
- `scripts/verify-receipt.ps1` — 48-check receipt verifier
- `scripts/run-win-rust-service-swap-proof.ps1` — Admin-elevated service swap proof

### Test Scripts
- `scripts/test-runtime-artifact-identity.ps1`
- `scripts/test-runtime-contract.ps1`
- `scripts/test-runtime-lifecycle.ps1`
- `scripts/test-runtime-network-boundary.ps1`
- `scripts/test-runtime-profiles.ps1`
- `scripts/test-runtime-limits.ps1`
- `scripts/test-runtime-cleanup.ps1`
- `scripts/test-context-fit.ps1`
- `scripts/test-model-fit.ps1`
- `scripts/test-reconcile-fit.ps1`
- `scripts/test-reconcile-optional.ps1`
- `scripts/test-reduced-offload-fit.ps1`
- `scripts/test-rust-router-endpoints.ps1`
- `scripts/test-rust-router-parity.ps1`
- `scripts/test-win-rust-service-swap.ps1`
- `scripts/test_network_boundary.ps1` (legacy naming)

### Operations Scripts
- `scripts/operations/runtime-start.ps1`
- `scripts/operations/runtime-stop.ps1`
- `scripts/operations/runtime-status.ps1`
- `scripts/operations/runtime-logs.ps1`
- `scripts/operations/runtime-clean-check.ps1`
- `scripts/start-librarian-runtime-node.ps1`
- `scripts/start-runtime.ps1`
- `scripts/stop-runtime.ps1`
- `scripts/start-phi4.example.ps1`
- `scripts/start-embeddings.example.ps1`

### Utility Scripts
- `scripts/check-mcp-health.ps1`
- `scripts/check-model-registry.ps1`
- `scripts/collect-inventory.ps1`
- `scripts/health-check.ps1`
- `scripts/list-models.ps1`
- `scripts/mcp-bridge.ps1`

### Python Test Harnesses
- `scripts/tests/test-win-runtime-operator-runbook.py`
- `scripts/tests/test-win-runtime-dry-run-readiness.py`
- `scripts/tests/test-win-runtime-dry-run-gap-close.py`
- `scripts/tests/test-startup-files-custody-inventory.py`
- `scripts/tests/test-router-context-runtime-design.py`
- `scripts/tests/test-router-context-runtime-contract.py`
- `scripts/tests/test-router-context-prototype.py`
- `scripts/tests/test-mcp-template-reconciliation.py`
- `scripts/tests/test-custody-normalization.py`
- `scripts/tests/test-context-route-contract.py`
- `scripts/tests/test-advisory-stub.py`

### Python Prototypes / Simulators
- `scripts/prototypes/router_context_decision_prototype.py`
- `scripts/simulators/context_reuse_simulator.py`
- `scripts/simulators/router_workload_optimizer.py`
- `scripts/measurements/measure-fast.py`
- `scripts/measurements/measure-router-context.ps1`
- `scripts/measurements/measure-router-context.py`

---

## 24. Findings Register

### Finding F-001: C: Drive Critically Low on Space

**Severity:** HIGH
**Description:** Drive C: has only 10.2 GB free (9.2% of 111 GB). This may cause build tool failures, Windows Update failures, temp file exhaustion, and general system instability.
**Recommended follow-up:** WIN-AGENT-HARNESS-CLEANUP-1 — free disk space on C: drive by moving large files, cleaning temp, disabling hibernation, or expanding storage.

### Finding F-002: dotnet SDK Not Found

**Severity:** MEDIUM
**Description:** `dotnet` at `C:\Program Files\dotnet\` exists but no .NET SDK is installed. Only a potential runtime exists. Any .NET-based development (e.g., Windows Librarian via .NET/WPF) cannot proceed.
**Recommended follow-up:** WIN-DOTNET-SDK-INSTALL-1 — install appropriate .NET SDK (8.0 or 9.0) if .NET tooling is needed.

### Finding F-003: MSVC Compiler Not in PATH

**Severity:** LOW
**Description:** `cl.exe` is not accessible from the default PowerShell session. A Developer Command Prompt or `vcvarsall.bat` invocation is required. Rust's MSVC toolchain may resolve this via VS Installer detection, but it has not been tested from this session.
**Recommended follow-up:** WIN-MSVCPATH-BASELINE-1 — test Rust build from this session, verify MSVC resolution, and document the vcvarsall path.

### Finding F-004: SESSION-HANDOFF.md Stale

**Severity:** LOW
**Description:** SESSION-HANDOFF.md records HEAD as `e7cfe33` but current HEAD is `08a8602`. The handoff also references a sprint sequence that does not include the latest sprints (CONTROLLED-ACTIVATION-1 et al.).
**Recommended follow-up:** Minor update to SESSION-HANDOFF.md as part of sprint closeout.

### Finding F-005: No FEATURE-STATUS.md or sprint-ledger.json

**Severity:** LOW
**Description:** The repo lacks a consolidated feature status file or sprint ledger. Sprint history must be inferred from `docs/sprints/` directory listing and `docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md`.
**Recommended follow-up:** WIN-SPRINT-LEDGER-1 — create a sprint ledger or consolidate status tracking.

### Finding F-006: Multiple Planning Docs Missing

**Severity:** MEDIUM
**Description:** The following planning documents referenced by this baseline inventory do not yet exist:
- `docs/planning/WIN-AGENT-HARNESS-PLAN.md`
- `docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md`
- `docs/planning/WIN-HARNESS-PARITY-ROADMAP.md`
- `docs/planning/WIN-LIBRARIAN-HOST-OPTIONS.md`
- `docs/planning/WIN-SPRINT-SEQUENCE.md`
**Recommended follow-up:** WIN-AGENT-HARNESS-PLAN-1 — create the harness plan and supporting documents as the next sprint.

### Finding F-007: Windows 10 Pro for Workstations — Build 19045 (22H2)

**Severity:** INFO
**Description:** This machine runs Windows 10 22H2, which is the final feature update for Windows 10 (EOS October 2025 — already past). No security updates after October 2025. This may be a compliance concern for production use.
**Recommended follow-up:** WIN-WINDOWS-UPGRADE-EVAL-1 — evaluate whether Windows 11 upgrade is needed for the Librarian host role.

### Finding F-008: Multiple Ollama/LM Studio Paths

**Severity:** LOW
**Description:** PATH contains entries for both Ollama (`C:\Users\andre\AppData\Local\Programs\Ollama` and `G:\Downloads\g\Ollama`) and LM Studio (`C:\Users\andre\.lmstudio\bin`). These are alternative runtimes, not used by the librarian-runtime-node which uses `llama-server.exe` directly.
**Recommended follow-up:** PATH hygiene check in a future cleanup sprint.

---

## 25. Recommended Next Sprint

### WIN-AGENT-HARNESS-PLAN-1 — Agent Harness Planning

**Purpose:** Create the governing plan documents for the Windows agent harness work lane.

**Scope:**
1. `docs/planning/WIN-AGENT-HARNESS-PLAN.md` — overall harness architecture and goals
2. `docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md` — how a harness sandbox maintains custody boundaries
3. `docs/planning/WIN-HARNESS-PARITY-ROADMAP.md` — parity targets with the Mac-side harness
4. `docs/planning/WIN-LIBRARIAN-HOST-OPTIONS.md` — host technology options for the Windows Librarian
5. `docs/planning/WIN-SPRINT-SEQUENCE.md` — forward sprint sequence incorporating harness work

**Dependency:** This baseline report provides the environment data needed for informed planning.

**Priority:** HIGH — these documents define the governance model for all future harness sprints.

---

## Appendix A: Baseline Verification Pass/Fail Summary

| Check | Expected | Actual | Pass/Fail |
|-------|----------|--------|-----------|
| HEAD recorded | Current commit hash | `08a8602` | ✅ PASS |
| Working tree clean | clean | clean | ✅ PASS |
| Service Stopped / Manual | Stopped / Manual | Stopped / Manual | ✅ PASS |
| Port 9130 free | FREE | FREE | ✅ PASS |
| Ports 9120–9125 free | FREE | FREE | ✅ PASS |
| No orphan llama-server | 0 | 0 | ✅ PASS |
| No orphan rust-router | 0 | 0 | ✅ PASS |
| No orphan python router | 0 | 0 | ✅ PASS |
| Elevation check recorded | documented | Non-admin | ✅ INFO |
| All tool versions recorded | documented | documented | ✅ PASS |
| Service NSSM config recorded | documented | documented | ✅ PASS |
| PATH recorded | documented | documented | ✅ PASS |
| Environment variables recorded | documented | documented | ✅ PASS |
| Findings documented | all | 8 findings | ✅ PASS |

**Overall: PASS** — All acceptance gates met. Inventory complete. No code or service state mutated.
