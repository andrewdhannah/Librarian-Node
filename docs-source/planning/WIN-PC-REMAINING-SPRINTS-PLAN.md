# Windows PC Remaining Sprints — Full Plan

**Status:** Draft (planning map — not a sprint contract)
**Date:** 2026-06-29
**Baseline ref:** `docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md`
**Plan refs:**
- `docs/planning/WIN-AGENT-HARNESS-PLAN.md`
- `docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md`
- `docs/planning/WIN-HARNESS-PARITY-ROADMAP.md`
- `docs/planning/WIN-LIBRARIAN-HOST-OPTIONS.md`
- `docs/planning/WIN-SPRINT-SEQUENCE.md`
- `docs/planning/WIN-PC-SPRINT-GUARDRAIL-NEEDS.md`
**Roadmap ref:** `docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md`
**Sprint ref:** `docs/sprints/WIN-PC-REMAINING-SPRINTS-PLAN-1.md`

---

## 1. Purpose

Synthesize all existing planning documents into a single coherent map of the remaining Windows PC readiness sprints. This document:

- Lists every future sprint with its purpose, phase, dependencies, scope boundaries, guardrail categories, preflight checks, acceptance gates, closeout requirements, and recommended next sprint
- Identifies all guardrail/profile categories needed across the remaining sprints (detailed in companion document `WIN-PC-SPRINT-GUARDRAIL-NEEDS.md`)
- Records Windows-specific constraints that differentiate the Windows lane from the future Mac/Librarian canonical guardrail-profile system
- Does **not** create the canonical guardrail-profile system — only records Windows-local sprint needs and expected guardrail categories

---

## 2. Sealed Baseline

All prior sprints are sealed. No sprint in this plan reopens or modifies sealed work.

| Sprint | HEAD | Status |
|--------|------|--------|
| *(22 prior sprints through WIN-RUNTIME-QUALIFICATION-1)* | `e7cfe33` | ✅ Sealed |
| WIN-RUNTIME-CONTROLLED-ACTIVATION-1 | `08a8602` | ✅ Sealed |
| WIN-AGENT-HARNESS-ENV-BASELINE-1 | `2895584` | ✅ Sealed |
| WIN-ORIGIN-AHEAD-RECONCILE-1 | `06768f3` | ✅ Sealed |
| WIN-AGENT-HARNESS-PLAN-1 | `7cc7d10` | ✅ Sealed |
| WIN-PACKET-VALIDATION-HOOK-1 | `7cc7d10` | ✅ Ready for seal (receipt PASS) |

**Pre-mutation hook exists at:** `scripts/harness/pre-mutation-check.ps1` — 11 checks, exit 0/1.

---

## 3. Windows-Specific Constraints

These constraints differentiate the Windows lane from the future Mac/Librarian canonical system. They must be recorded in every sprint that touches guardrail/profile design.

| # | Constraint | Source | Implication |
|---|------------|--------|-------------|
| C-001 | PowerShell 5.1 only (no pwsh) | Baseline §9 | All guardrail scripts must be PS 5.1 compatible |
| C-002 | No Swift toolchain | Baseline F-002 (inferred) | Canonical Swift guardrail system cannot run on Windows; Windows needs equivalent (not identical) guardrail tooling |
| C-003 | .NET SDK not installed | Baseline F-002 | .NET-based guardrail tooling blocked until disk triage |
| C-004 | C: drive critically low (10.2 GB free) | Baseline F-001; revised to MEDIUM per WIN-AGENT-HARNESS-PLAN-1 §7 | Guardrail preflight must measure C: free space and flag below threshold; large SDK installs blocked |
| C-005 | MSVC compiler not in PATH | Baseline F-003 | Rust builds may still work via VS Installer auto-detection; guardrails should test, not assume |
| C-006 | Non-admin shell (elevation required for service ops) | Baseline elevation check | Service-state guardrails must distinguish query (permitted) from mutation (blocked without elevation) |
| C-007 | NSSM-based service stack | Baseline §20 | Service guardrails must account for NSSM wrapping PowerShell launcher → Python router → llama-server chain |
| C-008 | Windows 10 22H2 past EOS | Baseline F-007 | Operational risk; guardrails should log Windows build but not block Phase 0 work |
| C-009 | PATH contains Ollama/LM Studio entries | Baseline F-008 | Guardrail startup-sequence should note conflicting PATH entries |
| C-010 | Model, repo, build artifacts on G: drive (132 GB free) | Baseline §7, WIN-AGENT-HARNESS-PLAN-1 §7 | C: drive space does not block model, build, or harness work; G: is the working drive |
| C-011 | Three-link proof chain sealed: source HEAD → artifact hash → governed rebuild | Roadmap §Runtime Proof Chain | Guardrail receipt system must interoperate with existing v2 receipt schema (48 checks) |
| C-012 | Pre-mutation hook exists at `scripts/harness/pre-mutation-check.ps1` | WIN-PACKET-VALIDATION-HOOK-1 | Future guardrails should call or compose with this hook, not replace it |

---

