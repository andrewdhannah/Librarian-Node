# Windows Agent Harness Plan

**Status:** Draft
**Date:** 2026-06-29
**Baseline reference:** `docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md`
**Environment host:** DESKTOP-ISNJ51B — MSI MS-7751 (i5-3570K, 24 GB RAM, RX 570 4 GB)
**Repo:** `G:\OpenWork\librarian-runtime-node`

---

## 1. Purpose

Define the architecture, principles, and component boundaries for a **governed Windows agent harness** that enables safe, reproducible, and custody-preserving agent work on the Windows PC lane.

The harness is not an application. It is a **toolkit and discipline layer** that sits between an agent (or human operator) and the Windows Runtime Node, ensuring every action is:

- **Verifiable** — actions leave receipts, not just side effects
- **Bounded** — no action exceeds its defined scope
- **Reversible** — custody boundaries prevent unrecoverable state changes
- **Auditable** — every sprint produces committed evidence

---

## 2. Why a Harness (Not Just Scripts)

The existing repo already has 50+ scripts (`scripts/`, `scripts/tests/`, `scripts/operations/`, `scripts/measurements/`), a runbook, a startup sequence, anti-loop rules, and multiple receipt schemas. What it lacks is a **unifying model** that connects these pieces:

| Problem | Harness Solution |
|---------|-----------------|
| Scripts scattered across directories with no consistent entry point | **Agent entry point** — one command to verify state before acting |
| No automated pre-flight check before mutation | **Pre-flight hooks** — verify state against expected baseline |
| Receipts are manually created per sprint | **Receipt templates** — standardized per action type |
| Orphan/port/service state must be manually checked | **State assertions** — automated before and after every action |
| Environment drift goes undetected | **Baseline comparison** — diff current state against sealed baseline |
| No cross-sprint custody chain | **Custody ledger** — each sprint records its start/end state |

---

## 3. Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    Windows Agent Harness                       │
│                                                               │
│  ┌─────────────────┐  ┌────────────────┐  ┌───────────────┐  │
│  │  Pre-flight      │  │  Action         │  │  Post-flight  │  │
│  │  Verification    │  │  Execution      │  │  Verification  │  │
│  │                  │  │  (Agent or      │  │               │  │
│  │  • HEAD check    │  │   Human Op)     │  │  • Port check  │  │
│  │  • Status check  │  │                 │  │  • Orphan chk  │  │
│  │  • Service check │  │  Bounded by:    │  │  • Service chk │  │
│  │  • Port check    │  │  • Sprint scope │  │  • Receipt gen │  │
│  │  • Orphan check  │  │  • Doc contract │  │  • State log   │  │
│  │  • Baseline cmp  │  │  • Proxy dirs   │  │               │  │
│  └────────┬─────────┘  └────────┬───────┘  └───────┬───────┘  │
│           │                     │                   │          │
│           └─────────────────────┼───────────────────┘          │
│                                 │                              │
│                    ┌────────────▼────────────┐                 │
│                    │    Custody Sandbox       │                 │
│                    │    (Boundary Layer)      │                 │
│                    │                          │                 │
│                    │  • Allowed paths         │                 │
│                    │  • Forbidden targets     │                 │
│                    │  • Proxy directories     │                 │
│                    │  • No-commit rules       │                 │
│                    │  • No-service rules      │                 │
│                    └──────────────────────────┘                 │
│                                                               │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │  Persistence Layer                                       │  │
│  │  • Sprint docs (sprints/)  • Planning docs (planning/)  │  │
│  │  • Receipts (receipts/)    • Baseline state (baseline/) │  │
│  │  • Operations docs (operations/)                        │  │
│  └─────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### 3.1 Pre-flight Verification

Every harness session (agent or human) must pass pre-flight before any mutation:

```powershell
# Conceptual harness entry point
.\harness\check.ps1 --baseline docs/planning/BASELINE.md
```

