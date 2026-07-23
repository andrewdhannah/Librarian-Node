# Sprint Specification: WIN-MULTIPLATFORM-LIBRARIAN-PLANNING-1

**Status:** Sealed
**Date:** 2026-07-01
**Phase:** Phase 0b — Parallel Maintenance (Planning)
**Dependencies:** WIN-STARTUP-STATE-RECONCILE-1

---

## 1. Purpose

Create Windows-side planning documents for the multiplatform Librarian architecture, multi-node MCP document custody, Mac rewrite portability impact, and cross-platform algorithm portability. These documents are planning inputs for the later Mac canonical Librarian design sprint.

---

## 2. Background

The Windows PC lane has proven:
- A governed runtime node with 5 model profiles
- A 9-tool harness infrastructure (pre-flight, post-flight, receipts, contract checks, baseline diff, ledger, action receipts)
- A three-link proof chain (source HEAD, artifact hash, governed rebuild)

The next architectural step requires defining how multi-node Librarian authority works across Mac and Windows. This sprint produces the Windows-side inputs to that design. It does not implement MCP tools, does not modify runtime code, and does not create the canonical Mac source-of-truth docs.

---

## 3. Deliverables

| File | Description |
|------|-------------|
| `docs/planning/WIN-MULTIPLATFORM-LIBRARIAN-PLANNING.md` | Windows-side view of the multiplatform Librarian: node roles, authority model, reversible Mac/PC roles |
| `docs/planning/WIN-MULTINODE-MCP-DOCUMENT-CUSTODY-NOTES.md` | PC requirements for MCP document custody: proposal/intake model, lock/lease, stale HEAD detection, proposed MCP tools |
| `docs/planning/WIN-MAC-REWRITE-PORTABILITY-IMPACT.md` | How the Mac rewrite should keep governance algorithms portable: Swift host layer vs portable core |
| `docs/planning/WIN-CROSSPLATFORM-ALGORITHM-PORTABILITY.md` | Portability matrix: what algorithms are portable, what requires OS adapters, schema-driven design |
| `docs/sprints/WIN-MULTIPLATFORM-LIBRARIAN-PLANNING-1.md` | This sprint specification |
| `docs/receipts/WIN-MULTIPLATFORM-LIBRARIAN-PLANNING-1-RECEIPT.md` | Sprint closeout receipt |

---

## 4. Allowed Mutation Scope

| Path | Action |
|------|--------|
| `docs/planning/WIN-MULTIPLATFORM-LIBRARIAN-PLANNING.md` | **Create** |
| `docs/planning/WIN-MULTINODE-MCP-DOCUMENT-CUSTODY-NOTES.md` | **Create** |
| `docs/planning/WIN-MAC-REWRITE-PORTABILITY-IMPACT.md` | **Create** |
| `docs/planning/WIN-CROSSPLATFORM-ALGORITHM-PORTABILITY.md` | **Create** |
| `docs/sprints/WIN-MULTIPLATFORM-LIBRARIAN-PLANNING-1.md` | **Create** |
| `docs/receipts/WIN-MULTIPLATFORM-LIBRARIAN-PLANNING-1-RECEIPT.md` | **Create** |
| `project-state/sprint-ledger.json` | **Update** — add sprint entry, update current_head |
| `SESSION-HANDOFF.md` | **Update** — HEAD ref, last sealed sprint |

---

## 5. Forbidden Actions

- No service start/stop
- No model workload
- No runtime/router/model code changes
- No MCP implementation
- No Swift implementation
- No Python implementation
- No Windows app implementation
- No environment repair
- No firewall change
- No auto-start change
- No broad agent autonomy

---

## 6. Required Pre-Sprint Checks

| Check | Expected |
|-------|----------|
| HEAD matches expected baseline | `05aabee` |
| Working tree clean | ✅ |
| `origin/main` in sync | ✅ |
| `SESSION-HANDOFF.md` reads correctly | ✅ |
| `validate-sprint-ledger.ps1` passes | ✅ |
| All 7 harness scripts parse cleanly | ✅ |
| `pre-mutation-check.ps1` passes | ✅ 11/11 |

---

## 7. Acceptance Gates

| Gate | Description | Result |
|------|-------------|--------|
| G-001 | All 4 planning docs created | ✅ |
| G-002 | Each doc has explicit purpose, scope, non-goals, and Mac-canonical follow-up | ✅ |
| G-003 | Sprint doc and receipt created | ✅ |
| G-004 | Node role model defined with 7 role types | ✅ |
| G-005 | Reversible Mac/PC roles documented | ✅ |
| G-006 | MCP document custody requirements and 13 proposed tools documented | ✅ |
| G-007 | Mac rewrite portability impact and architecture guidance documented | ✅ |
| G-008 | Cross-platform portability matrix documented (11 portable, 4 OS-specific) | ✅ |
| G-009 | No service/model/runtime/environment files changed | ✅ |
| G-010 | sprint-ledger validates after update | ✅ |
| G-011 | next_authorized_sprint is correct | ✅ |

---

## 8. Corrected State

```
HEAD:        05aabee (WIN-STARTUP-STATE-RECONCILE-1)
Working tree: 4 new planning docs + sprint doc + receipt + ledger/handoff updates

next_authorized_sprint: WIN-HARNESS-CUSTODY-LEDGER-1
```

---

## 9. Owner Approval

**APPROVE_AND_SEAL** — 2026-07-01
