# Windows PC Sprint Roadmap

**Status:** Active
**Scope:** Windows Runtime Node, portable Router, and future Windows Librarian client/app lane
**Current runtime-node baseline:** `e7cfe33`
**Last sealed sprint:** WIN-RUNTIME-QUALIFICATION-1
**Previous roadmap version:** Sealed at WINDOWS-PC-PLAN-UPDATE-1 (REDUCED-OFFLOAD-FIT-1 baseline)

---

## Purpose

This roadmap defines the Windows PC work lane.

The Windows PC is not merely a test machine. It is a parallel execution lane for proving local runtime infrastructure, Router portability, and eventually a Windows version of The Librarian.

The work is divided into three layers:

| Layer | Focus |
|-------|-------|
| **Layer 1 — Runtime Node Reliability** | Make the Windows PC a dependable local advisory compute limb |
| **Layer 2 — Portable Router / Native Daemon** | Prevent Router behavior from becoming Windows-shaped |
| **Layer 3 — Windows Librarian Client/App** | Start a governed Owner-facing app on Windows |

---

## Architectural Distinctions

These must remain explicit in all planning and execution:

| Component | Role |
|-----------|------|
| **Runtime Node** | Local advisory compute limb. Not The Librarian. |
| **Router** | Portable runtime control contract (Python ref → Rust native) |
| **Windows Librarian** | Governed Owner-facing app/client on Windows |
| **Main Librarian core** | Custody, authority, receipts, validation, approval model |

> **Operating Principle:** The Windows Runtime Node is not The Librarian. It is an advisory compute limb. A Windows version of The Librarian must preserve custody, receipts, approval, validation, and Owner authority.

---

## Runtime Proof Chain (Established)

The following three-link proof chain is now complete and sealed across both repositories:

| Link | Status | Evidence |
|------|--------|----------|
| **1. Source HEAD proof** | ✅ Complete | Runtime-node HEAD `e7cfe33`, TheLibrarian-main HEAD `1e32002` |
| **2. Artifact hash proof** | ✅ Complete | SHA-256 `84EB797A715FDC4CDB634A85FDFEC5C81CB90F37FB700F6EA6C97576B9569DC9` — captured in v2 receipt |
| **3. Governed rebuild qualification proof** | ✅ Complete | Rebuild from HEAD `f82d301` produces matching hash; 38/38 qualification gate passed |

**Key artifact SHA-256:** `84EB797A715FDC4CDB634A85FDFEC5C81CB90F37FB700F6EA6C97576B9569DC9`

**Infrastructure built:**

| Component | Description | Path |
|-----------|-------------|------|
| v2 Receipt schema | Separates source HEAD from artifact proof; adds `listener_active`, `connectivity`, `router_binary_sha256` | `receipts/runtime-integration/schema-v2.json` |
| 48-check receipt verifier | Validates structure, artifact hash format, cleanup semantics, secret safety | `scripts/verify-receipt.ps1` |
| Integration proof v2 script | Automated lifecycle proof emitting v2 receipts | `scripts/run-integration-proof-v2.ps1` |
| Runtime qualification script | Governed rebuild + hash comparison | `scripts/run-runtime-qualification.ps1` |
| Qualification verifier | Gate: validates qualification record against receipt | `scripts/verify-runtime-qualification.ps1` |

---

## Current Completed State

The Windows Runtime Node has proven the core local runtime path through **12 completed sprints**, including the proof chain.

### Layer 0 — Foundation (completed)

| Sprint | Status | Result |
|--------|--------|--------|
| RUNTIME-REPO-INIT-1 | Complete | Runtime-node repo initialized |
| WIN-SERVICE-LIFECYCLE-1 | Complete | Windows service + router lifecycle proved |
| WIN-BACKEND-SERVICE-PROOF-1 | Complete | Service-started router launched backend and cleaned up with no orphan |
| WIN-ROUTER-HARDEN-1 | Complete | Router endpoints and failure cases verified |
| WIN-MODEL-CONTEXT-FIT-2 | Complete | RX 570 context fit tested |
| ROUTER-PORTABILITY-1 | Complete | Portable Router contract documented |
| REDUCED-OFFLOAD-FIT-1 | Complete | Reduced GPU offload for OOM profiles verified (ngl=80, ctx=4096) |
| **WINDOWS-PC-PLAN-UPDATE-1** | **Complete** | Roadmap doc, startup sequence, sprint index established |