## 4. Remaining Sprint Inventory

Each sprint entry includes:
- **Sprint ID** — unique identifier
- **Purpose** — one-line description
- **Phase** — Phase 0a/0b/0c/0d/1
- **Dependencies** — prerequisite sprints
- **Allowed mutation scope** — paths and file types the sprint may create/modify
- **Forbidden actions** — actions explicitly out of scope
- **Expected guardrail categories** — which guardrail categories (from companion document) this sprint requires
- **Required preflight checks** — checks that must pass before sprint work begins
- **Acceptance gates** — verifiable conditions for sprint completion
- **Closeout requirements** — evidence that must be produced at closeout
- **Recommended next sprint** — natural successor
- **Origin note** — which planning document originally proposed this sprint

---

### Phase 0a — Harness Core (Active)

Closes the remaining gaps in the custody sandbox mechanical layer (Layer 1 of the custody model). Builds on the pre-mutation hook completed in WIN-PACKET-VALIDATION-HOOK-1.

#### S-01: WIN-HARNESS-POSTFLIGHT-1

| Field | Value |
|-------|-------|
| **Purpose** | Build post-flight state verification and receipt generation for the harness. Complements the pre-mutation hook to complete the pre/post-flight cycle defined in the custody sandbox model. |
| **Phase** | Phase 0a — Harness Core |
| **Dependencies** | WIN-PACKET-VALIDATION-HOOK-1 (pre-mutation hook exists) |
| **Allowed mutation scope** | `scripts/harness/` (new post-flight script), `docs/sprints/WIN-HARNESS-POSTFLIGHT-1.md`, `docs/receipts/WIN-HARNESS-POSTFLIGHT-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | Service start/stop, model workload, runtime/router/model code change, environment repair, firewall change, auto-start change, modification of existing planning docs from WIN-AGENT-HARNESS-PLAN-1, modification of pre-mutation-check.ps1 |
| **Expected guardrail categories** | G-001 (Pre-mutation state verification), G-002 (Post-mutation state verification), G-004 (Orphan-process guard) |
| **Required preflight checks** | HEAD matches expected sprint start; working tree clean (or classified); `LibrarianRunTimeNode` Stopped/Manual; ports 9120-9130 free; zero orphan runtime processes; C: drive >= 5 GB |
| **Acceptance gates** | Post-flight script exists; script executes without parse errors; script detects state changes after mutation; script generates machine-readable receipt; pre-mutation hook still functional; no service start/stop performed |
| **Closeout requirements** | Receipt documenting PASS/FAIL for each gate; working tree state; HEAD unchanged; origin sync status |
| **Recommended next sprint** | WIN-HARNESS-RECEIPT-TEMPLATE-1 |
| **Origin** | WIN-CUSTODY-SANDBOX-MODEL.md §7 (P1); WIN-SPRINT-SEQUENCE.md §4 (A3); WIN-HARNESS-PARITY-ROADMAP.md §5 (step 2) |

#### S-02: WIN-HARNESS-RECEIPT-TEMPLATE-1

| Field | Value |
|-------|-------|
| **Purpose** | Create standardized sprint receipt templates for the harness. Define receipt conventions that can be reused by all subsequent sprints. |
| **Phase** | Phase 0a — Harness Core |
| **Dependencies** | S-01 (WIN-HARNESS-POSTFLIGHT-1) |
| **Allowed mutation scope** | `scripts/harness/` (new receipt template/generator), `docs/sprints/WIN-HARNESS-RECEIPT-TEMPLATE-1.md`, `docs/receipts/WIN-HARNESS-RECEIPT-TEMPLATE-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | Same as S-01, plus no modification of existing receipt schemas (v1/v2) without explicit scope |
| **Expected guardrail categories** | G-002 (Post-mutation state verification), G-003 (Service-state guard), G-005 (Git-state guard), G-012 (Receipt-integrity guard) |
| **Required preflight checks** | Same as S-01 |
| **Acceptance gates** | Receipt template exists; template validates against sprint conventions; example receipt generated from template; template is machine-readable; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; template file; example output |
| **Recommended next sprint** | WIN-HARNESS-CONTRACT-RUNNER-1 |
| **Origin** | WIN-SPRINT-SEQUENCE.md §4 (A4); WIN-AGENT-HARNESS-PLAN.md §8 (row 3) |

#### S-03: WIN-HARNESS-CONTRACT-RUNNER-1

