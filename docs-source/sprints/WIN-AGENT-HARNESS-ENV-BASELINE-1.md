# WIN-AGENT-HARNESS-ENV-BASELINE-1

**Status:** ACTIVE — IN PROGRESS
**Previous sprint:** WIN-RUNTIME-CONTROLLED-ACTIVATION-1 (PROMOTED)
**Repo:** `librarian-runtime-node`
**Platform:** Windows (PowerShell 5.1)
**Date:** 2026-06-29

---

## Sprint Summary

Create a read-only Windows agent-host environment baseline for future governed harness work and Windows Librarian host preparation.

This sprint inventories the entire machine environment — OS, hardware, toolchain, runtime state, network profile, service configuration — and records findings for follow-up sprints.

**No mutations beyond sprint documentation, baseline report, and receipt/evidence files.**

---

## Scope

### In Scope
- Read-only inventory of all environment dimensions listed in the baseline checklist
- Machine-identity, CPU, RAM, GPU/VRAM, disk, network
- PowerShell, Git, Python, Node, Rust, Visual Studio/MSVC/build tools
- PATH and key environment variables
- Service state for `LibrarianRunTimeNode`
- Port state for 9120–9125 and 9130
- Orphan process audit
- Existing harness/check script inventory
- Repo location verification
- Allowed writable workspace paths
- Forbidden/secret-risk path identification

### Out of Scope (Do Not)
- Do not start any service or process
- Do not stop any service or process
- Do not run any model workload
- Do not change any runtime/router/model code
- Do not change any firewall rule
- Do not change any auto-start configuration
- Do not implement any app feature
- Do not grant broad agent autonomy
- Do not fix any discovered issue — record as finding only

---

## Starting Baseline

| Check | Value |
|-------|-------|
| Repo | `G:\OpenWork\librarian-runtime-node` |
| Branch | `main` |
| HEAD | `08a8602` |
| Subject | `docs(sprint): close WIN-RUNTIME-CONTROLLED-ACTIVATION-1 — PROMOTE` |
| Working tree | Clean ✅ |
| Ahead of origin | 20 commits |
| Remote | `https://github.com/andrewdhannah/librarian-runtime-node.git` |

---

## Sprint Boundary

This is a **read-only inventory and documentation sprint**. It is not:
- a service lifecycle sprint
- a router implementation sprint
- a model routing policy sprint
- a new feature sprint
- a bug-fix sprint
- a code mutation sprint

---

## Deliverables

| Path | Description |
|------|-------------|
| `docs/sprints/WIN-AGENT-HARNESS-ENV-BASELINE-1.md` | This sprint document |
| `docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md` | Full baseline report with all inventory data |
| `docs/receipts/WIN-AGENT-HARNESS-ENV-BASELINE-1-RECEIPT.md` | Sprint receipt/closeout evidence |

---

## Acceptance Gates

| Gate | Description |
|------|-------------|
| BL-001 | All inventory dimensions collected into baseline report |
| BL-002 | All findings recorded without repair (read-only discipline) |
| BL-003 | No service start/stop performed |
| BL-004 | No model workload executed |
| BL-005 | No runtime/router/model code changed |
| BL-006 | No firewall or auto-start changes |
| BL-007 | Working tree remains clean at closeout |
| BL-008 | Receipt/evidence file emitted |
| BL-009 | Recommended next sprint documented |

---

## Findings Expected

Any missing tool, bad PATH entry, suspicious config, or environmental concern discovered during inventory should be recorded as a **finding** in the baseline report, with a recommended follow-up sprint, NOT repaired in this sprint.

---

## Next-Session Prompt (WIN-AGENT-HARNESS-PLAN-1)

```
Open Windows Phase 0 sprint: WIN-AGENT-HARNESS-PLAN-1.

Repo root:
G:\OpenWork\librarian-runtime-node

Goal:
Create the missing Windows agent-harness and Windows Librarian host planning
documents using the completed WIN-AGENT-HARNESS-ENV-BASELINE-1 evidence.

First verify durable repo state, not memory:
- current HEAD
- git status
- SESSION-HANDOFF.md
- docs/sprints/WIN-AGENT-HARNESS-ENV-BASELINE-1.md
- docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md
- docs/receipts/WIN-AGENT-HARNESS-ENV-BASELINE-1-RECEIPT.md
- docs/sprints/WIN-RUNTIME-CONTROLLED-ACTIVATION-1.md
- docs/receipts/WIN-RUNTIME-CONTROLLED-ACTIVATION-1-RECEIPT.md

Boundaries:
Planning/docs only.
No service start.
No service stop.
No model workload.
No runtime/router/model code change.
No firewall change.
No auto-start change.
No app implementation.
No broad agent autonomy.
No environment repair.
Do not fix baseline findings in this sprint; classify them and create
follow-up recommendations only.

Deliver:
- docs/planning/WIN-AGENT-HARNESS-PLAN.md
- docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md
- docs/planning/WIN-HARNESS-PARITY-ROADMAP.md
- docs/planning/WIN-LIBRARIAN-HOST-OPTIONS.md
- docs/planning/WIN-SPRINT-SEQUENCE.md
- docs/sprints/WIN-AGENT-HARNESS-PLAN-1.md
- receipt if repo convention requires it
- explicit treatment of all 8 baseline findings
- recommended next sprint
```

**Key planning constraints from the baseline:**
- **F-001 (HIGH):** C: drive has only 10.2 GB free (9.2%). Classify as a gating risk before any model workload, long-running stability test, or large build/test cache. Does not block docs/planning.
- **F-007 (INFO):** Windows 10 22H2 past EOS (October 2025). Document as operational risk. Does not block local Phase 0 planning.

**After this sprint, expected fork:**
- WIN-DISK-SPACE-RISK-TRIAGE-1 (clear C: drive risk first), OR
- WIN-PACKET-VALIDATION-HOOK-1 (continue harness implementation)