### Layer 1 — Runtime Qualification and Receipts (completed)

| Sprint | Status | HEAD |
|--------|--------|------|
| WIN-RUNTIME-INTEGRATION-1 | Complete | TheLibrarian-main `5d2ecb3` |
| WIN-RUNTIME-RECEIPT-CLEANUP-1 | Complete | librarian-runtime-node `51c2e85` |
| WIN-RUNTIME-RECEIPTS-2 | Complete | librarian-runtime-node `f82d301` |
| WIN-RUNTIME-QUALIFICATION-1 | Complete | librarian-runtime-node `e7cfe33` |

**WIN-RUNTIME-INTEGRATION-1 validation caveat:** `swift test` was not run in the Windows environment because the Swift toolchain was unavailable. Record as manual implementation audit PASS, pending future Swift harness verification on a capable environment.

### Layer 1 Milestone — Proof Chain Complete

The proof chain is now sealed. All further Layer 1, 2, and 3 work builds on this baseline.

---

## Layer 1 — Runtime Node Reliability

Goal: make the Windows PC a dependable local advisory compute limb.

### 1. WIN-RUNTIME-OPERATIONS-1 (NEXT)

**Purpose:** Create a small operator toolkit for this Windows Runtime Node.

**Scope:** Add or document commands for:
- start service
- stop service
- restart service
- status
- logs
- profile list
- active backend
- clean orphan check
- port check
- service config dump

**Suggested scripts:**
- `scripts/runtime-status.ps1`
- `scripts/runtime-start.ps1`
- `scripts/runtime-stop.ps1`
- `scripts/runtime-logs.ps1`
- `scripts/runtime-clean-check.ps1`

**Acceptance:**
- scripts do not require unnecessary mutation
- scripts do not kill unrelated processes
- scripts preserve `LibrarianRunTimeNode` Manual startup policy
- scripts make support/debug easier

**Expected artifacts:**
- `docs/sprints/WIN-RUNTIME-OPERATIONS-1.md`
- optional scripts under `scripts/`

**Dependency:** None. Ready to execute from current baseline.

### 2. WIN-RUNTIME-PROFILES-CLEANUP-1

**Purpose:** Normalize `config/model-profiles.json` metadata to reflect verified reality.

**Current state:** The profile config already contains `verified_status`, `evidence_path`, `known_behavior`, `limitations`, and `test_cells` for all 5 profiles. The following metadata fields are **missing** and remain to be added:

| Missing Field | Purpose |
|---------------|---------|
| `verified_context` | Explicit context size verified at this ngl |
| `verified_ngl` | Explicit GPU layer count verified as stable |
| `stability` | Stability rating (e.g. `stable`, `conditional`, `unstable`) |
| `requires_reduced_offload` | Boolean: does this profile need ngl < 99 on RX 570 4GB? |
| `notes` | Free-text operational notes |

**Acceptance:**
- router still loads all profiles
- endpoint matrix still passes
- config reflects evidence from context-fit, reduced-offload, and integration receipts
- no model binaries committed

**Expected artifacts:**
- updated `config/model-profiles.json`
- `docs/sprints/WIN-RUNTIME-PROFILES-CLEANUP-1.md`

**Dependency:** After WIN-RUNTIME-OPERATIONS-1 (operator scripts make profile testing easier).

---

## Layer 2 — Portable Router / Native Daemon Evolution

Goal: prevent Router behavior from becoming Windows-shaped.

### 3. ROUTER-CONTRACT-TESTS-1

**Purpose:** Create shared conformance tests for any implementation of the portable Router contract.

**Applies to:**
- current Python reference router
- existing Rust router
- future macOS/Linux wrappers

**Scope:** Test contract cases from `docs/architecture/ROUTER-PORTABILITY-CONTRACT.md`.

**Minimum cases:**
- start/status
- profiles
- health
- select
- chat
- restart
- unknown profile refusal
- invalid JSON refusal
- missing field refusal
- advisory-only enforcement
- no orphan after restart/stop