| Field | Value |
|-------|-------|
| **Purpose** | Create a unified contract test runner that wraps existing test scripts under a single harness entry point. Reduces agent guesswork about which test to run. |
| **Phase** | Phase 0a — Harness Core |
| **Dependencies** | S-01 (WIN-HARNESS-POSTFLIGHT-1) |
| **Allowed mutation scope** | `scripts/harness/` (new contract-runner script), `docs/sprints/WIN-HARNESS-CONTRACT-RUNNER-1.md`, `docs/receipts/WIN-HARNESS-CONTRACT-RUNNER-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | Same as S-01; no modification of individual test scripts (wrapper only) |
| **Expected guardrail categories** | G-006 (Contract-test guard), G-002 (Post-mutation state verification) |
| **Required preflight checks** | Same as S-01 |
| **Acceptance gates** | Runner script exists; runner discovers and invokes existing test scripts; runner reports pass/fail per test; runner exits 0 on all-pass, 1 on any-fail; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; runner script; test output log |
| **Recommended next sprint** | WIN-HARNESS-BASELINE-DIFF-1 or WIN-SPRINT-LEDGER-1 |
| **Origin** | WIN-SPRINT-SEQUENCE.md §4 (A5); WIN-HARNESS-PARITY-ROADMAP.md §5 (step 4) |

#### S-04: WIN-HARNESS-BASELINE-DIFF-1

| Field | Value |
|-------|-------|
| **Purpose** | Create a baseline comparison tool that diffs current environment state against the sealed baseline from WIN-AGENT-HARNESS-ENV-BASELINE-1. Detects and reports environment drift. |
| **Phase** | Phase 0a — Harness Core |
| **Dependencies** | WIN-AGENT-HARNESS-ENV-BASELINE-1 (baseline exists); S-01 (WIN-HARNESS-POSTFLIGHT-1) |
| **Allowed mutation scope** | `scripts/harness/` (new baseline-diff script), `docs/sprints/WIN-HARNESS-BASELINE-DIFF-1.md`, `docs/receipts/WIN-HARNESS-BASELINE-DIFF-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | Same as S-01; no modification of the sealed baseline document |
| **Expected guardrail categories** | G-007 (Baseline-drift guard), G-008 (Tool-version guard), G-011 (Environment-health guard) |
| **Required preflight checks** | Same as S-01; baseline document must exist |
| **Acceptance gates** | Diff script exists; script reads baseline file; script detects intentional drift; script reports no false positives on clean state; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; diff script; example diff output |
| **Recommended next sprint** | WIN-SPRINT-LEDGER-1 |
| **Origin** | WIN-CUSTODY-SANDBOX-MODEL.md §7 (P2); WIN-AGENT-HARNESS-PLAN.md §3.3 |

#### S-05: WIN-SPRINT-LEDGER-1

| Field | Value |
|-------|-------|
| **Purpose** | Create a `sprint-ledger.json` convention and tooling for machine-readable sprint tracking. Addresses baseline finding F-005. |
| **Phase** | Phase 0a — Harness Core |
| **Dependencies** | S-02 (WIN-HARNESS-RECEIPT-TEMPLATE-1) or S-04 |
| **Allowed mutation scope** | Root (new `sprint-ledger.json`), `scripts/harness/` (ledger tooling), `docs/sprints/WIN-SPRINT-LEDGER-1.md`, `docs/receipts/WIN-SPRINT-LEDGER-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | Same as S-01; no modification of existing sprint docs (ledger is additive) |
| **Expected guardrail categories** | G-009 (Sprint-scope guard), G-012 (Receipt-integrity guard) |
| **Required preflight checks** | Same as S-01 |
| **Acceptance gates** | `sprint-ledger.json` exists and validates; ledger schema documented; ledger tooling (create/update/query) exists; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; ledger schema; example ledger entry |
| **Recommended next sprint** | Phase 0b — Parallel Maintenance (any) or Phase 0c — Layer 1 Operations |
| **Origin** | Baseline F-005; WIN-SPRINT-SEQUENCE.md §4 (A6) |

---

### Phase 0b — Parallel Maintenance

Addresses baseline findings and operational risks. Non-blocking — can run in parallel with Phase 0a or Phase 0c.

#### S-06: WIN-DISK-SPACE-RISK-TRIAGE-1

| Field | Value |
|-------|-------|
| **Purpose** | Free C: drive space to create safe operating margin for future build/test operations. Addresses baseline finding F-001 (revised to MEDIUM). |
| **Phase** | Phase 0b — Parallel Maintenance |
| **Dependencies** | None (parallel with Phase 0a) |
| **Allowed mutation scope** | No repo mutations (OS/disk level only: temp file cleanup, log rotation, hibernation disable, user cache cleanup). Docs: `docs/sprints/WIN-DISK-SPACE-RISK-TRIAGE-1.md`, `docs/receipts/WIN-DISK-SPACE-RISK-TRIAGE-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | No model file deletion, no repo file deletion, no service mutation, no registry changes outside documented scope, no application uninstall |
| **Expected guardrail categories** | G-011 (Environment-health guard — disk space threshold) |
| **Required preflight checks** | Same as S-01; record exact C: free space before cleanup |
| **Acceptance gates** | C: drive free space increased by documented amount; no repo files harmed; no service disruption; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; before/after disk space measurements; list of cleanup actions taken |
| **Recommended next sprint** | WIN-MSVCPATH-BASELINE-1 or return to Phase 0a |
| **Origin** | Baseline F-001; WIN-SPRINT-SEQUENCE.md §4 (B1) |

#### S-07: WIN-MSVCPATH-BASELINE-1

