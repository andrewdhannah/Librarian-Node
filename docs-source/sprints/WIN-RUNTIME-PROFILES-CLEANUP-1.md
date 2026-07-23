# Sprint: WIN-RUNTIME-PROFILES-CLEANUP-1

**Status:** COMPLETED
**Date:** 2026-06-23
**Repository:** librarian-runtime-node

## Objective

Normalize runtime model profile metadata so each profile records verified operating constraints and stability notes without changing runtime behavior.

## Starting State

| Check | Status |
|-------|--------|
| TheLibrarian-main HEAD | `1e32002` — clean, up to date |
| librarian-runtime-node HEAD | `d5fa12d` — clean, ahead by 2 |
| Stashes (both repos) | Empty |
| LibrarianRunTimeNode service | Stopped / Manual |
| Port 9130 | Free |
| llama-server orphans | 0 |
| rust-router orphans | 0 |

## Files Changed

| File | Action |
|------|--------|
| `config/model-profiles.json` | **Updated** — 5 new metadata fields added to all 5 profiles |
| `docs/sprints/WIN-RUNTIME-PROFILES-CLEANUP-1.md` | **New** — This closeout document |

## Metadata Fields Added

Five new fields added to each of the 5 profiles:

| Field | Type | Purpose |
|-------|------|---------|
| `verified_context` | integer | Context size verified as stable at the verified ngl |
| `verified_ngl` | integer | GPU layer count verified as stable |
| `stability` | string | Stability rating: `stable` or `conditional` |
| `requires_reduced_offload` | boolean | Does this profile need ngl < 99 on RX 570 4GB? |
| `notes` | string | Free-text operational notes |

## Per-Profile Values and Evidence

### phi-4

| Field | Value | Evidence |
|-------|-------|----------|
| `verified_context` | `4096` | `fixtures/.../phi-4-ngl99.json` — PASS at context=2048 and context=4096 with restart-per-config-change method |
| `verified_ngl` | `99` | `fixtures/.../phi-4-ngl99.json` — verified at ngl=99 |
| `stability` | `"stable"` | Consistently passes all tests. No OOM or crash observed across multiple sprint cycles. |
| `requires_reduced_offload` | `false` | Runs at ngl=99 on RX 570 4GB without issue. |
| `notes` | `"2.32 GB Q4_K_M. General advisory model..."` | Compiled from limitations + known_behavior + evidence. |

### qwen-coder

| Field | Value | Evidence |
|-------|-------|----------|
| `verified_context` | `4096` | `fixtures/.../qwen-coder-ngl99.json` — PASS at context=2048 and context=4096 |
| `verified_ngl` | `99` | `fixtures/.../qwen-coder-ngl99.json` — verified at ngl=99 |
| `stability` | `"stable"` | Consistently passes all tests. Smallest model in deployment. |
| `requires_reduced_offload` | `false` | Runs at ngl=99 on RX 570 4GB without issue. |
| `notes` | `"1.76 GB Q8_0. Best for code tasks..."` | Compiled from evidence verdict + limitations. |

### llama-3.2

| Field | Value | Evidence |
|-------|-------|----------|
| `verified_context` | `4096` | REDUCED-OFFLOAD-FIT-1 — PASS at context=2048, 3072, 4096 at ngl=80 |
| `verified_ngl` | `80` | REDUCED-OFFLOAD-FIT-1 — highest stable ngl on ladder test (ngl=99 OOM) |
| `stability` | `"conditional"` | Stable at ngl=80, OOM at ngl=99. Values between 80-99 untested. |
| `requires_reduced_offload` | `true` | OOM at ngl=99 at any context on RX 570 4GB |
| `notes` | `"2.16 GB Q5_K_M. 3B params..."` | Compiled from limitations + REDUCED-OFFLOAD-FIT-1 results. |

### qwen3

| Field | Value | Evidence |
|-------|-------|----------|
| `verified_context` | `4096` | REDUCED-OFFLOAD-FIT-1 — PASS at context=2048, 3072, 4096 at ngl=80 |
| `verified_ngl` | `80` | REDUCED-OFFLOAD-FIT-1 — highest stable ngl (ngl=99 OOM) |
| `stability` | `"conditional"` | Stable at ngl=80, OOM at ngl=99. Values between 80-99 untested. |
| `requires_reduced_offload` | `true` | OOM at ngl=99 at any context on RX 570 4GB |
| `notes` | `"2.33 GB Q4_K_M. 4B params..."` | Compiled from limitations + REDUCED-OFFLOAD-FIT-1 results. |

### gemma-3

