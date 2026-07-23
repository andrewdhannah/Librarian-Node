# Windows PC Sprint Guardrail Needs

**Status:** Draft (guardrail category register — not an implementation spec)
**Date:** 2026-06-29
**Plan ref:** `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md`
**Baseline ref:** `docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md`
**Custody model ref:** `docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md`

---

## 1. Purpose

Define the guardrail and profile categories required across all remaining Windows PC readiness sprints. This document:

- Assigns each guardrail a stable category ID and definition
- Maps each guardrail to the custody sandbox model layer (Layer 1 — Mechanical, Layer 2 — Policy, Layer 3 — Authority)
- Records the Windows-specific implementation constraints for each category
- Notes the expected future mapping to the Mac/Librarian canonical guardrail-profile system
- Does **not** implement any guardrail — it is a catalog of requirements

---

## 2. Guardrail Categories

### G-001: Pre-mutation State Verification

| Field | Value |
|-------|-------|
| **Layer** | Layer 1 — Mechanical (Harness Tools) |
| **Purpose** | Verify environment state before any agent or human mutation. Gate on preconditions, do not repair. |
| **Windows constraints** | Must work in PowerShell 5.1; must be read-only; must report deterministic pass/fail; must check 11+ dimensions (HEAD, working tree, service, ports, orphans, disk, origin, files) |
| **Implementation status** | ✅ `scripts/harness/pre-mutation-check.ps1` exists (WIN-PACKET-VALIDATION-HOOK-1) |
| **Future canonical mapping** | Pre-mutation will be a canonical guardrail type. Windows adapts via PowerShell; canonical may use Swift or native Rust. |
| **Required by sprints** | S-01, S-10 |

### G-002: Post-mutation State Verification

| Field | Value |
|-------|-------|
| **Layer** | Layer 1 — Mechanical (Harness Tools) |
| **Purpose** | Verify environment state after a sprint mutation to ensure no unintended side effects. Detect orphan processes, port leaks, service state changes, working tree dirtiness. |
| **Windows constraints** | Must work in PowerShell 5.1; must compare against pre-mutation snapshot; must generate machine-readable output (JSON receipt) |
| **Implementation status** | ❌ Planned — WIN-HARNESS-POSTFLIGHT-1 |
| **Future canonical mapping** | Post-mutation will be a canonical guardrail type. Windows adapts via PowerShell. |
| **Required by sprints** | S-01, S-02, S-03 |

### G-003: Service-state Guard

| Field | Value |
|-------|-------|
| **Layer** | Layer 1 — Mechanical (Harness Tools) / Layer 3 — Authority |
| **Purpose** | Guard against unauthorized service mutation. Query is always permitted. Mutating (start, stop, change start-type) requires Layer 3 (Owner) approval. |
| **Windows constraints** | `Get-Service` available in PS 5.1 (query). Service mutation requires admin elevation (`Start-Service`, `Stop-Service`, `Set-Service`). NSSM stack (NSSM → PowerShell → Python → llama-server) requires understanding of the multi-process chain. |
| **Implementation status** | ✅ Query implemented in pre-mutation-check.ps1 (check 5). Mutation gating is procedural (runbook/SOP). |
| **Future canonical mapping** | Service-state guardrail will be a canonical category. Windows native service model differs from macOS launchd — canonical spec must allow platform-specific service management. |
| **Required by sprints** | S-02, S-10, S-16 |

### G-004: Orphan-process Guard

| Field | Value |
|-------|-------|
| **Layer** | Layer 1 — Mechanical (Harness Tools) |
| **Purpose** | Detect orphan runtime processes (`llama-server.exe`, `rust-router.exe`, `python.exe` running router code). Orphans indicate unclean lifecycle — must be detected before next mutation. |
| **Windows constraints** | `Get-CimInstance Win32_Process` used in PS 5.1. Must distinguish orphan `python.exe` (running router.py) from non-orphan Python processes. Kill only within documented sprint scope. |
| **Implementation status** | ✅ Implemented in pre-mutation-check.ps1 (check 8) |
| **Future canonical mapping** | Orphan detection will be a canonical guardrail. Windows process enumeration differs from macOS `ps`/`pgrep`. |
| **Required by sprints** | S-01, S-10, S-16 |

### G-005: Git-state Guard

