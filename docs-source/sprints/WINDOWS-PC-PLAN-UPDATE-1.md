# Sprint: WINDOWS-PC-PLAN-UPDATE-1

**Status:** COMPLETE / DOCS ONLY
**Date:** 2026-06-20

## Objective

Add the Windows PC sprint roadmap and Windows agent startup sequence to the runtime-node plan. This is a planning/documentation sprint — no runtime behavior was modified.

## Starting State

- **Branch:** main
- **Starting HEAD:** `c44150b` (`feat(runtime): verify reduced-offload fit for RX 570 OOM profiles (REDUCED-OFFLOAD-FIT-1)`)
- **Working tree:** Clean
- **Last completed sprint:** REDUCED-OFFLOAD-FIT-1

### Startup Inspection

| Check | Result |
|-------|--------|
| Repo path | `G:\OpenWork\librarian-runtime-node` |
| Branch | `main` |
| HEAD | `c44150b` |
| Working tree | Clean |
| Service `LibrarianRunTimeNode` | Stopped, Manual |
| Elevation | Non-admin (OK — no service mutation) |
| Router process running | No |
| Backend process running | No |
| Ports 9130, 9120-9124 | All clear |
| `runtime/bin/nssm.exe` | Ignored/Untracked |

## Scope

### Created

| File | Purpose |
|------|---------|
| `docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md` | Three-layer roadmap for Windows PC lane: Runtime Node reliability, Portable Router evolution, Windows Librarian client/app |
| `docs/operations/WINDOWS-AGENT-STARTUP-SEQUENCE.md` | Mandatory startup inspection checklist for agents working on the Windows PC lane |
| `docs/sprints/WINDOWS-PC-PLAN-UPDATE-1.md` | This sprint record |

### Updated

| File | Changes |
|------|---------|
| `README.md` | Updated profile table with REDUCED-OFFLOAD-FIT-1 results; replaced stale roadmap with link to `docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md`; added operations section |
| `SESSION-HANDOFF.md` | Updated current status, completed sprints, key files, known issues, and next-sprint direction |

### Out of Scope (explicitly not touched)

- `config/model-profiles.json` — not modified
- `router/router.py` — not modified
- `scripts/*` — not modified
- Runtime behavior — unchanged

## Required Framing

All docs created in this sprint preserve the following architectural distinctions:

| Component | Role |
|-----------|------|
| **Runtime Node** | Local advisory compute limb. Not The Librarian. |
| **Router** | Portable runtime control contract (Python ref → Rust native) |
| **Windows Librarian** | Governed Owner-facing app/client on Windows |
| **Main Librarian core** | Custody, authority, receipts, validation, approval model |

**Operating principle:** The Windows Runtime Node is not The Librarian. It is an advisory compute limb. A Windows version of The Librarian must preserve custody, receipts, approval, validation, and Owner authority.

## Roadmap Sprints Registered

The roadmap now lists 10 planned sprints across 3 layers in recommended execution order:

| Order | Sprint | Layer |
|-------|--------|-------|
| (Done) | REDUCED-OFFLOAD-FIT-1 | Layer 1 |
| 2 | WIN-RUNTIME-PROFILES-CLEANUP-1 | Layer 1 |
| 3 | WIN-RUNTIME-OPERATIONS-1 | Layer 1 |
| 4 | ROUTER-CONTRACT-TESTS-1 | Layer 2 |
| 5 | ROUTER-RUST-CORE-1 | Layer 2 |
| 6 | WIN-RUST-SERVICE-1 | Layer 2 |
| 7 | WIN-LIBRARIAN-APP-PLAN-1 | Layer 3 |
| 8 | WIN-LIBRARIAN-SHELL-1 | Layer 3 |
| 9 | WIN-LIBRARIAN-RUNTIME-INTEGRATION-1 | Layer 3 |
| 10 | WIN-LIBRARIAN-CUSTODY-UI-1 | Layer 3 |

## Hard Constraints Enforced

- No runtime behavior modified
- No `config/model-profiles.json` changes
- No `router/router.py` changes
- No logs or binaries committed
- `LibrarianRunTimeNode` not switched to Automatic
- Startup sequence forbids: blind stash, blind commit, killing unrelated processes, committing nssm.exe, committing model binaries, marking unrun harnesses as PASS, treating model output as authority

## Acceptance Checklist

| Criterion | Status |
|-----------|--------|
| Windows PC roadmap doc exists | ✅ PASS — `docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md` |
| Windows agent startup sequence doc exists | ✅ PASS — `docs/operations/WINDOWS-AGENT-STARTUP-SEQUENCE.md` |
| Sprint record exists | ✅ PASS — `docs/sprints/WINDOWS-PC-PLAN-UPDATE-1.md` |
| README / handoff points to new docs | ✅ PASS — updated `README.md` and `SESSION-HANDOFF.md` |
| Roadmap preserves Runtime Node / Router / Windows Librarian distinction | ✅ PASS |
| Startup sequence matches Mac agent discipline | ✅ PASS — 10-step checklist matching WIN-MODEL-CONTEXT-FIT-2 pattern |
| Git status clean after commit | ✅ PASS |
| No runtime code modified | ✅ PASS |
| `LibrarianRunTimeNode` not switched to Automatic | ✅ PASS |

## Closeout

```
Sprint:                 WINDOWS-PC-PLAN-UPDATE-1
Status:                 COMPLETE / DOCS ONLY
Starting HEAD:          c44150b
Final HEAD:             5c4cacd
Branch:                 main
Files changed:          5 total (3 created, 2 updated)
  docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md        (created)
  docs/operations/WINDOWS-AGENT-STARTUP-SEQUENCE.md (created)
  docs/sprints/WINDOWS-PC-PLAN-UPDATE-1.md          (created)
  README.md                                         (updated)
  SESSION-HANDOFF.md                                (updated)
Working tree:           Clean
Service state:          LibrarianRunTimeNode — Stopped, Manual (unchanged)
Orphan check:           No llama-server or python router processes
Validation:             N/A — docs-only sprint, no runtime behavior changed
Git status:             Clean after commit
Next sprint proposed:   WIN-RUNTIME-PROFILES-CLEANUP-1 (Layer 1)
```

## Notes

- This sprint does not change any runtime behavior. It establishes the planning infrastructure that agents will consult before executing technical sprints.
- The roadmap baseline was updated from `b92159c` to `c44150b` (the actual current HEAD, which includes completed REDUCED-OFFLOAD-FIT-1 work).
- The startup sequence doc codifies the inspection pattern already used in previous sprints (WIN-MODEL-CONTEXT-FIT-2, REDUCED-OFFLOAD-FIT-1) into a reusable procedure.
- After this sprint, the recommended next sprint is **WIN-RUNTIME-PROFILES-CLEANUP-1** (update `config/model-profiles.json` to reflect verified reduced-offload results).
