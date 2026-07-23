# Windows Phase 0 Sprint Sequence

**Status:** Draft
**Date:** 2026-06-29
**Plan ref:** `docs/planning/WIN-AGENT-HARNESS-PLAN.md`
**Roadmap ref:** `docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md`

---

## 1. Purpose

Define the forward sprint sequence for Windows Phase 0 work, incorporating harness development, baseline findings triage, and the transition to the existing Layer 1/2/3 roadmap.

---

## 2. Completed Sprints (Sealed)

| Sprint | HEAD | Status |
|--------|------|--------|
| *(22 prior sprints through WIN-RUNTIME-QUALIFICATION-1)* | `e7cfe33` | ✅ Sealed |
| WIN-RUNTIME-CONTROLLED-ACTIVATION-1 | `08a8602` | ✅ Sealed/pushed |
| WIN-AGENT-HARNESS-ENV-BASELINE-1 | `2895584` | ✅ Sealed/pushed |
| WIN-ORIGIN-AHEAD-RECONCILE-1 | `06768f3` | ✅ Sealed/pushed |

---

## 3. Current Sprint

| Sprint | Status |
|--------|--------|
| **WIN-AGENT-HARNESS-PLAN-1** | **← Creating plan documents** |

---

## 4. Proposed Forward Sequence

### Track A — Harness Implementation

These sprints build the custody sandbox tools defined in the harness plan.

| Order | Sprint | Purpose | Dependencies |
|-------|--------|---------|-------------|
| A1 | WIN-DISK-SPACE-RISK-TRIAGE-1 | Free C: drive space for safe build/test operations | None (can run in parallel with A2) |
| A2 | WIN-PACKET-VALIDATION-HOOK-1 | First harness pre-flight verification hook | Harness plan (§4) |
| A3 | WIN-HARNESS-POSTFLIGHT-1 | Post-flight state verification and receipt generation | A2 |
| A4 | WIN-HARNESS-RECEIPT-TEMPLATE-1 | Standardized sprint receipt templates | A3 |
| A5 | WIN-HARNESS-CONTRACT-RUNNER-1 | Unified contract test runner wrapping existing test scripts | A2 |
| A6 | WIN-SPRINT-LEDGER-1 | Create sprint-ledger.json convention and tooling | A3 |

### Track B — Environment Hardening

These sprints address baseline findings and operational risks.

| Order | Sprint | Purpose | Baseline Finding |
|-------|--------|---------|-----------------|
| B1 | WIN-WINDOWS-UPGRADE-EVAL-1 | Evaluate Windows 11 upgrade for Librarian host role | F-007 |
| B2 | WIN-PATH-HYGIENE-1 | Clean up conflicting PATH entries (Ollama, LM Studio) | F-008 |
| B3 | WIN-MSVCPATH-BASELINE-1 | Document and verify MSVC resolution path for Rust builds | F-003 |

### Track C — Layer 1 Continuation

These sprints continue the existing Layer 1 roadmap from `WINDOWS-PC-SPRINT-ROADMAP.md`.

| Order | Sprint | Purpose | Roadmap Ref |
|-------|--------|---------|-------------|
| C1 | WIN-RUNTIME-OPERATIONS-1 | Operator toolkit scripts | Layer 1, Sprint 1 |
| C2 | WIN-RUNTIME-PROFILES-CLEANUP-1 | Normalize profile metadata | Layer 1, Sprint 2 |

### Track D — Layer 2 & 3 (Future)

| Order | Sprint | Purpose | Roadmap Ref |
|-------|--------|---------|-------------|
| D1 | ROUTER-CONTRACT-TESTS-1 | Shared conformance tests | Layer 2, Sprint 3 |
| D2 | ROUTER-RUST-CORE-1 | Native Router core | Layer 2, Sprint 4 |
| D3 | WIN-LIBRARIAN-APP-PLAN-1 | Windows Librarian app architecture | Layer 3, Sprint 6 |

---

## 5. Execution Order