| Field | Value |
|-------|-------|
| **Purpose** | Document and verify MSVC resolution path for Rust builds from a non-Developer-Command-Prompt session. Addresses baseline finding F-003. |
| **Phase** | Phase 0b — Parallel Maintenance |
| **Dependencies** | None (parallel with Phase 0a) |
| **Allowed mutation scope** | `docs/sprints/WIN-MSVCPATH-BASELINE-1.md`, `docs/receipts/WIN-MSVCPATH-BASELINE-1-RECEIPT.md`, `SESSION-HANDOFF.md`. No script mutation (pure documentation). |
| **Forbidden actions** | No MSVC installation, no VS BuildTools modification, no PATH mutation |
| **Expected guardrail categories** | G-008 (Tool-version guard) |
| **Required preflight checks** | Same as S-01; note whether `cargo build` works from current shell |
| **Acceptance gates** | MSVC resolution path documented; `cargo build --release` tested and result recorded; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; MSVC path documentation; build test result |
| **Recommended next sprint** | WIN-PATH-HYGIENE-1 |
| **Origin** | Baseline F-003; WIN-SPRINT-SEQUENCE.md §4 (B3) |

#### S-08: WIN-PATH-HYGIENE-1

| Field | Value |
|-------|-------|
| **Purpose** | Clean up conflicting PATH entries (Ollama, LM Studio) that may interfere with runtime-node binary selection. Addresses baseline finding F-008. |
| **Phase** | Phase 0b — Parallel Maintenance |
| **Dependencies** | None (parallel with Phase 0a) |
| **Allowed mutation scope** | No repo mutations (user environment level only). Docs: `docs/sprints/WIN-PATH-HYGIENE-1.md`, `docs/receipts/WIN-PATH-HYGIENE-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | No removal of required PATH entries, no deletion of Ollama/LM Studio installations (just PATH cleanup) |
| **Expected guardrail categories** | G-008 (Tool-version guard), G-011 (Environment-health guard) |
| **Required preflight checks** | Same as S-01; record full PATH before cleanup |
| **Acceptance gates** | PATH documented before/after; pre-mutation hook still functional; alternative runtimes still launchable if needed |
| **Closeout requirements** | Receipt; before/after PATH recording; list of changes |
| **Recommended next sprint** | WIN-WINDOWS-UPGRADE-EVAL-1 |
| **Origin** | Baseline F-008; WIN-SPRINT-SEQUENCE.md §4 (B2) |

#### S-09: WIN-WINDOWS-UPGRADE-EVAL-1

| Field | Value |
|-------|-------|
| **Purpose** | Evaluate whether upgrading from Windows 10 22H2 (past EOS) to Windows 11 is needed for the Librarian host role. Addresses baseline finding F-007. |
| **Phase** | Phase 0b — Parallel Maintenance |
| **Dependencies** | None (parallel with Phase 0a) |
| **Allowed mutation scope** | Documentation only: `docs/sprints/WIN-WINDOWS-UPGRADE-EVAL-1.md`, `docs/receipts/WIN-WINDOWS-UPGRADE-EVAL-1-RECEIPT.md`, `SESSION-HANDOFF.md`. Potentially `docs/planning/WIN-WINDOWS-UPGRADE-EVAL.md` |
| **Forbidden actions** | No upgrade execution, no OS modification, no registry changes |
| **Expected guardrail categories** | G-011 (Environment-health guard) |
| **Required preflight checks** | Same as S-01 |
| **Acceptance gates** | Evaluation document created; upgrade decision recorded; risk assessment completed; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; evaluation document; decision record |
| **Recommended next sprint** | Return to Phase 0a or Phase 0c |
| **Origin** | Baseline F-007; WIN-SPRINT-SEQUENCE.md §4 (B1) |

---

### Phase 0c — Layer 1 Operations

Continue the existing Layer 1 roadmap from `WINDOWS-PC-SPRINT-ROADMAP.md`. These sprints improve operational tooling and profile metadata.

#### S-10: WIN-RUNTIME-OPERATIONS-1

| Field | Value |
|-------|-------|
| **Purpose** | Create a small operator toolkit for the Windows Runtime Node (status, start, stop, logs, clean-check scripts). |
| **Phase** | Phase 0c — Layer 1 Operations |
| **Dependencies** | None (can start immediately after Phase 0a harness core is stable) |
| **Allowed mutation scope** | `scripts/operations/` (new operator scripts), `docs/sprints/WIN-RUNTIME-OPERATIONS-1.md`, `docs/receipts/WIN-RUNTIME-OPERATIONS-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | Service start/stop outside documented scope; model workload; modification of existing harness scripts |
| **Expected guardrail categories** | G-001 (Pre-mutation state verification), G-003 (Service-state guard), G-004 (Orphan-process guard), G-011 (Environment-health guard) |
| **Required preflight checks** | Same as S-01; plus verify operations scripts directory exists |
| **Acceptance gates** | Operator scripts exist and parse; scripts do not kill unrelated processes; scripts preserve Manual startup policy; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; list of created scripts; test output |
| **Recommended next sprint** | WIN-RUNTIME-PROFILES-CLEANUP-1 |
| **Origin** | WINDOWS-PC-SPRINT-ROADMAP.md §1; WIN-SPRINT-SEQUENCE.md §4 (C1) |

