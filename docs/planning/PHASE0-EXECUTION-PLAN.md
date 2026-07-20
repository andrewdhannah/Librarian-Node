# Phase 0 Execution Plan

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  
**Purpose:** Produce a complete evidence-based audit of the Windows Runtime Node before any corrective work begins.

---

## 1. Objective

Produce a complete factual audit of the Windows Runtime Node in its current state. The scope is strictly observational. No modifications to source code, configuration, databases, dependencies, services, security settings, or runtime behavior.

---

## 2. Scope

### 2.1 Collect

| Area | Artifact |
|------|----------|
| Runtime process inventory | `evidence/phase0/process-list.md` |
| MCP connectivity and configuration | `evidence/phase0/mcp-status.md` |
| Network listeners | `evidence/phase0/network-listeners.md` |
| Rust toolchain and dependency versions | `evidence/phase0/rust-version.md` |
| GPU and hardware inventory | `evidence/phase0/hardware.md`, `evidence/phase0/gpu.md` |
| Database presence, schema versions, migration state | `evidence/phase0/database-status.md` |
| Registry reconciliation status | `evidence/phase0/registry-status.md` |
| Startup and error logs | `evidence/phase0/startup-log.md`, `evidence/phase0/error-log.md` |
| Known failures and reproducible symptoms | `evidence/phase0/known-issues.md` |
| Installed software inventory | `evidence/phase0/installed-software.md` |
| Runtime configuration | `evidence/phase0/runtime-config.md` |
| Dependency list | `evidence/phase0/dependency-list.md` |
| llama.cpp version | `evidence/phase0/llama-version.md` |
| Runtime inventory | `evidence/phase0/runtime-inventory.md` |
| Summary | `evidence/phase0/phase0-summary.md` |

### 2.2 Analyze

- Compare against expected state
- Identify discrepancies
- Document known issues
- Classify findings by severity and impact

### 2.3 Report

- Store all findings under `evidence/phase0/`
- Produce `phase0-summary.md` with classification

---

## 3. Execution

### Step 1: Prepare Collection Environment

```powershell
# Ensure execution policy allows script collection
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force

# Verify write access to evidence/phase0/
Test-Path "evidence/phase0/"
```

### Step 2: Collect Runtime Evidence

```powershell
# Process inventory
Get-Process | Where-Object { $_.ProcessName -match "librarian|llama|rust|python" } | Format-Table -AutoSize

# Network listeners
Get-NetTCPConnection | Where-Object { $_.State -eq "Listen" } | Select-Object LocalAddress, LocalPort, OwningProcess

# Service status
Get-Service | Where-Object { $_.Name -match "librarian|llama|rust" } | Format-Table -AutoSize
```

### Step 3: Collect Build Environment Evidence

```powershell
# Rust toolchain
rustc --version
cargo --version

# Platform version
[System.Environment]::OSVersion.Version
```

### Step 4: Collect Hardware Evidence

```powershell
# GPU information
Get-WmiObject Win32_VideoController | Format-Table -AutoSize

# System information
Get-ComputerInfo | Select-Object CsName, CsTotalPhysicalMemory, WindowsVersion
```

### Step 5: Collect Database Evidence

```powershell
# Database file presence
Get-ChildItem -Path "G:\openwork\librarian-runtime-node\data\" -Filter "*.db" -Recurse

# Database schema (requires sqlite3)
sqlite3 "path\to\database.db" ".schema"
```

### Step 6: Collect Configuration Evidence

```powershell
# Runtime config files
Get-ChildItem -Path "G:\openwork\librarian-runtime-node\config\" -Recurse

# MCP config files
Get-ChildItem -Path "G:\openwork\librarian-runtime-node\mcp\" -Recurse
```

### Step 7: Collect Log Evidence

```powershell
# Startup logs
Get-Content -Path "logs\*.log" -Tail 100

# Error logs
Get-Content -Path "logs\*.err" -Tail 100
```

---

## 4. Deliverables

| Deliverable | Location |
|-------------|----------|
| Runtime inventory | `evidence/phase0/runtime-inventory.md` |
| Hardware documentation | `evidence/phase0/hardware.md` |
| GPU documentation | `evidence/phase0/gpu.md` |
| Installed software | `evidence/phase0/installed-software.md` |
| Rust version | `evidence/phase0/rust-version.md` |
| llama.cpp version | `evidence/phase0/llama-version.md` |
| MCP status | `evidence/phase0/mcp-status.md` |
| Process list | `evidence/phase0/process-list.md` |
| Network listeners | `evidence/phase0/network-listeners.md` |
| Runtime configuration | `evidence/phase0/runtime-config.md` |
| Database status | `evidence/phase0/database-status.md` |
| Registry status | `evidence/phase0/registry-status.md` |
| Dependency list | `evidence/phase0/dependency-list.md` |
| Startup log | `evidence/phase0/startup-log.md` |
| Error log | `evidence/phase0/error-log.md` |
| Known issues | `evidence/phase0/known-issues.md` |
| Phase 0 summary | `evidence/phase0/phase0-summary.md` |

---

## 5. Classification System

Findings are classified by:

### Severity

| Level | Description |
|-------|-------------|
| Critical | System cannot operate, data loss risk |
| High | Significant degradation, security concern |
| Medium | Operational impact, non-critical |
| Low | Minor issue, cosmetic |
| Informational | No impact, noteworthy |

### Impact

| Level | Description |
|-------|-------------|
| Security | Affects confidentiality, integrity, availability |
| Performance | Affects throughput, latency, resource usage |
| Reliability | Affects uptime, crash recovery, data consistency |
| Maintainability | Affects debugging, upgrading, migrating |
| Governance | Affects evidence, receipts, authority boundaries |

---

## 6. Exit Criteria

Phase 0 is complete when:

- [ ] Runtime process inventory documented
- [ ] MCP connectivity and configuration documented
- [ ] Network listeners documented
- [ ] Rust toolchain and dependency versions documented
- [ ] GPU and hardware inventory documented
- [ ] Database presence, schema versions, migration state documented
- [ ] Registry reconciliation status documented
- [ ] Startup and error logs documented
- [ ] Known failures and reproducible symptoms documented
- [ ] Configuration reviewed
- [ ] Dependency reviewed
- [ ] Findings classified by severity and impact
- [ ] Final phase0-summary.md produced

---

## 7. Restrictions

**Observation only.**

- No corrective changes
- No configuration changes
- No optimization
- No security modifications
- No database modifications
- No code modifications
- No dependency upgrades

---

## 8. References

- ADR-PLATFORM-001 — Core / Node Authority Architecture
- ADR-PLATFORM-002 — Platform Lifecycle
- ARCHITECTURAL-BOUNDARY-MAP — Code organization
- WINDOWS-RUNTIME-NODE-PLANNING-SPRINT.md — Planning sprint definition