```
Phase 0 — Completed
═══════════════════════════════════════════
WIN-AGENT-HARNESS-ENV-BASELINE-1          ✅
WIN-ORIGIN-AHEAD-RECONCILE-1              ✅
WIN-AGENT-HARNESS-PLAN-1                  ← CURRENT

Phase 0a — Harness Core
═══════════════════════════════════════════
WIN-PACKET-VALIDATION-HOOK-1              ← Recommended NEXT
WIN-HARNESS-POSTFLIGHT-1
WIN-HARNESS-RECEIPT-TEMPLATE-1
WIN-SPRINT-LEDGER-1

Phase 0b — Parallel Maintenance (non-blocking)
═══════════════════════════════════════════
WIN-DISK-SPACE-RISK-TRIAGE-1              ← Parallel maintenance
WIN-MSVCPATH-BASELINE-1
WIN-PATH-HYGIENE-1

Phase 0c — Layer 1 Operations
═══════════════════════════════════════════
WIN-RUNTIME-OPERATIONS-1
WIN-RUNTIME-PROFILES-CLEANUP-1

Phase 1 — Layer 2/3 Transition
═══════════════════════════════════════════
ROUTER-CONTRACT-TESTS-1
ROUTER-RUST-CORE-1
WIN-LIBRARIAN-APP-PLAN-1
```

---

## 6. Corrections Applied in This Sprint

**F-001 severity revision (see WIN-AGENT-HARNESS-PLAN.md §7 Addendum):**
- Original baseline severity: HIGH (WIN-AGENT-HARNESS-ENV-BASELINE-1)
- Revised planning severity: MEDIUM (effective WIN-AGENT-HARNESS-PLAN-1)
- Reason: Model files, repo, git history, and Rust build artifacts are on G: (132 GB free), not C:
- Result: Disk triage is non-blocking parallel maintenance, not a gate

---

## 7. Baseline Findings Mapping

| Finding | Sprint | Priority |
|---------|--------|----------|
| F-001: C: drive critically low | WIN-DISK-SPACE-RISK-TRIAGE-1 | MEDIUM (revised from HIGH — see §6) |
| F-002: dotnet SDK not found | Blocked until C: drive cleared | MEDIUM (parked) |
| F-003: MSVC not in PATH | WIN-MSVCPATH-BASELINE-1 | LOW |
| F-004: SESSION-HANDOFF.md stale | ✅ Already corrected | — |
| F-005: No sprint-ledger.json | WIN-SPRINT-LEDGER-1 | LOW |
| F-006: Planning docs missing | ✅ This sprint (WIN-AGENT-HARNESS-PLAN-1) | — |
| F-007: Windows 10 22H2 past EOS | WIN-WINDOWS-UPGRADE-EVAL-1 | INFO |
| F-008: PATH clutter | WIN-PATH-HYGIENE-1 | LOW |

---

## 8. Owner Decision Points

The following points require Owner input:

1. **After this sprint:** Approve WIN-PACKET-VALIDATION-HOOK-1 as next sprint?
2. **After WIN-PACKET-VALIDATION-HOOK-1:** Does the pre-flight hook match expectations?
3. **After WIN-DISK-SPACE-RISK-TRIAGE-1 (parallel):** How much space was freed? Is model work now safer?
3. **After WIN-PACKET-VALIDATION-HOOK-1:** Does the pre-flight hook match expectations?
4. **At Phase 0c completion:** Ready for Layer 1 operations, or continue harness hardening?
5. **At Layer 2 transition:** Approve native Router core planning.

---

## 9. Sprint Doc Index

For a complete list of all sprint docs (past, present, and planned), see the `docs/sprints/` directory.

**Sealed sprints:** 26 total (22 prior + WIN-RUNTIME-CONTROLLED-ACTIVATION-1 + WIN-AGENT-HARNESS-ENV-BASELINE-1 + WIN-ORIGIN-AHEAD-RECONCILE-1 + this sprint).