#### S-11: WIN-RUNTIME-PROFILES-CLEANUP-1

| Field | Value |
|-------|-------|
| **Purpose** | Normalize `config/model-profiles.json` metadata to reflect verified reality. Add missing fields: `verified_context`, `verified_ngl`, `stability`, `requires_reduced_offload`, `notes`. |
| **Phase** | Phase 0c — Layer 1 Operations |
| **Dependencies** | S-10 (WIN-RUNTIME-OPERATIONS-1) for reliable start/stop during profile testing |
| **Allowed mutation scope** | `config/model-profiles.json` (metadata fields only — no profile removal/addition), `docs/sprints/WIN-RUNTIME-PROFILES-CLEANUP-1.md`, `docs/receipts/WIN-RUNTIME-PROFILES-CLEANUP-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | No profile addition or removal; no model binary change; no backend swap; no service mutation beyond documented start/stop for testing |
| **Expected guardrail categories** | G-010 (Model-profile guard), G-001 (Pre-mutation state verification), G-011 (Environment-health guard) |
| **Required preflight checks** | Same as S-01; plus profile config must parse as valid JSON |
| **Acceptance gates** | All 5 profiles have complete metadata; router loads all profiles without error; endpoint matrix still passes; no model binaries committed; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; updated `model-profiles.json`; profile test output |
| **Recommended next sprint** | Phase 1 — Layer 2/3 Transition (ROUTER-CONTRACT-TESTS-1) |
| **Origin** | WINDOWS-PC-SPRINT-ROADMAP.md §2; WIN-SPRINT-SEQUENCE.md §4 (C2) |

---

### Phase 0d — Harness Hardening

Additional harness tooling beyond the core pre/post-flight loop. Lower priority — can be deferred to Phase 1 if needed.

#### S-12: WIN-HARNESS-ACTION-RECEIPT-1

| Field | Value |
|-------|-------|
| **Purpose** | Create granular action receipt generation for discrete harness actions (service start, port check, model select). |
| **Phase** | Phase 0d — Harness Hardening |
| **Dependencies** | S-02 (WIN-HARNESS-RECEIPT-TEMPLATE-1) |
| **Allowed mutation scope** | `scripts/harness/` (action receipt tooling), `docs/sprints/WIN-HARNESS-ACTION-RECEIPT-1.md`, `docs/receipts/WIN-HARNESS-ACTION-RECEIPT-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | Same as S-01; no modification of sprint-level receipt conventions |
| **Expected guardrail categories** | G-012 (Receipt-integrity guard) |
| **Required preflight checks** | Same as S-01 |
| **Acceptance gates** | Action receipt tool exists; tool generates receipts for test actions; receipts validate against schema; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; action receipt examples |
| **Recommended next sprint** | WIN-HARNESS-LEDGER-1 |
| **Origin** | WIN-CUSTODY-SANDBOX-MODEL.md §7 (P3) |

#### S-13: WIN-HARNESS-LEDGER-1

| Field | Value |
|-------|-------|
| **Purpose** | Build the cross-sprint custody ledger tooling that chains sprint receipts into an auditable sequence. |
| **Phase** | Phase 0d — Harness Hardening |
| **Dependencies** | S-05 (WIN-SPRINT-LEDGER-1) — ledger convention must exist first |
| **Allowed mutation scope** | `scripts/harness/` (ledger tooling), `sprint-ledger.json` (update), `docs/sprints/WIN-HARNESS-LEDGER-1.md`, `docs/receipts/WIN-HARNESS-LEDGER-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | Same as S-01; no modification of sealed sprint receipts |
| **Expected guardrail categories** | G-009 (Sprint-scope guard), G-012 (Receipt-integrity guard) |
| **Required preflight checks** | Same as S-01; sprint-ledger.json must exist |
| **Acceptance gates** | Ledger tooling exists; tool chains sprint receipts into sequence; tool detects gaps in chain; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; ledger tooling; example chained ledger output |
| **Recommended next sprint** | Phase 1 — Layer 2/3 Transition |
| **Origin** | WIN-CUSTODY-SANDBOX-MODEL.md §7 (P3) |

---

### Phase 1 — Layer 2/3 Transition

Router portability, native daemon, and Windows Librarian app work.

#### S-14: ROUTER-CONTRACT-TESTS-1

| Field | Value |
|-------|-------|
| **Purpose** | Create shared conformance tests for any implementation of the portable Router contract. Covers start/status, profiles, health, select, chat, restart, refusal cases, and cleanup. |
| **Phase** | Phase 1 — Layer 2/3 Transition |
| **Dependencies** | S-10 (WIN-RUNTIME-OPERATIONS-1) for reliable start/stop |
| **Allowed mutation scope** | `tests/router-contract/` (new test files), `scripts/harness/` (contract runner update), `docs/sprints/ROUTER-CONTRACT-TESTS-1.md`, `docs/receipts/ROUTER-CONTRACT-TESTS-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | No modification of Python router; no modification of Rust router; no change to router endpoints |
| **Expected guardrail categories** | G-006 (Contract-test guard), G-002 (Post-mutation state verification) |
| **Required preflight checks** | Same as S-01; plus operator scripts must be available for service start/stop |
| **Acceptance gates** | Contract test suite exists; Python router passes all tests; failures reported as contract violations; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; test suite; test run output |
| **Recommended next sprint** | ROUTER-RUST-CORE-1 |
| **Origin** | WINDOWS-PC-SPRINT-ROADMAP.md §3; WIN-SPRINT-SEQUENCE.md §4 (D1) |

