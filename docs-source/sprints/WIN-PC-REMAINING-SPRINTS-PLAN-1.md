# WIN-PC-REMAINING-SPRINTS-PLAN-1

**Status:** ACTIVE — IN PROGRESS
**Previous sprint:** WIN-PACKET-VALIDATION-HOOK-1 (READY FOR SEAL)
**Repo:** `librarian-runtime-node`
**Platform:** Windows (PowerShell 5.1)
**Date:** 2026-06-29

---

## Sprint Summary

Create a Windows-local planning map for the remaining PC readiness sprints. Scope each future sprint, identify required guardrail/profile categories, define Windows-specific constraints, and preserve the future Mac/Librarian canonical guardrail-profile system as a later source of truth.

This sprint does **not** create the canonical guardrail profile system. It only records Windows-local sprint needs and expected guardrail categories.

---

## Scope

### In Scope
- `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md` — Full remaining sprint map across 5 phases, 20 sprints
- `docs/planning/WIN-PC-SPRINT-GUARDRAIL-NEEDS.md` — 13 guardrail categories + 4 profile classifications
- `docs/sprints/WIN-PC-REMAINING-SPRINTS-PLAN-1.md` — This sprint specification
- `docs/receipts/WIN-PC-REMAINING-SPRINTS-PLAN-1-RECEIPT.md` — Closeout receipt

### Out of Scope (Do Not)
- No service start or stop
- No model workload
- No runtime/router/model code change
- No harness implementation (no script creation)
- No environment repair
- No firewall change
- No auto-start change
- No app implementation
- No canonical Mac/Librarian guardrail system creation
- Do not modify existing planning docs from WIN-AGENT-HARNESS-PLAN-1
- Do not modify existing harness scripts
- Do not modify pre-mutation-check.ps1

---

## Starting Baseline

| Check | Value |
|-------|-------|
| Repo | `G:\OpenWork\librarian-runtime-node` |
| Branch | `main` |
| HEAD | `7cc7d10` — `WIN-AGENT-HARNESS-PLAN-1 planning docs` |
| Working tree | See Durable State Verification below |
| Origin | Up to date (0 ahead, 0 behind) |

---

## Durable State Verification

| # | Check | Expected | Finding |
|---|-------|----------|---------|
| 1 | HEAD matches `7cc7d10` | `7cc7d10` | ✅ `7cc7d10` — WIN-AGENT-HARNESS-PLAN-1 planning docs |
| 2 | git status is clean | clean | ⚠️ SESSION-HANDOFF.md modified (handoff update from PACKET-VALIDATION-HOOK-1); untracked files from WIN-PACKET-VALIDATION-HOOK-1 (sprint doc, receipt, pre-mutation-check.ps1) |
| 3 | origin/main is in sync | 0 ahead, 0 behind | ✅ 0 ahead, 0 behind |
| 4 | SESSION-HANDOFF.md reads correctly | current | ✅ Updated for WIN-PACKET-VALIDATION-HOOK-1 handoff |
| 5 | WIN-AGENT-HARNESS-PLAN.md | exists | ✅ Exists |
| 6 | WIN-CUSTODY-SANDBOX-MODEL.md | exists | ✅ Exists |
| 7 | WIN-HARNESS-PARITY-ROADMAP.md | exists | ✅ Exists |
| 8 | WIN-LIBRARIAN-HOST-OPTIONS.md | exists | ✅ Exists |
| 9 | WIN-SPRINT-SEQUENCE.md | exists | ✅ Exists |
| 10 | WIN-AGENT-HARNESS-PLAN-1-RECEIPT.md | exists | ✅ Exists |
| 11 | WIN-PACKET-VALIDATION-HOOK-1 receipt | PASS | ✅ Receipt exists — CLOSED — READY FOR SEAL |
| 12 | pre-mutation-check.ps1 exists | exists | ✅ `scripts/harness/pre-mutation-check.ps1` — 11 checks |

**Note:** Working tree has 3 untracked files from the completed WIN-PACKET-VALIDATION-HOOK-1 sprint (sprint doc, receipt, pre-mutation-check.ps1) and a modified SESSION-HANDOFF.md. These are pre-existing from the prior sprint and do not conflict with this planning-only sprint. All mutations to establish this sprint's baseline are documented closeout state only.

---

## Deliverables

### Planning Documents Created

| File | Description |
|------|-------------|
| `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md` | Full remaining sprint map: 20 sprints across 5 phases, each with purpose, dependencies, scope boundaries, guardrail categories, preflight checks, acceptance gates, closeout requirements, next-sprint recommendation |
| `docs/planning/WIN-PC-SPRINT-GUARDRAIL-NEEDS.md` | 13 guardrail categories (G-001 through G-013), 4 profile classifications (P-001 through P-004), guardrail-to-sprint dependency matrix, non-canonical status declaration |