| Field | Value |
|-------|-------|
| **Layer** | Layer 1 — Mechanical (Harness Tools) |
| **Purpose** | Enforce correct git state before and after mutations. Checks: HEAD matches expected, working tree clean, branch is main, origin/main in sync. |
| **Windows constraints** | `git` available in PS 5.1 as external command. Must handle CRLF warnings (LF→CRLF replacement notices are non-fatal). Must handle detached HEAD if sprint requires it. |
| **Implementation status** | ✅ Partially implemented in pre-mutation-check.ps1 (checks 2, 3, 4, 10). Post-mutation git-state verification planned for S-02. |
| **Future canonical mapping** | Git-state guardrail will be a canonical category. Same `git` tool on all platforms — implementation is inherently portable. |
| **Required by sprints** | S-02 |

### G-006: Contract-test Guard

| Field | Value |
|-------|-------|
| **Layer** | Layer 1 — Mechanical (Harness Tools) |
| **Purpose** | Run portable Router contract tests against any router implementation. Guard against contract violations during router evolution. |
| **Windows constraints** | Test runner must work in PS 5.1. Can invoke Python test scripts as subprocesses. Must support both Python reference router and Rust native router as targets. |
| **Implementation status** | ❌ Planned — WIN-HARNESS-CONTRACT-RUNNER-1 |
| **Future canonical mapping** | Contract tests are inherently cross-platform. The test suite itself is the canonical guardrail — PowerShell runner is Windows-local. |
| **Required by sprints** | S-03, S-14, S-15 |

### G-007: Baseline-drift Guard

| Field | Value |
|-------|-------|
| **Layer** | Layer 1 — Mechanical (Harness Tools) |
| **Purpose** | Compare current environment state against the sealed baseline. Detect and report drift in tool versions, PATH, environment variables, disk state, service state, port assignments. |
| **Windows constraints** | Must parse the baseline Markdown document (or a structured extract). Must handle PS 5.1 CIM/WMI for system state queries. Baseline was created on a different HEAD (`08a8602`) — comparison must account for evolved state. |
| **Implementation status** | ❌ Planned — WIN-HARNESS-BASELINE-DIFF-1 |
| **Future canonical mapping** | Baseline drift will be a canonical guardrail. Baseline format should be canonical (machine-readable JSON or similar) shared across Windows and Mac. |
| **Required by sprints** | S-04 |

### G-008: Tool-version Guard

| Field | Value |
|-------|-------|
| **Layer** | Layer 1 — Mechanical (Harness Tools) |
| **Purpose** | Verify that required tool versions (git, python, node, rust, cargo) are present and within expected ranges. Log tool version drift as informational. |
| **Windows constraints** | Tool installation paths may differ from macOS/Linux. `rustup`, `nvm-windows`, `pyenv-win` have different conventions. MSVC resolution via VS Installer auto-detection is Windows-specific. |
| **Implementation status** | ❌ Not yet implemented. Partial coverage in baseline report (section 24 comparison). |
| **Future canonical mapping** | Tool-version guardrail will be a canonical category. Version ranges should be canonical; installation path detection is platform-specific. |
| **Required by sprints** | S-04, S-07, S-08, S-15 |

### G-009: Sprint-scope Guard

| Field | Value |
|-------|-------|
| **Layer** | Layer 2 — Policy (Sprint Contract) |
| **Purpose** | Enforce that a sprint's mutations stay within its declared scope. Prevent accidental or unauthorized changes outside the sprint's allowed mutation paths. |
| **Windows constraints** | Guardrail must compare changed files against sprint doc's "Allowed mutation scope" list. Path matching must account for Windows case-insensitive filesystem. |
| **Implementation status** | ❌ Planned — WIN-SPRINT-LEDGER-1, WIN-HARNESS-LEDGER-1 |
| **Future canonical mapping** | Sprint-scope guardrail is inherently a policy-layer concern. Canonical system will define a scope contract schema; Windows implements a scope verifier. |
| **Required by sprints** | S-05, S-13, S-20 |

### G-010: Model-profile Guard