#### S-15: ROUTER-RUST-CORE-1

| Field | Value |
|-------|-------|
| **Purpose** | Plan and/or begin a native Router core implementation in Rust against the portable contract. Maps contract tests to implementation, keeps OS-specific service code thin. |
| **Phase** | Phase 1 — Layer 2/3 Transition |
| **Dependencies** | S-14 (ROUTER-CONTRACT-TESTS-1) |
| **Allowed mutation scope** | `rust-router/src/` (new Rust code matching contract), `docs/sprints/ROUTER-RUST-CORE-1.md`, `docs/receipts/ROUTER-RUST-CORE-1-RECEIPT.md`, `SESSION-HANDOFF.md`. Planning-only version: documentation only. |
| **Forbidden actions** | No premature replacement of Python router; no Windows-specific code in portable contract layer |
| **Expected guardrail categories** | G-006 (Contract-test guard), G-008 (Tool-version guard) |
| **Required preflight checks** | Same as S-01; plus `cargo build` must work |
| **Acceptance gates** | Rust core maps to contract tests; OS-specific code is thin; Python reference router remains functional; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; architecture document; contract-test mapping |
| **Recommended next sprint** | WIN-RUST-SERVICE-1 |
| **Origin** | WINDOWS-PC-SPRINT-ROADMAP.md §4; WIN-SPRINT-SEQUENCE.md §4 (D2) |

#### S-16: WIN-RUST-SERVICE-1

| Field | Value |
|-------|-------|
| **Purpose** | Replace the current NSSM → PowerShell → Python service stack with a native Windows service wrapper driven by the Rust router daemon. |
| **Phase** | Phase 1 — Layer 2/3 Transition |
| **Dependencies** | S-15 (ROUTER-RUST-CORE-1) |
| **Allowed mutation scope** | `rust-router/` (service wrapper code), `runtime/bin/` (new binary if approved), `docs/sprints/WIN-RUST-SERVICE-1.md`, `docs/receipts/WIN-RUST-SERVICE-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | No removal of Python router until feature parity is proven; no service mutation without documented rollback path |
| **Expected guardrail categories** | G-003 (Service-state guard), G-004 (Orphan-process guard), G-005 (Git-state guard) |
| **Required preflight checks** | Same as S-01; plus Owner approval for service mutation |
| **Acceptance gates** | Native service achieves feature parity with NSSM proof; lifecycle behavior is not weaker; rollback path exists and is documented; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; service test results; rollback documentation |
| **Recommended next sprint** | WIN-LIBRARIAN-APP-PLAN-1 |
| **Origin** | WINDOWS-PC-SPRINT-ROADMAP.md §5; WIN-SPRINT-SEQUENCE.md §4 (implicit) |

#### S-17: WIN-LIBRARIAN-APP-PLAN-1

| Field | Value |
|-------|-------|
| **Purpose** | Decide the architecture for a Windows version of The Librarian app. Survey tech options (Tauri, Electron, Python web, .NET), decide the app shell, and document the architecture. |
| **Phase** | Phase 1 — Layer 2/3 Transition |
| **Dependencies** | Phase 0c complete (Layer 1 ops and profiles stable) |
| **Allowed mutation scope** | `docs/planning/WINDOWS-LIBRARIAN-APP.md` (architecture doc), `docs/sprints/WIN-LIBRARIAN-APP-PLAN-1.md`, `docs/receipts/WIN-LIBRARIAN-APP-PLAN-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | No app implementation; no code scaffolding; no service mutation; no model workload |
| **Expected guardrail categories** | G-013 (App-boundary guard — conceptual at this stage) |
| **Required preflight checks** | Same as S-01; plus WIN-LIBRARIAN-HOST-OPTIONS.md must be current |
| **Acceptance gates** | Architecture document exists; tech decision recorded; dependency chain documented; no implementation performed; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; architecture document; decision record |
| **Recommended next sprint** | WIN-LIBRARIAN-SHELL-1 |
| **Origin** | WINDOWS-PC-SPRINT-ROADMAP.md §6; WIN-SPRINT-SEQUENCE.md §4 (D3); WIN-LIBRARIAN-HOST-OPTIONS.md |

#### S-18: WIN-LIBRARIAN-SHELL-1

