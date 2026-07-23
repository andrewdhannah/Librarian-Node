# WIN-AGENT-HARNESS-PLAN-1

**Status:** ACTIVE — IN PROGRESS
**Previous sprint:** WIN-AGENT-HARNESS-ENV-BASELINE-1 (SEALED)
**Previous sprint:** WIN-ORIGIN-AHEAD-RECONCILE-1 (SEALED)
**Repo:** `librarian-runtime-node`
**Platform:** Windows (PowerShell 5.1)
**Date:** 2026-06-29

---

## Sprint Summary

Create the missing Windows agent-harness and Windows Librarian host planning documents using the completed `WIN-AGENT-HARNESS-ENV-BASELINE-1` evidence.

---

## Scope

### In Scope
- `docs/planning/WIN-AGENT-HARNESS-PLAN.md` — Overall harness architecture and component boundaries
- `docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md` — Three-layer custody model (mechanical, policy, authority)
- `docs/planning/WIN-HARNESS-PARITY-ROADMAP.md` — Parity targets with Mac-side verification tooling
- `docs/planning/WIN-LIBRARIAN-HOST-OPTIONS.md` — Host technology options (Rust/Tauri, Electron, .NET, Python web, WinUI)
- `docs/planning/WIN-SPRINT-SEQUENCE.md` — Forward sprint sequence incorporating harness work
- `docs/sprints/WIN-AGENT-HARNESS-PLAN-1.md` — This sprint doc
- Closeout receipt

### Out of Scope (Do Not)
- No service start or stop
- No model workload
- No runtime/router/model code change
- No firewall change
- No auto-start change
- No app implementation
- No environment repair
- Do not fix any baseline finding — classify and create follow-up recommendations only

---

## Starting Baseline

| Check | Value |
|-------|-------|
| Repo | `G:\OpenWork\librarian-runtime-node` |
| Branch | `main` |
| HEAD | `06768f3` — `WIN-ORIGIN-AHEAD-RECONCILE-1 receipt` |
| Working tree | Clean ✅ |
| Origin | Up to date ✅ |

---

## Durable State Verification (pre-work)

1. ✅ HEAD matches `06768f3`
2. ✅ git status is clean
3. ✅ origin/main is in sync (0 ahead, 0 behind)
4. ✅ SESSION-HANDOFF.md is current
5. ✅ `docs/receipts/WIN-AGENT-HARNESS-ENV-BASELINE-1-RECEIPT.md` exists
6. ✅ `docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md` exists
7. ✅ `docs/receipts/WIN-ORIGIN-AHEAD-RECONCILE-1-RECEIPT.md` exists

---

## Acceptance Gates

| Gate | Description |
|------|-------------|
| PL-001 | WIN-AGENT-HARNESS-PLAN.md created with component inventory and gating risks |
| PL-002 | WIN-CUSTODY-SANDBOX-MODEL.md created with three-layer model and boundaries |
| PL-003 | WIN-HARNESS-PARITY-ROADMAP.md created with tiered parity targets |
| PL-004 | WIN-LIBRARIAN-HOST-OPTIONS.md created with constraint-aware survey |
| PL-005 | WIN-SPRINT-SEQUENCE.md created with fork point and finding mapping |
| PL-006 | All 8 baseline findings treated as constraints, not tasks |
| PL-007 | Working tree clean at closeout (only new planning/sprint/receipt files) |
| PL-008 | No environment repairs performed |
| PL-009 | Receipt/evidence file emitted |
| PL-010 | Recommended next sprint documented |

---

## Baseline Findings Reference

All 8 findings from WIN-AGENT-HARNESS-ENV-BASELINE-1 must be explicitly addressed in the plan set:

| Finding | Treatment in Plan Set |
|---------|----------------------|
| F-001: C: drive critically low | Classified as gating risk for model/stability sprints. Addressed in harness plan (§7), host options (§6), sprint sequence (fork point §6) |
| F-002: dotnet SDK not found | Documented in harness plan (§7) and host options as blocked until disk space cleared. |
| F-003: MSVC not in PATH | Referred to WIN-MSVCPATH-BASELINE-1 in sprint sequence |
| F-004: SESSION-HANDOFF.md stale | ✅ Already corrected prior to this sprint |
| F-005: No sprint-ledger.json | Referred to WIN-SPRINT-LEDGER-1 in sprint sequence |
| F-006: 5 planning docs missing | ✅ This sprint creates them |
| F-007: Windows 10 22H2 past EOS | Documented as operational risk in harness plan (§7), referred to WIN-WINDOWS-UPGRADE-EVAL-1 |
| F-008: PATH clutter | Referred to WIN-PATH-HYGIENE-1 in sprint sequence |

---

## Next-Session Suggestion

After this sprint, the recommended next sprint is:

**WIN-PACKET-VALIDATION-HOOK-1** — Implement the first harness pre-flight verification hook.

**Rationale:** F-001 severity was revised from HIGH to MEDIUM after correcting the storage map (model files and build artifacts are on G:, not C:). Disk-space triage is deferred to a parallel maintenance track and does not block harness implementation.

See `docs/planning/WIN-SPRINT-SEQUENCE.md` §6 for the revision record.