### Sprint Document Created

| File | Description |
|------|-------------|
| `docs/sprints/WIN-PC-REMAINING-SPRINTS-PLAN-1.md` | This file — sprint specification |

### Receipt Created

| File | Description |
|------|-------------|
| `docs/receipts/WIN-PC-REMAINING-SPRINTS-PLAN-1-RECEIPT.md` | Closeout receipt |

---

## Acceptance Gates

| Gate | Description |
|------|-------------|
| RP-001 | `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md` created with all 20 sprint entries |
| RP-002 | Each sprint entry includes: ID, purpose, phase, dependencies, allowed mutation scope, forbidden actions, expected guardrail categories, required preflight checks, acceptance gates, closeout requirements, recommended next sprint |
| RP-003 | `docs/planning/WIN-PC-SPRINT-GUARDRAIL-NEEDS.md` created with all guardrail categories |
| RP-004 | Each guardrail category includes: Layer mapping, purpose, Windows constraints, implementation status, canonical mapping |
| RP-005 | Windows-specific constraints documented (13 constraints, C-001 through C-013) |
| RP-006 | Canonical Mac/Librarian guardrail-profile system explicitly declared as out of scope |
| RP-007 | F-001 revised interpretation (C: drive = MEDIUM risk) preserved from WIN-AGENT-HARNESS-PLAN-1 |
| RP-008 | No service start/stop performed |
| RP-009 | No runtime/router/model code changed |
| RP-010 | No harness implementation performed (no scripts created) |
| RP-011 | No canonical guardrail system created |
| RP-012 | Receipt/evidence file emitted |
| RP-013 | Recommended next sprint documented |

---

## Boundary Adherence

| Boundary | Status |
|----------|--------|
| Only `docs/planning/`, `docs/sprints/`, `docs/receipts/` mutated | ✅ Two new planning docs, one sprint doc, one receipt |
| No service mutation | ✅ Enforced — no service commands executed |
| No runtime/model code change | ✅ Zero runtime, router, or model files touched |
| No harness implementation | ✅ No scripts created or modified |
| No environment repair | ✅ No repairs performed |
| No canonical guardrail system | ✅ Explicitly deferred |
| Pre-mutation hook preserved | ✅ Not modified |

---

## Hard Constraints

| Constraint | Status |
|------------|--------|
| No service start | ✅ |
| No service stop | ✅ |
| No model workload | ✅ |
| No runtime/router/model code change | ✅ |
| No firewall change | ✅ |
| No auto-start change | ✅ |
| No app implementation | ✅ |
| No harness implementation | ✅ |
| No canonical Mac/Librarian guardrail system | ✅ |
| All baseline findings classified, not repaired | ✅ |
| F-001 treated as MEDIUM (revised interpretation) | ✅ |
| Planning/docs only sprint | ✅ |

---

## Recommended Next Sprint

**WIN-HARNESS-POSTFLIGHT-1** — Build post-flight state verification and receipt generation for the harness.

**Rationale:** The pre-mutation hook (WIN-PACKET-VALIDATION-HOOK-1) is complete. The custody sandbox model (`docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md`) defines a pre-flight + post-flight cycle. The pre-flight gate exists; the post-flight gate is the natural next step to complete the cycle. This aligns with the recommended next sprint from both the WIN-PACKET-VALIDATION-HOOK-1 receipt and the Phase 0a sequence defined in `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md`.

**Alternative:** WIN-HARNESS-RECEIPT-TEMPLATE-1 if post-flight template alignment is desired first.

See `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md` §4 (S-01) and `docs/planning/WIN-SPRINT-SEQUENCE.md` §4 Track A.

---

## Suggested Session Prompt

```
# WIN-PC-REMAINING-SPRINTS-PLAN-1 — Windows PC Remaining Sprints Planning Map

Repo root: G:\OpenWork\librarian-runtime-node
HEAD: 7cc7d10

Verify durable state:
1. HEAD matches 7cc7d10
2. git status is clean (except pre-existing WIN-PACKET-VALIDATION-HOOK-1 files)
3. origin/main is in sync
4. All existing planning docs present

Create:
- docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md
- docs/planning/WIN-PC-SPRINT-GUARDRAIL-NEEDS.md
- docs/sprints/WIN-PC-REMAINING-SPRINTS-PLAN-1.md
- docs/receipts/WIN-PC-REMAINING-SPRINTS-PLAN-1-RECEIPT.md

Boundaries:
- Planning/docs only. No service, model, code, or harness implementation.
- No canonical Mac/Librarian guardrail system creation.
- F-001 = MEDIUM risk per revised interpretation.
```