| Field | Value |
|-------|-------|
| **Purpose** | Create the first Windows app shell with no risky custody logic yet. App launches, shows placeholder navigation, can display runtime-node status. |
| **Phase** | Phase 1 — Layer 2/3 Transition |
| **Dependencies** | S-17 (WIN-LIBRARIAN-APP-PLAN-1) |
| **Allowed mutation scope** | New app directory (as specified by architecture doc), `docs/sprints/WIN-LIBRARIAN-SHELL-1.md`, `docs/receipts/WIN-LIBRARIAN-SHELL-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | No custody logic; no file writes outside app sandbox; no service mutation; no authority-bearing actions |
| **Expected guardrail categories** | G-013 (App-boundary guard) |
| **Required preflight checks** | Same as S-01 |
| **Acceptance gates** | App launches; navigation renders; runtime-node status is visible; no authority-bearing actions implemented; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; app shell screenshot/log |
| **Recommended next sprint** | WIN-LIBRARIAN-RUNTIME-INTEGRATION-1 |
| **Origin** | WINDOWS-PC-SPRINT-ROADMAP.md §7 |

#### S-19: WIN-LIBRARIAN-RUNTIME-INTEGRATION-1

| Field | Value |
|-------|-------|
| **Purpose** | Connect Windows Librarian UI to the local runtime node as advisory compute only. Show profiles, select advisory runtime, send prompt, display advisory response. |
| **Phase** | Phase 1 — Layer 2/3 Transition |
| **Dependencies** | S-18 (WIN-LIBRARIAN-SHELL-1) |
| **Allowed mutation scope** | App source code (as specified by architecture doc), `docs/sprints/WIN-LIBRARIAN-RUNTIME-INTEGRATION-1.md`, `docs/receipts/WIN-LIBRARIAN-RUNTIME-INTEGRATION-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | No file writes from app; no approval bypass; no canonical output claims |
| **Expected guardrail categories** | G-013 (App-boundary guard), G-010 (Model-profile guard) |
| **Required preflight checks** | Same as S-01; plus LibrarianRunTimeNode service must be stoppable for testing |
| **Acceptance gates** | Profiles display; advisory runtime selection works; prompts send and responses display; output is clearly advisory; failure is non-fatal; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; integration test output |
| **Recommended next sprint** | WIN-LIBRARIAN-CUSTODY-UI-1 |
| **Origin** | WINDOWS-PC-SPRINT-ROADMAP.md §8 |

#### S-20: WIN-LIBRARIAN-CUSTODY-UI-1

| Field | Value |
|-------|-------|
| **Purpose** | Bring governed custody concepts into the Windows app UI. Project identity, imported repo state, receipts, validation, Owner approval/rejection, governed action packet display, proof chain visualization. |
| **Phase** | Phase 1 — Layer 2/3 Transition |
| **Dependencies** | S-19 (WIN-LIBRARIAN-RUNTIME-INTEGRATION-1) |
| **Allowed mutation scope** | App source code (custody UI additions), `docs/sprints/WIN-LIBRARIAN-CUSTODY-UI-1.md`, `docs/receipts/WIN-LIBRARIAN-CUSTODY-UI-1-RECEIPT.md`, `SESSION-HANDOFF.md` |
| **Forbidden actions** | No raw action execution without canonical semantics; no weakening of Owner authority |
| **Expected guardrail categories** | G-013 (App-boundary guard), G-009 (Sprint-scope guard), G-012 (Receipt-integrity guard) |
| **Required preflight checks** | Same as S-01 |
| **Acceptance gates** | UI displays project identity and repo state; receipts render in UI; Owner approval/rejection flow works; proof chain visualization renders; pre-mutation hook still functional |
| **Closeout requirements** | Receipt; custody UI test output |
| **Recommended next sprint** | Review — evaluate whether Windows Librarian enters maintenance or new capability phase |
| **Origin** | WINDOWS-PC-SPRINT-ROADMAP.md §9 |

---

## 5. Phase Diagram

```
Phase 0a — Harness Core
═══════════════════════════════════════════════════════════════════
Current: WIN-PC-REMAINING-SPRINTS-PLAN-1  (planning — this sprint)
                                                                      
WIN-PACKET-VALIDATION-HOOK-1              ✅ Complete (pre-mutation hook)
       │
       ▼
WIN-HARNESS-POSTFLIGHT-1                  ← Recommended NEXT
       │
       ├──→ WIN-HARNESS-RECEIPT-TEMPLATE-1
       │         │
       │         ├──→ WIN-HARNESS-CONTRACT-RUNNER-1
       │         │
       │         └──→ WIN-SPRINT-LEDGER-1
       │
       └──→ WIN-HARNESS-BASELINE-DIFF-1
                 │
                 └──→ WIN-SPRINT-LEDGER-1


Phase 0b — Parallel Maintenance (non-blocking, any order)
═══════════════════════════════════════════════════════════════════
WIN-DISK-SPACE-RISK-TRIAGE-1
WIN-MSVCPATH-BASELINE-1
WIN-PATH-HYGIENE-1
WIN-WINDOWS-UPGRADE-EVAL-1


Phase 0c — Layer 1 Operations (after harness core is stable)
═══════════════════════════════════════════════════════════════════
WIN-RUNTIME-OPERATIONS-1
       │
       ▼
WIN-RUNTIME-PROFILES-CLEANUP-1


Phase 0d — Harness Hardening (lower priority, deferrable)
═══════════════════════════════════════════════════════════════════
WIN-HARNESS-ACTION-RECEIPT-1
       │
       ▼
WIN-HARNESS-LEDGER-1


Phase 1 — Layer 2/3 Transition (after Phase 0c)
═══════════════════════════════════════════════════════════════════
ROUTER-CONTRACT-TESTS-1
       │
       ▼
ROUTER-RUST-CORE-1
       │
       ▼
WIN-RUST-SERVICE-1
       │
       ▼
WIN-LIBRARIAN-APP-PLAN-1
       │
       ▼
WIN-LIBRARIAN-SHELL-1
       │
       ▼
WIN-LIBRARIAN-RUNTIME-INTEGRATION-1
       │
       ▼
WIN-LIBRARIAN-CUSTODY-UI-1
```

