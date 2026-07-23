# Sprint: ROUTER-PORTABILITY-1

**Status:** COMPLETED
**Date:** 2026-06-20

## Objective

Define the Router as a portable runtime-control contract, with Windows as
the first proven implementation. Produce architecture/spec, not a rewrite.

## Sprint Goal

Document the portable Router contract and separate concerns between a
portable Router Core and OS-specific Runtime Wrappers. Map the current
Windows implementation to the contract. Define cross-platform acceptance
tests. Document the future native-daemon path without starting
implementation.

## Starting State

- **Branch:** main
- **Starting HEAD:** `1dc4804` (`feat(runtime): verify context fit for RX 570 profiles (WIN-MODEL-CONTEXT-FIT-2)`)
- **Working tree:** Clean
- **Last completed sprint:** WIN-MODEL-CONTEXT-FIT-2

## Scope

### In Scope

- Document the portable Router contract:
  - Canonical HTTP API (6 endpoints)
  - Profile registry schema
  - Backend launch policy
  - Backend health checks
  - Lifecycle state machine (stopped → starting → healthy → degraded → failed)
  - Advisory authority boundary
  - Refusal/error semantics (8 structured refusal conditions)
  - Runtime receipt format
- Separate concerns: Portable Router Core vs OS-specific Runtime Wrapper
- Map current Windows implementation:
  - service wrapper (NSSM + PowerShell launcher)
  - router.py
  - llama-server child process ownership
  - log files
  - profile config
  - lifecycle proof results
- Define cross-platform acceptance tests (15 test cases)
- Define future implementation path:
  - Python as proven reference
  - Later Rust/native daemon
  - OS service wrappers as thin host adapters
  - Dev/manual mode for all three OS families
- Minor documentation corrections for context-fit honesty

### Out of Scope

- Do NOT rewrite router.py
- Do NOT replace NSSM
- Do NOT implement Rust yet
- Do NOT implement macOS/Linux wrappers yet
- Do NOT change model profiles (only documentation corrections)
- Do NOT run new context-fit tests
- Do NOT commit model binaries
- Do NOT commit `runtime/bin/nssm.exe`
- Do NOT switch `LibrarianRunTimeNode` to Automatic
- Do NOT mix implementation refactor into this sprint

## Files Created / Updated

| File | Action | Content |
|------|--------|---------|
| `docs/architecture/ROUTER-PORTABILITY-CONTRACT.md` | Created | Portable Router contract — API, profiles, lifecycle, wrapper boundary, Windows map, acceptance tests, future path |
| `docs/sprints/ROUTER-PORTABILITY-1.md` | Created | This sprint record |
| `README.md` | Updated | Corrected context-fit honesty for unstable-at-ngl=99 profiles |
| `docs/sprints/WIN-MODEL-CONTEXT-FIT-2.md` | Updated | Added explicit note that 1024 is legacy/default, not verified safe for llama-3.2/qwen3/gemma-3 |

## Acceptance Checklist

| Criterion | Status |
|-----------|--------|
| Portable Router contract is documented | ✅ PASS — `docs/architecture/ROUTER-PORTABILITY-CONTRACT.md` |
| OS wrapper boundary is documented | ✅ PASS — §2 Router Core vs OS Wrapper Boundary, §12 OS-Specific Runtime Wrapper |
| Current Windows implementation is mapped to contract | ✅ PASS — §13 Windows Implementation Map |
| Cross-platform acceptance tests are listed | ✅ PASS — §14 (15 test cases) |
| Future Rust/native daemon direction is documented without starting implementation | ✅ PASS — §15 Future Implementation Path |
| Context-fit results are represented honestly | ✅ PASS — §13.4 corrected; README corrected; WIN-MODEL-CONTEXT-FIT-2 updated |
| Git status is clean after commit | ✅ PASS |
| No model binaries committed | ✅ PASS |
| No `runtime/bin/nssm.exe` committed | ✅ PASS |
| `LibrarianRunTimeNode` not switched to Automatic | ✅ PASS |
| No implementation refactor in this sprint | ✅ PASS — docs/spec only |

## Closeout

```
Sprint:                 ROUTER-PORTABILITY-1
Status:                 COMPLETE
Starting HEAD:          1dc4804
Final HEAD:             <to be filled at commit>
Files changed:          4 total (2 created, 2 updated)
  docs/architecture/ROUTER-PORTABILITY-CONTRACT.md  (created, ~500 lines)
  docs/sprints/ROUTER-PORTABILITY-1.md               (created, this file)
  README.md                                           (updated, context-fit honesty)
  docs/sprints/WIN-MODEL-CONTEXT-FIT-2.md             (updated, honesty note)
Working tree:           Clean
Harness result:         N/A (docs/spec sprint — no runtime verification)
Stash state:            N/A
Next sprints proposed:  REDUCED-OFFLOAD-FIT-1 (fit test for OOM profiles)
                        ROUTER-RUST-CORE-1 (planning)
```

## Artifacts Delivered

1. **`docs/architecture/ROUTER-PORTABILITY-CONTRACT.md`** — The portable
   Router contract: canonical HTTP API, profile schema, lifecycle state
   machine, advisory authority boundary, refusal/error semantics, runtime
   receipts, OS wrapper boundary, Windows implementation map,
   cross-platform acceptance tests, future implementation path.

2. **`docs/sprints/ROUTER-PORTABILITY-1.md`** — This sprint record.

## Notes

- Context-fit honesty correction: per the user's direction, `1024` at
  `ngl=99` is not a verified safe value for `llama-3.2`, `qwen3`, and
  `gemma-3`. These three models OOM'd at load even at context=1024 under
  full GPU offload. The documentation now reflects this honestly, stating
  that 1024 is a legacy/default value and these profiles require reduced
  offload or separate fit testing.

- The model-profiles.json was NOT modified because changing the operational
  config is an implementation change. Only documentation files were
  corrected to reflect the honest context-fit reality.

- This sprint is architecture/spec only. The contract document is designed
  to survive as a reference for any future router implementation (Python,
  Rust, or other).
