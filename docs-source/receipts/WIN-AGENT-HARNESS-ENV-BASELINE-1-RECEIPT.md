# Closeout Receipt: WIN-AGENT-HARNESS-ENV-BASELINE-1

**Status:** CLOSED — PROMOTE
**Date:** 2026-06-29
**Previous sprint:** WIN-RUNTIME-CONTROLLED-ACTIVATION-1 (PROMOTED)

---

## Summary

Read-only Windows agent-host environment baseline for future governed harness work and Windows Librarian host preparation.

Full machine inventory collected across 24 dimensions. All findings recorded without repair (read-only discipline maintained).

**Result: PASS** — all acceptance gates met.

---

## Pre-Work Baseline

| Check | Expected | Actual | Pass |
|-------|----------|--------|------|
| HEAD | Current | `08a8602` | ✅ |
| Working tree | clean | clean | ✅ |
| Service state | Stopped / Manual | Stopped / Manual | ✅ |
| Orphan llama-server.exe | 0 | 0 | ✅ |
| Orphan rust-router.exe | 0 | 0 | ✅ |
| Port 9130 | FREE | FREE | ✅ |
| Ports 9120–9125 | FREE | FREE | ✅ |

---

## Inventory Dimensions Collected

| # | Dimension | Status |
|---|-----------|--------|
| 1 | Windows version/build/edition | ✅ Collected |
| 2 | Machine identity | ✅ Collected |
| 3 | CPU topology | ✅ Collected |
| 4 | RAM | ✅ Collected |
| 5 | GPU/VRAM | ✅ Collected |
| 6 | Disks and free space | ✅ Collected |
| 7 | Network profile and local IP | ✅ Collected |
| 8 | PowerShell version and execution policy | ✅ Collected |
| 9 | Git version/config (line endings, hooks) | ✅ Collected |
| 10 | Python version | ✅ Collected |
| 11 | Node/npm version | ✅ Collected |
| 12 | Rust/Cargo version | ✅ Collected |
| 13 | Visual Studio/MSVC/build tools | ✅ Collected |
| 14 | PATH summary | ✅ Collected |
| 15 | Key environment variables (non-secret) | ✅ Collected |
| 16 | Repo locations | ✅ Collected |
| 17 | Allowed writable workspace paths | ✅ Collected |
| 18 | Forbidden/secret-risk paths | ✅ Collected |
| 19 | Service state | ✅ Collected |
| 20 | Port state (9120–9125, 9130) | ✅ Collected |
| 21 | Orphan process state | ✅ Collected |
| 22 | Existing harness/check scripts | ✅ Collected |
| 23 | Durable state (HEAD, status, handoff) | ✅ Verified |
| 24 | Existing planning docs inventory | ✅ Completed |

---

## Findings Summary

| # | Severity | Title |
|---|----------|-------|
| F-001 | HIGH | C: Drive critically low on space (10.2 GB / 9.2%) |
| F-002 | MEDIUM | dotnet SDK not found |
| F-003 | LOW | MSVC compiler not in PATH |
| F-004 | LOW | SESSION-HANDOFF.md stale |
| F-005 | LOW | No FEATURE-STATUS.md or sprint-ledger.json |
| F-006 | MEDIUM | Multiple planning docs missing |
| F-007 | INFO | Windows 10 22H2 past EOS (October 2025) |
| F-008 | LOW | Multiple Ollama/LM Studio paths in PATH |

**Zero findings repaired during this sprint.** Read-only discipline maintained.

---

## Files Changed

| File | Change | Scope |
|------|--------|-------|
| `docs/sprints/WIN-AGENT-HARNESS-ENV-BASELINE-1.md` | Create | Sprint doc |
| `docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md` | Create | Baseline report |
| `docs/receipts/WIN-AGENT-HARNESS-ENV-BASELINE-1-RECEIPT.md` | Create | This receipt |

---

## Acceptance Gates

| Gate | Description | Result |
|------|-------------|--------|
| BL-001 | All inventory dimensions collected | ✅ PASS |
| BL-002 | All findings recorded without repair | ✅ PASS |
| BL-003 | No service start/stop performed | ✅ PASS |
| BL-004 | No model workload executed | ✅ PASS |
| BL-005 | No runtime/router/model code changed | ✅ PASS |
| BL-006 | No firewall or auto-start changes | ✅ PASS |
| BL-007 | Working tree clean at closeout | ✅ PASS |
| BL-008 | Receipt/evidence file emitted | ✅ PASS |
| BL-009 | Recommended next sprint documented | ✅ PASS |

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
| No broad agent autonomy | ✅ |
| No mutation outside sprint docs, baseline, and receipt files | ✅ |
| Findings recorded, not repaired | ✅ |
| Follow Windows anti-loop rules | ✅ |

---

## Closeout State

| Check | Value |
|-------|-------|
| HEAD | `08a8602` |
| Working tree | Clean ✅ |
| Service | Stopped / Manual ✅ |
| Orphan processes | 0 ✅ |
| Ports 9120–9130 | All free ✅ |

---

## Recommended Next Sprint

**WIN-AGENT-HARNESS-PLAN-1** — Create governing plan documents for the agent harness work lane:
1. `docs/planning/WIN-AGENT-HARNESS-PLAN.md` — overall harness architecture
2. `docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md` — custody sandbox model
3. `docs/planning/WIN-HARNESS-PARITY-ROADMAP.md` — Mac parity targets
4. `docs/planning/WIN-LIBRARIAN-HOST-OPTIONS.md` — host technology options
5. `docs/planning/WIN-SPRINT-SEQUENCE.md` — forward sprint sequence

---

**Receipt generated:** 2026-06-29
**Closing HEAD:** `08a8602`
