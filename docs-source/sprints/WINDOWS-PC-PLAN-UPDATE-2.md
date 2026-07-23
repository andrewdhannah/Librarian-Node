# Sprint: WINDOWS-PC-PLAN-UPDATE-2

**Status:** COMPLETED
**Date:** 2026-06-23
**Repository:** librarian-runtime-node

## Objective

Update the Windows PC runtime roadmap and session-handoff documentation to reflect the completed runtime integration proof chain and re-order the next Layer 1 priorities.

The proof chain changed the planning baseline. Jumping into execution sprints from the stale roadmap risked building on incorrect assumptions. This sprint is a docs-only reconciliation sprint.

## Starting State

| Check | Status |
|-------|--------|
| TheLibrarian-main HEAD | `1e32002` — clean, up to date |
| librarian-runtime-node HEAD | `e7cfe33` — clean, up to date |
| Stashes (both repos) | Empty |
| LibrarianRunTimeNode service | Stopped / Manual |
| Port 9130 | Free |
| llama-server orphans | 0 |
| rust-router orphans | 0 |

## Completed Proof Chain (to be memorialized)

| Link | Status | Key Evidence |
|------|--------|-------------|
| WIN-RUNTIME-RECEIPT-CLEANUP-1 | Sealed at `51c2e85` | Reconciled prior `partial` receipt; fresh proof shows clean stop |
| WIN-RUNTIME-RECEIPTS-2 | Sealed at `f82d301` | v2 schema, 48-check verifier, artifact hash capture |
| WIN-RUNTIME-QUALIFICATION-1 | Sealed at `e7cfe33` | Governed rebuild + hash comparison; 38/38 gate passed |
| Source HEAD proof | ✅ | `e7cfe33` (runtime-node), `1e32002` (main) |
| Artifact hash proof | ✅ | SHA-256 `84EB797A715FDC4CDB634A85FDFEC5C81CB90F37FB700F6EA6C97576B9569DC9` |
| Governed rebuild qualification proof | ✅ | Rebuild hash matches receipt artifact hash |

## Files Changed

| File | Action | Description |
|------|--------|-------------|
| `docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md` | **Updated** | Full rewrite reflecting proof chain, re-ordered sprints, receipt/verifier/qualification reference section |
| `SESSION-HANDOFF.md` | **Updated** | Replaced stale handoff with current HEADs, proof chain summary, updated sprint table, known issues |
| `docs/sprints/WINDOWS-PC-PLAN-UPDATE-2.md` | **New** | This closeout document |

## Summary of Roadmap Changes

### 1. Header metadata updated
- `Current runtime-node baseline:` `c44150b` → `e7cfe33`
- `Last sealed sprint:` `REDUCED-OFFLOAD-FIT-1` → `WIN-RUNTIME-QUALIFICATION-1`
- Added `Previous roadmap version:` reference

### 2. New section: "Runtime Proof Chain (Established)"
Added a dedicated section documenting the three-link proof chain with infrastructure table linking to:
- v2 receipt schema (`receipts/runtime-integration/schema-v2.json`)
- 48-check receipt verifier (`scripts/verify-receipt.ps1`)
- Integration proof v2 script (`scripts/run-integration-proof-v2.ps1`)
- Runtime qualification scripts

### 3. Completed sprints table expanded
Layer 0 — Foundation expanded to include all 8 base sprints.
New section: **Layer 1 — Runtime Qualification and Receipts** with 4 completed sprints (INTEGRATION-1, RECEIPT-CLEANUP-1, RECEIPTS-2, QUALIFICATION-1).
Added Layer 1 Milestone note: "The proof chain is now sealed. All further work builds on this baseline."

### 4. Remaining sprints re-ordered
The recommended execution order table was rewritten:

| Old Order | New Order |
|-----------|-----------|
| 1. PROFILES-CLEANUP-1 | 1–5. (Proof Chain) ✅ Done |
| 2. OPERATIONS-1 | 6. PLAN-UPDATE-1 ✅ Done |
| 3. CONTRACT-TESTS-1 | **7. PLAN-UPDATE-2 ← Current** |
| 4. RUST-CORE-1 | **8. OPERATIONS-1 ← Next** |
| 5. RUST-SERVICE-1 | 9. PROFILES-CLEANUP-1 |
| ... | 10. CONTRACT-TESTS-1 |
| | 11. RUST-CORE-1 |
| | 12. RUST-SERVICE-1 |
| | 13–16. Layer 3 sprints |