**Expected artifacts:**
- `docs/sprints/ROUTER-CONTRACT-TESTS-1.md`
- `tests/router-contract/` or equivalent
- optional PowerShell runner for Windows

**Acceptance:**
- current Python router passes the contract tests
- failures are reported as contract failures, not implementation quirks
- proof chain receipts from integration runs serve as partial contract evidence

**Dependency:** After WIN-RUNTIME-OPERATIONS-1 (operator scripts provide reliable start/stop for test harness).

### 4. ROUTER-RUST-CORE-1

**Purpose:** Plan and/or begin a native Router core implementation in Rust against the portable contract.

> **Important rule:** This is not "rewrite the Windows router." It is "implement the portable Router contract in a native cross-platform daemon."

**Scope options:**

*Planning-only version:*
- Rust crate structure
- API server choice
- config loader
- process manager abstraction
- OS wrapper boundary
- contract-test mapping

*Implementation version:*
- minimal Rust daemon with `/backend/status` and `/backend/profiles`
- no backend spawning yet unless explicitly scoped

**Acceptance:**
- does not replace Python router prematurely
- maps directly to portability contract
- keeps OS-specific service code thin

**Expected artifacts:**
- `docs/sprints/ROUTER-RUST-CORE-1.md`
- optional `rust/` or root Rust project skeleton, if implementation is approved

**Dependency:** After ROUTER-CONTRACT-TESTS-1 (contract tests define the implementation target).

### 5. WIN-RUST-SERVICE-1

**Purpose:** Replace the current NSSM → PowerShell → Python service stack with a native Windows service wrapper only after Rust core is ready enough.

**Current proven path:**
```
Windows Service → NSSM → PowerShell launcher → Python router → llama-server
```

**Future target:**
```
Windows Service → native router daemon → llama-server
```

**Scope:**
- native service installation
- manual startup by default
- start/stop/restart proof
- backend child cleanup proof
- logs/receipts
- no orphan process after stop

**Acceptance:**
- feature parity with current NSSM proof
- no weaker lifecycle behavior
- rollback path preserved

**Expected artifacts:**
- `docs/sprints/WIN-RUST-SERVICE-1.md`
- native service code if approved

**Dependency:** After ROUTER-RUST-CORE-1 (needs a native router daemon to wrap).

---

## Layer 3 — Windows Librarian Client/App Lane

Goal: start a Windows version of The Librarian as a governed Owner-facing app/client.

> **Important boundary:** The Windows Runtime Node is not The Librarian. The Runtime Node is a local advisory compute limb. The Windows Librarian app must preserve: Owner authority, custody, receipts, validation, governed actions, approval flow, and non-canonical model output boundaries.

### 6. WIN-LIBRARIAN-APP-PLAN-1

**Purpose:** Decide the architecture for a Windows version of The Librarian.

**Architecture options:**
- native Windows app
- .NET / WPF / WinUI app
- Tauri shell
- Electron shell
- local web app / localhost UI
- thin client to existing Librarian core

**Required decisions:**
- app shell technology
- shared core strategy
- repo strategy
- local storage strategy
- runtime-node integration boundary
- Owner UI model
- receipt and validation display model
- Qualification/receipt consumption from runtime-node proof chain

**Acceptance:**
- no implementation until architecture is explicit
- runtime-node remains advisory-only
- custody/authority model remains intact

**Expected artifacts:**
- `docs/sprints/WIN-LIBRARIAN-APP-PLAN-1.md`
- `docs/architecture/WINDOWS-LIBRARIAN-APP.md`

**Dependency:** After Layer 1 operations and profile cleanup are complete.

### 7. WIN-LIBRARIAN-SHELL-1

**Purpose:** Create the first Windows app shell with no risky custody logic yet.

**Scope:**
- app launches
- shows placeholder navigation
- can show runtime-node status
- cannot mutate project files yet
- cannot approve or validate work yet

**Acceptance:**
- shell is stable
- runtime-node status is visible
- no authority-bearing actions implemented prematurely

### 8. WIN-LIBRARIAN-RUNTIME-INTEGRATION-1

**Purpose:** Connect Windows Librarian UI to the local runtime node as advisory compute only.