---

## 6. Total Sprint Count

| Phase | Sprint Count | Status |
|-------|-------------|--------|
| Phase 0a — Harness Core | 5 sprints (S-01 through S-05) | Active |
| Phase 0b — Parallel Maintenance | 4 sprints (S-06 through S-09) | Available |
| Phase 0c — Layer 1 Operations | 2 sprints (S-10, S-11) | Pending harness core |
| Phase 0d — Harness Hardening | 2 sprints (S-12, S-13) | Deferrable |
| Phase 1 — Layer 2/3 Transition | 7 sprints (S-14 through S-20) | Future |
| **Total remaining** | **20 sprints** | |

---

## 7. Guardrail Categories Summary

See companion document `docs/planning/WIN-PC-SPRINT-GUARDRAIL-NEEDS.md` for full definitions.

| ID | Guardrail / Profile Category | Phase Required | Sprint Coverage |
|----|------------------------------|----------------|-----------------|
| G-001 | Pre-mutation state verification | 0a | S-01, S-10 |
| G-002 | Post-mutation state verification | 0a | S-01, S-02, S-03 |
| G-003 | Service-state guard | 0a | S-02, S-10, S-16 |
| G-004 | Orphan-process guard | 0a | S-01, S-10, S-16 |
| G-005 | Git-state guard | 0a | S-02 |
| G-006 | Contract-test guard | 0a | S-03, S-14, S-15 |
| G-007 | Baseline-drift guard | 0a | S-04 |
| G-008 | Tool-version guard | 0a | S-04, S-07, S-08, S-15 |
| G-009 | Sprint-scope guard | 0a | S-05, S-13, S-20 |
| G-010 | Model-profile guard | 0c | S-11, S-19 |
| G-011 | Environment-health guard | 0b | S-04, S-06, S-08, S-09, S-10, S-11 |
| G-012 | Receipt-integrity guard | 0a | S-02, S-05, S-12, S-13, S-20 |
| G-013 | App-boundary guard | 1 | S-17, S-18, S-19, S-20 |

---

## 8. Relationship to Canonical Mac/Librarian Guardrail-Profile System

This document records **Windows-local** sprint needs and expected guardrail categories. It is **not** the canonical guardrail-profile system.

When the Mac/Librarian side defines a canonical guardrail-profile system (expected in a future sprint), the following mapping rules apply:

1. **Canonical wins.** If the Mac/Librarian system defines a guardrail or profile that covers the same concern as a Windows-local guardrail, the canonical definition takes precedence.
2. **Windows adapts.** Windows-local guardrails may require adaptation (e.g., PowerShell 5.1 instead of Swift, NSSM service model instead of launchd). These adaptations are recorded in the Windows implementation, not in the canonical spec.
3. **Superset permitted.** Windows may implement guardrail categories that the canonical system does not yet define, as long as they do not conflict with canonical boundaries.
4. **Receipt interoperability.** All Windows-local receipts must be convertible to or compatible with the canonical receipt schema.
5. **Migration path.** When the canonical system is ready, a `WIN-CANONICAL-GUARDRAIL-ADOPT-1` sprint will map each Windows-local guardrail to its canonical equivalent and retire the superseded Windows-only implementation.

---

## 9. Owner Decision Points

| Decision | Required At | Question |
|----------|-------------|----------|
| D-01 | Now (this sprint) | Approve WIN-HARNESS-POSTFLIGHT-1 as the next sprint? |
| D-02 | After Phase 0a | Ready for Phase 0c (Layer 1 Operations) or continue harness hardening? |
| D-03 | After Phase 0b (any) | Is C: drive space sufficient for SDK installs? |
| D-04 | After S-10 | Approve profile metadata normalization (model context, ngl, stability fields)? |
| D-05 | At Phase 1 entry | Approve native Router core planning? |
| D-06 | At S-16 | Approve NSSM→native service migration? |
| D-07 | At S-17 | Approve Windows Librarian app architecture? |