Key re-ordering decisions:
- **WIN-RUNTIME-OPERATIONS-1 moves ahead of PROFILES-CLEANUP-1** — operator scripts make profile testing and future work faster and safer.
- **WIN-RUNTIME-PROFILES-CLEANUP-1 re-scoped** — the profile config already has `verified_status`, `evidence_path`, `known_behavior`, `limitations`. Remaining gaps are structured metadata fields (`verified_context`, `verified_ngl`, `stability`, `requires_reduced_offload`, `notes`).
- Dependencies added to each sprint entry.

### 5. New section: "Receipt and Verification Reference"
Added consolidated reference for:
- v2 receipt schema and improvements over v1
- 48-check receipt verifier
- Runtime qualification gate (38/38)
- Anti-loop rules pointer

### 6. Layer 3 scopes updated
Added note to WIN-LIBRARIAN-CUSTODY-UI-1 about runtime-node proof chain visualization.

### 7. SESSION-HANDOFF.md updated
- Sprint roadmap table updated to show 12 completed sprints + current + next
- Proof chain summary added
- Current HEADs updated to `e7cfe33` / `1e32002`
- Receipt section updated to include v2 receipt and qualification record
- Key files table updated with new proof-chain scripts
- Known issues updated with model-profiles metadata gap

## Model Profiles Metadata Gap Inventory

The following fields are present in `config/model-profiles.json`:
- `alias`, `model_file`, `model_path`, `gguf_size_gb`, `backend`
- `ngl`, `context`, `port`, `launch_command`
- `task_classes`, `verified_status`, `evidence_path`
- `authority_status`, `limitations`, `known_behavior`, `test_cells`

The following fields are **missing** (scheduled for WIN-RUNTIME-PROFILES-CLEANUP-1):

| Field | Purpose | Example Value |
|-------|---------|---------------|
| `verified_context` | Explicit context size verified at this ngl | `4096` |
| `verified_ngl` | Explicit GPU layer count verified as stable | `80` or `99` |
| `stability` | Stability rating | `stable`, `conditional`, `unstable` |
| `requires_reduced_offload` | Does this profile need ngl < 99 on RX 570 4GB? | `true` / `false` |
| `notes` | Free-text operational notes | `"Intermittent OOM at ngl=99 with phi-4 on RX 570 4GB"` |

## Hard Constraints Verification

| Constraint | Status |
|------------|--------|
| Documentation only | ✅ No code modified |
| No runtime code modified | ✅ Confirmed |
| No service configuration modified | ✅ Confirmed |
| No model/profile experiments run | ✅ Confirmed |
| No secrets committed | ✅ Confirmed |
| No binaries, logs, cache files committed | ✅ Confirmed |
| Windows anti-loop rules followed | ✅ Confirmed |

## Next Sprint Recommendation

**WIN-RUNTIME-OPERATIONS-1** — Create operator scripts for status, start, stop, logs, and clean-check.

Rationale:
1. Reliable operator commands reduce friction before touching profile metadata or service lifecycle.
2. Receipt and qualification scripts already exist; operator scripts complete the tooling layer.
3. PROFILES-CLEANUP-1 will benefit from being able to quickly start/stop/check the runtime during profile testing.

## Final State

| Check | Result |
|-------|--------|
| **Starting HEADs** | TheLibrarian-main: `1e32002`, runtime-node: `e7cfe33` |
| **Final HEADs** | TheLibrarian-main: `1e32002` (unchanged), runtime-node: `e7cfe33` (unchanged) |
| **Service** | Stopped / Manual ✅ |
| **Port 9130** | Free ✅ |
| **llama-server orphans** | 0 ✅ |
| **rust-router orphans** | 0 ✅ |
| **Stashes** | Empty (unchanged) ✅ |
| **Working tree (runtime-node)** | Modified: 2 files updated (+1 new) — roadmap, handoff, sprint doc |
| **Working tree (TheLibrarian-main)** | Clean ✅ |