**Scope:**
- show available profiles
- select advisory runtime
- send prompt
- display advisory response
- show `[ADVISORY RUNTIME]` or structured advisory metadata
- no file writes
- no approval bypass

**Acceptance:**
- Owner can see runtime output
- runtime output is never canonical
- failure is non-fatal

### 9. WIN-LIBRARIAN-CUSTODY-UI-1

**Purpose:** Bring governed custody concepts into the Windows app UI.

**Scope:**
- project identity
- imported repo state
- receipts
- validation state
- Owner approval / rejection
- governed action packet display
- Runtime-node proof chain visualization (source HEAD, artifact hash, qualification status)

**Acceptance:**
- UI actions map to governed action meanings
- no raw "button event" without canonical action semantics
- Owner authority preserved

---

## Recommended Execution Order (Updated)

| Order | Sprint | Layer | Prerequisite |
|-------|--------|-------|-------------|
| 1 | REDUCED-OFFLOAD-FIT-1 | Layer 0 | ✅ Done |
| 2–5 | *(Proof Chain: INTEGRATION-1, RECEIPT-CLEANUP-1, RECEIPTS-2, QUALIFICATION-1)* | Layer 1 | ✅ Done |
| 6 | WINDOWS-PC-PLAN-UPDATE-1 | Mgmt | ✅ Done |
| **7** | **WINDOWS-PC-PLAN-UPDATE-2** | **Mgmt** | **← Current sprint** |
| **8** | **WIN-RUNTIME-OPERATIONS-1** | **Layer 1** | **← Next** |
| 9 | WIN-RUNTIME-PROFILES-CLEANUP-1 | Layer 1 | After operations |
| 10 | ROUTER-CONTRACT-TESTS-1 | Layer 2 | After operations |
| 11 | ROUTER-RUST-CORE-1 | Layer 2 | After contract tests |
| 12 | WIN-RUST-SERVICE-1 | Layer 2 | After Rust core |
| 13 | WIN-LIBRARIAN-APP-PLAN-1 | Layer 3 | After Layer 1 |
| 14 | WIN-LIBRARIAN-SHELL-1 | Layer 3 | After app plan |
| 15 | WIN-LIBRARIAN-RUNTIME-INTEGRATION-1 | Layer 3 | After shell |
| 16 | WIN-LIBRARIAN-CUSTODY-UI-1 | Layer 3 | After runtime integration |

---

## Receipt and Verification Reference

### v2 Receipt Schema
- **Location:** `receipts/runtime-integration/schema-v2.json`
- **Schema ID:** `win-runtime-receipt/v2`
- **Key improvements over v1:**
  - `cleanup.listener_active` — distinguishes LISTENING socket from TCP TIME_WAIT residue
  - `cleanup.connectivity` — TCP connect test result (`refused` / `listening`)
  - `artifact.router_binary_sha256` — artifact-level proof (separate from source HEAD)
  - `artifact.governed_path_match` — binary path vs governed/expected path

### Receipt Verifier
- **Script:** `scripts/verify-receipt.ps1`
- **Checks:** 48 total (schema, artifact hash format, cleanup semantics, secret safety, overall derivation)
- **Compatibility:** v1 and v2 receipts

### Runtime Qualification Gate
- **Scripts:** `scripts/run-runtime-qualification.ps1`, `scripts/verify-runtime-qualification.ps1`
- **Gate checks:** 38/38 passed on sealed qualification record
- **Rebuild hash:** Matches receipt artifact hash (`84EB797A...`)
- **Does not assume reproducible builds:** records match/mismatch honestly

### Anti-Loop Rules
Refer to `docs/operations/WINDOWS-AGENT-STARTUP-SEQUENCE.md` for the full anti-loop rule set. Key rules:
- Stop after two failed attempts at the same command/code path.
- Never edit evidence to force pass.
- Restore service state if runtime state becomes ambiguous.
- Do not broaden sprint scope after a timeout.

---

## Operating Principle

The PC lane should prove useful local capability without weakening the main Librarian authority model.

- **Runtime node work** proves compute.
- **Router work** proves portable runtime control.
- **Windows Librarian work** proves a governed Owner-facing app on Windows.