| Field | Value |
|-------|-------|
| **Layer** | Layer 1 — Mechanical (Harness Tools) / Layer 2 — Policy |
| **Purpose** | Guard against invalid model profile selection. Verify that profile config fields are complete and accurate. Prevent selection of a model that exceeds available GPU memory or system RAM. |
| **Windows constraints** | Model files on G: drive, not C:. GPU is RX 570 4 GB (Vulkan). Context size and GPU layers must be verified per profile. Reduced-offload profiles (ngl < 99) are documented for this GPU. |
| **Implementation status** | ⚠️ Partial. Profiles exist in `config/model-profiles.json` with verified fields. Missing metadata fields (`verified_context`, `verified_ngl`, `stability`, `requires_reduced_offload`, `notes`) remain to be added in S-11. |
| **Future canonical mapping** | Model-profile guard is a canonical concern. Windows GPU (AMD RX 570 4 GB) differs from Mac GPU (Apple Silicon unified memory). Profile schema must be canonical; GPU-specific constraints are platform-local. |
| **Required by sprints** | S-11, S-19 |

### G-011: Environment-health Guard

| Field | Value |
|-------|-------|
| **Layer** | Layer 1 — Mechanical (Harness Tools) |
| **Purpose** | Monitor overall environment health: disk free space, Windows version/EOS status, OS update status, available memory. Flag conditions that could impair sprint work but do not block it. |
| **Windows constraints** | C: drive threshold (default 5 GB min). Windows 10 22H2 past EOS (informational). Must use `Get-CimInstance Win32_LogicalDisk` for disk queries. Temp file accumulation on C: is a Windows-specific concern. |
| **Implementation status** | ✅ Partially implemented in pre-mutation-check.ps1 (check 9 — C: drive space). Wider health monitoring planned for S-04 (baseline diff) and S-10 (operator scripts). |
| **Future canonical mapping** | Environment-health guardrail will be a canonical category. Disk space threshold is platform-agnostic; Windows-specific dimensions (page file, temp space, Windows Update status) are local. |
| **Required by sprints** | S-04, S-06, S-08, S-09, S-10, S-11 |

### G-012: Receipt-integrity Guard

| Field | Value |
|-------|-------|
| **Layer** | Layer 1 — Mechanical (Harness Tools) |
| **Purpose** | Verify that sprint receipts are structurally valid, complete, and consistent with the sprint contract. Prevent forged or incomplete receipts from being accepted as evidence. |
| **Windows constraints** | Must work with existing v2 receipt schema (`receipts/runtime-integration/schema-v2.json`, 48 checks via `verify-receipt.ps1`). Receipts must not contain secrets. JSON parsing in PS 5.1 via `ConvertFrom-Json`. |
| **Implementation status** | ✅ 48-check receipt verifier exists (`scripts/verify-receipt.ps1`). Sprint-level receipt templates planned for S-02. |
| **Future canonical mapping** | Receipt-integrity is a canonical concern. Receipt schema should be canonical (shared between Windows and Mac). The verifier implementation is platform-local. |
| **Required by sprints** | S-02, S-05, S-12, S-13, S-20 |

### G-013: App-boundary Guard

| Field | Value |
|-------|-------|
| **Layer** | Layer 2 — Policy / Layer 3 — Authority |
| **Purpose** | Define and enforce the boundary between the Windows Librarian app and the runtime node. The app must be advisory-only, must not weaken custody, and must not perform authority-bearing actions without Owner approval. |
| **Windows constraints** | App tech choice (Tauri, Electron, Python web) determines the enforcement mechanism. Tauri offers stronger OS-boundary guarantees than Electron. Python web offers weakest but fastest path. |
| **Implementation status** | ❌ Conceptual — architecture defined in WIN-LIBRARIAN-HOST-OPTIONS.md. Implementation deferred to Phase 1. |
| **Future canonical mapping** | App-boundary guardrail is the most Windows-specific concern. The Mac Librarian app already exists (Swift/macOS). The Windows version must match the canonical custody model while using Windows-native technologies. |
| **Required by sprints** | S-17, S-18, S-19, S-20 |

---

## 3. Guardrail Profile Categories

In addition to the operational guardrails above, the following profile categories are needed to classify guardrail behavior:

### P-001: Hard Guard (Must-Pass)

| Field | Value |
|-------|-------|
| **Behavior** | If this guardrail fails, the sprint MUST NOT proceed. Exit 1, block mutation. |
| **Examples** | G-001 (Pre-mutation verification) — if HEAD is wrong or working tree is dirty, agent must not mutate. G-005 (Git-state guard) — working tree must be clean before mutation. |
| **Windows treatment** | Hard guards must exit 1 with clear failure message. Must not auto-repair. |
| **Future canonical mapping** | Hard guard is a canonical classification. |