| Field | Value | Evidence |
|-------|-------|----------|
| `verified_context` | `4096` | REDUCED-OFFLOAD-FIT-1 — PASS at context=2048, 3072, 4096 at ngl=80 |
| `verified_ngl` | `80` | REDUCED-OFFLOAD-FIT-1 — highest stable ngl (ngl=99 OOM) |
| `stability` | `"conditional"` | Stable at ngl=80, OOM at ngl=99. Values between 80-99 untested. |
| `requires_reduced_offload` | `true` | OOM at ngl=99 at any context on RX 570 4GB |
| `notes` | `"2.32 GB Q4_K_M. 4B params..."` | Compiled from limitations + REDUCED-OFFLOAD-FIT-1 results. |

## Acceptance Gate Results

| ID | Check | Result |
|----|-------|--------|
| **PROFILE-001** | Locate canonical profile file | ✅ PASS — `config/model-profiles.json` |
| **PROFILE-002** | Add/normalize `verified_context` | ✅ PASS — All 5 profiles have evidence-backed values |
| **PROFILE-003** | Add/normalize `verified_ngl` | ✅ PASS — All 5 profiles have evidence-backed values |
| **PROFILE-004** | Add/normalize `stability` | ✅ PASS — `stable` (2) / `conditional` (3) |
| **PROFILE-005** | Add/normalize `requires_reduced_offload` | ✅ PASS — `false` (2) / `true` (3) |
| **PROFILE-006** | Add/normalize `notes` | ✅ PASS — All 5 profiles have evidence-compiled notes |
| **PROFILE-007** | Every value evidence-backed or marked pending/unknown | ✅ PASS — All values have documented evidence; no pending/unknown fields |
| **PROFILE-008** | Existing aliases and selection behavior unchanged | ✅ PASS — `alias`, `ngl`, `context`, `port`, `launch_command`, `task_classes` all unchanged |
| **PROFILE-009** | JSON validates | ✅ PASS — `ConvertFrom-Json` succeeds, 5 profiles, all fields present |
| **PROFILE-010** | runtime-clean-check passes | ✅ PASS — exit 0, all checks pass |
| **PROFILE-011** | Service Stopped / Manual | ✅ PASS |
| **PROFILE-012** | Port free, orphans 0, tree clean, stashes empty | ✅ PASS |

## Validation

```powershell
# JSON validation via ConvertFrom-Json
$json = Get-Content config/model-profiles.json -Raw | ConvertFrom-Json
$json.profiles.Count  # 5

# Field presence verified per profile:
#   verified_context   — integer
#   verified_ngl       — integer
#   stability          — string
#   requires_reduced_offload — boolean
#   notes              — string
```

## Hard Constraints Verification

| Constraint | Status |
|------------|--------|
| Router behavior not modified | ✅ Confirmed — no router code touched |
| Service configuration not modified | ✅ Confirmed |
| No model benchmark experiments run | ✅ Confirmed — all values from existing evidence |
| No profile facts invented | ✅ Confirmed — every value has sprint-documented evidence |
| Unknown values recorded honestly | ✅ N/A — all values known; no guessing needed |
| No secrets committed | ✅ Confirmed |
| No binaries, model files, logs, cache committed | ✅ Confirmed |
| Windows anti-loop rules followed | ✅ Confirmed |

## Next Sprint Recommendation

This was the final planned Layer 1 sprint. Future options:

- **ROUTER-CONTRACT-TESTS-1** (Layer 2) — Shared conformance tests for the portable Router contract
- **TIGHTEN-FIT-1** (Layer 1 enhancement) — Test ngl=85/90/95 for the three conditional-stability profiles to find maximum stable offload
- **WIN-LIBRARIAN-APP-PLAN-1** (Layer 3) — Architecture decision for a Windows Librarian client

Layer 1 (Runtime Node Reliability) is now substantially complete with:
- Foundation (8 sprints) ✅
- Runtime Integration + Receipts (4 sprints) ✅
- Runtime Qualification (1 sprint) ✅
- Operator Toolkit (1 sprint) ✅
- Profile Metadata Normalization (1 sprint) ✅

## Final State

| Check | Result |
|-------|--------|
| **Starting HEADs** | TheLibrarian-main: `1e32002`, runtime-node: `d5fa12d` |
| **Final HEADs** | TheLibrarian-main: `1e32002` (unchanged), runtime-node: `[committed]` |
| **Service** | Stopped / Manual ✅ |
| **Port 9130** | Free ✅ |
| **llama-server orphans** | 0 ✅ |
| **rust-router orphans** | 0 ✅ |
| **Stashes** | Empty (both repos) ✅ |
| **Working tree (runtime-node)** | Clean after commit ✅ |
| **Working tree (TheLibrarian-main)** | Clean ✅ |