Pre-flight checks:
- HEAD matches expected sprint start
- Working tree clean (or classified dirty files)
- Service `LibrarianRunTimeNode` in expected state (Stopped/Manual)
- Ports 9120–9130 free
- Zero orphan `llama-server`, `rust-router`, `python` (router) processes
- Environment baseline within acceptable drift thresholds

### 3.2 Action Execution (Bounded)

Agent or human works within explicit sprint boundaries:
- Files to change are listed in the sprint doc
- Files to NOT touch are listed (no-go list)
- No service mutation without Owner approval
- No model workload outside defined profiles
- No network boundary weakening

### 3.3 Post-flight Verification

Every sprint must close with:
- Port check (all ports free)
- Orphan check (zero orphan processes)
- Service state check (Stopped/Manual preserved)
- Working tree check (clean or documented exceptions)
- Receipt generation (machine-readable evidence)
- State diff against baseline (what changed)

---

## 4. Component Inventory

| Component | Status | Location |
|-----------|--------|----------|
| Environment baseline | ✅ Done (WIN-AGENT-HARNESS-ENV-BASELINE-1) | `docs/planning/*-BASELINE.md` |
| Agent startup sequence | ✅ Existing | `docs/operations/WINDOWS-AGENT-STARTUP-SEQUENCE.md` |
| Anti-loop rules | ✅ Existing | Integrated into startup sequence |
| Operator runbook | ✅ Existing | `docs/operations/WIN-RUNTIME-OPERATOR-RUNBOOK.md` |
| Sprint doc convention | ✅ Existing | `docs/sprints/*.md` |
| Receipt convention | ✅ Existing | `docs/receipts/*.md` |
| Custody sandbox model | ❌ Missing | This sprint (next doc) |
| Harness parity roadmap | ❌ Missing | This sprint (next doc) |
| Windows Librarian host options | ❌ Missing | This sprint (next doc) |
| Sprint sequence | ❌ Missing | This sprint (next doc) |
| Unified pre-flight command | ❌ Future | `harness/check.ps1` |
| Post-flight receipt generator | ❌ Future | `harness/receipt.ps1` |
| Baseline comparison tool | ❌ Future | `harness/diff-baseline.ps1` |
| Custody ledger | ❌ Future | `harness/ledger.ps1` |

**Color key:** Existing components can be used as-is or adapted. Missing components are future work.

---

## 5. Operational Principles

### 5.1 Receipts Before Results
Every harness action emits a receipt before the action is considered complete. A receipt records:
- Action type and scope
- Starting state (HEAD, service, ports, orphans)
- Ending state
- Any anomalies found
- Whether the action passed its acceptance gates

### 5.2 Failures Are Evidence
A harness test that fails is still valuable evidence. Receipts may report `partial` or `fail` — these are not edited to force pass. An honest failure record is more valuable than a forged pass.

### 5.3 No Implicit Authority
The harness does not grant the agent authority. All authority remains with:
- The Owner (human)
- The custody sandbox (boundary layer)
- The receipt verifier (validation gate)

The agent can only act within the sprint doc's explicit scope. The harness enforces this by refusing actions outside the doc-defined boundary.

### 5.4 Monotonic Baseline
The environment baseline (`WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md`) is a sealed reference point. It should be updated only through a dedicated baseline-update sprint, not drift-snapped on every session. The harness detects drift relative to this monotonic baseline.

---

## 6. Relationship to Other Work Lanes

| Lane | Relationship |
|------|-------------|
| **Runtime Node** (Layer 1) | Harness wraps runtime node operations with pre/post-flight checks |
| **Portable Router** (Layer 2) | Harness runs contract tests against any router implementation |
| **Windows Librarian** (Layer 3) | Harness provides the custody boundary that the librarian app will eventually own |
| **Mac-side Harness** (companion) | Harness targets eventual parity with Mac-side verification tooling |

---

## 7. Gating Risks from Baseline

The following findings from the baseline inventory are constraints on harness design:

| Finding | Impact on Harness |
|---------|------------------|
| **F-001: C: drive critically low (10.2 GB free)** | See addendum below. Original baseline severity was HIGH based on assumption that model files and build artifacts consumed C: space. |
| **F-002: dotnet SDK not found** | Any harness component written in C#/.NET cannot be developed or run on this machine. Harness implementation should avoid .NET dependency unless it's explicitly added. |
| **F-003: MSVC compiler not in PATH** | Rust builds via cargo may still work (VS Installer auto-detection). Harness should test and document the exact build resolution path. |
| **F-004: SESSION-HANDOFF.md stale** | Now corrected. Harness should treat SESSION-HANDOFF.md as a living document updated at sprint closeout. |
| **F-005: No FEATURE-STATUS.md or sprint-ledger.json** | Harness may introduce a light sprint-ledger.json as a machine-readable index of completed/current/next sprints. |
| **F-006: 5 planning docs missing** | This sprint creates them. Future harness work fills remaining gaps. |
| **F-007: Windows 10 22H2 past EOS** | Harness should document Windows version as an operational risk but not block Phase 0 work. A future WIN-WINDOWS-UPGRADE-EVAL-1 may address this if the machine role expands. |
| **F-008: Multiple Ollama/LM Studio paths in PATH** | PATH hygiene is a minor concern. Harness pre-flight should note PATH entries that conflict with the runtime-node's explicit binary selection. |

---

### §7 Addendum — F-001 Interpretation Revision

**Effective from this sprint (WIN-AGENT-HARNESS-PLAN-1) forward.**

**Original baseline severity (WIN-AGENT-HARNESS-ENV-BASELINE-1):** HIGH
**Revised planning severity:** MEDIUM

**Reason for revision:** The baseline report recorded C: drive with 10.2 GB free (9.2%) and classified this as HIGH severity. However, subsequent investigation revealed that the following critical assets reside on **G:** drive (132 GB free), not C::

| Asset | Actual Location | Actual Load |
|-------|----------------|-------------|
| Model GGUFs | `G:\llama.cpp\models` | G: drive (132 GB free) |
| Repo root + git history | `G:\OpenWork\librarian-runtime-node` | G: drive |
| Rust build artifacts (`target/`) | `rust-router/target/` (gitignored) | G: drive |
| Python site-packages | `C:\Python314\Lib` | C: (small, ~100 MB typical) |
| Cargo registry cache | `C:\Users\andre\.cargo\registry` | C: (~500 MB typical) |

Since model files, repo data, and build artifacts all live on G:, C: drive capacity does not block:
- Model workload sprints (inference, GGUF selection, profile testing)
- Rust build operations (`cargo build --release`)
- Python virtual environment creation
- Git operations (clone, fetch, history growth)
- Harness implementation sprints (scripts, tests, receipt tools)

**Residual C: drive concerns (MEDIUM severity):**
- Windows Update: 10 GB may be insufficient for feature updates
- Temp file accumulation: `%TEMP%` and `%TMP%` point to C:
- New SDK installs (.NET, Windows App SDK) would consume C: space
- User profile caches (npm, cargo, rustup) reside on C:

**Harness treatment:** Pre-flight check should measure and report C: free space, warn if it drops below 5 GB, but not block harness implementation, script development, or docs-only sprints at current levels.

---

## 8. Next Steps

After this planning sprint, the recommended execution sequence is:

| Sprint | Purpose | Depends On |
|--------|---------|------------|
| WIN-PACKET-VALIDATION-HOOK-1 | Implement first harness pre-flight verification hook | This plan |
| WIN-HARNESS-POSTFLIGHT-1 | Build post-flight state verification | Packet validation hook |
| WIN-HARNESS-RECEIPT-TEMPLATE-1 | Standardized sprint receipt generation | Post-flight |
| WIN-SPRINT-LEDGER-1 | Create sprint-ledger.json convention | This plan |

Disk-space triage (WIN-DISK-SPACE-RISK-TRIAGE-1) is deferred to a parallel maintenance track — it does not block harness implementation.