### P-002: Soft Guard (Warn)

| Field | Value |
|-------|-------|
| **Behavior** | If this guardrail fails, emit a WARNING and document the issue, but allow the sprint to proceed. |
| **Examples** | G-011 (Environment-health guard) — C: drive below 10 GB but above 5 GB threshold: warn, do not block. G-008 (Tool-version guard) — minor version drift: log, do not block. |
| **Windows treatment** | Soft guards must emit clear warning message and continue with exit 0. Warnings should be captured in sprint receipt. |
| **Future canonical mapping** | Soft guard is a canonical classification. |

### P-003: Logging Guard (Informational)

| Field | Value |
|-------|-------|
| **Behavior** | Record the state of a dimension without pass/fail judgment. Used for audit trail and drift detection. |
| **Examples** | G-008 (Tool-version guard) — tool version recording. G-011 (Environment-health guard) — Windows build number, uptime. |
| **Windows treatment** | Logging guards should emit structured key=value pairs or JSON for machine consumption. |
| **Future canonical mapping** | Informational guard is a canonical classification. |

### P-004: Authority Guard (Owner-Approval Required)

| Field | Value |
|-------|-------|
| **Behavior** | This guardrail gates actions that require human Owner approval. No agent can bypass. |
| **Examples** | G-003 (Service-state guard — mutation branch). G-013 (App-boundary guard — any crossing). Service start, stop, install, firewall change, model activation. |
| **Windows treatment** | Authority guards must produce a clear "requires Owner approval" message. Must document what approval looks like (verbal, written, sprint doc signature). |
| **Future canonical mapping** | Authority guard is a canonical classification — this is the core of The Librarian's custody model. |

---

## 4. Guardrail-to-Sprint Dependency Matrix

| Guardrail | S-01 | S-02 | S-03 | S-04 | S-05 | S-06 | S-07 | S-08 | S-09 | S-10 | S-11 | S-12 | S-13 | S-14 | S-15 | S-16 | S-17 | S-18 | S-19 | S-20 |
|-----------|------|------|------|------|------|------|------|------|------|------|------|------|------|------|------|------|------|------|------|------|
| G-001 | R | — | — | — | — | — | — | — | — | R | — | — | — | — | — | — | — | — | — | — |
| G-002 | C | C | R | — | — | — | — | — | — | — | — | — | — | — | — | — | — | — | — | — |
| G-003 | — | R | — | — | — | — | — | — | — | C | — | — | — | — | — | C | — | — | — | — |
| G-004 | C | — | — | — | — | — | — | — | — | C | — | — | — | — | — | C | — | — | — | — |
| G-005 | — | C | — | — | — | — | — | — | — | — | — | — | — | — | — | R | — | — | — | — |
| G-006 | — | — | C | — | — | — | — | — | — | — | — | — | — | C | C | — | — | — | — | — |
| G-007 | — | — | — | C | — | — | — | — | — | — | — | — | — | — | — | — | — | — | — | — |
| G-008 | — | — | — | C | — | — | C | C | — | — | — | — | — | — | C | — | — | — | — | — |
| G-009 | — | — | — | — | C | — | — | — | — | — | — | — | C | — | — | — | — | — | — | C |
| G-010 | — | — | — | — | — | — | — | — | — | — | C | — | — | — | — | — | — | — | C | — |
| G-011 | — | — | — | C | — | C | — | C | C | C | C | — | — | — | — | — | — | — | — | — |
| G-012 | — | C | — | — | C | — | — | — | — | — | — | C | C | — | — | — | — | — | — | C |
| G-013 | — | — | — | — | — | — | — | — | — | — | — | — | — | — | — | — | C | C | C | C |

**Legend:** R = Requires (pre-existing dependency), C = Creates/Implements

---

## 5. Non-Canonical Status Declaration

This document and its guardrail categories are **Windows-local** and **non-canonical**. They serve as:

1. A **requirements catalog** for Windows PC sprint planning
2. A **constraint record** for the future Mac/Librarian canonical guardrail-profile system designers
3. A **migration baseline** — when the canonical system arrives, this document's categories will be mapped to canonical equivalents

**The canonical guardrail-profile system is not created by this sprint. It remains a future Mac/Librarian deliverable.**
