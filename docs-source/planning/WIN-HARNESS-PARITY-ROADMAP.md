# Windows Agent-Harness Parity Roadmap

**Status:** Draft
**Date:** 2026-06-29
**Plan ref:** `docs/planning/WIN-AGENT-HARNESS-PLAN.md`
**Baseline ref:** `docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md`

---

## 1. Purpose

Define how the Windows agent harness achieves functional parity with the Mac-side verification and custody tooling used by The Librarian project.

This is not a rewrite. It is a **target model** — document what the Mac side has, assess what Windows needs, and plan the gap-closing sprints.

---

## 2. What the Mac Side Has (Known Capabilities)

Based on existing repo context and project conventions, the Mac-side environment (`TheLibrarian-main`) provides:

| Capability | Mac Status | Windows Status |
|------------|------------|----------------|
| Swift-based test harness | ✅ Existing (`swift test`) | ❌ Unavailable (no Swift toolchain on Windows) |
| Integration proof script | ✅ `integration_proof.py` | ⚠️ Partial (v2 proof at `run-integration-proof-v2.ps1`) |
| Receipt verifier | ✅ `receipt_verifier.py` (29 checks) | ✅ `verify-receipt.ps1` (48 checks) |
| Receipt schema (v1) | ✅ Shared on disk | ✅ Shared on disk |
| Receipt schema (v2) | ❌ Not present | ✅ `schema-v2.json` (Windows-led improvement) |
| Agent startup sequence | ✅ Mac-specific version | ✅ Windows-specific version exists |
| Operator runbook | ❌ Not present | ✅ `WIN-RUNTIME-OPERATOR-RUNBOOK.md` |
| Custody sandbox | ✅ Implicit (Swift app model) | ❌ Drafted in this plan set |
| Pre-flight automation | ❌ Manual procedure | ❌ Planned (harness component) |
| Sprint doc convention | ✅ Shared | ✅ Shared |
| Receipt convention | ✅ Shared | ✅ Shared |

---

## 3. Parity Targets

### Tier 1 — Must Have (Blocks Safe Agent Work)

| Target | Windows Path | Effort |
|--------|-------------|--------|
| Pre-flight verification script | `scripts/harness/check.ps1` | Medium |
| Post-flight verification script | `scripts/harness/verify.ps1` | Medium |
| Service-state guard (no mutation without approval) | Already documented in runbook | Low |
| Port/orphan baseline check | Already documented in startup sequence | Low |

### Tier 2 — Should Have (Improves Quality of Life)

| Target | Windows Path | Effort |
|--------|-------------|--------|
| Unified receipt template | Sprite receipt generator | Low |
| Sprint closeout checklist automation | Automated receipt generation | Medium |
| Environment drift detection | Baseline diff tool | Medium |
| CI-style test runner for contract tests | `run-router-contract-tests.ps1` | Low |

### Tier 3 — Nice to Have (Long-term Parity)

| Target | Windows Path | Effort |
|--------|-------------|--------|
| Swift test harness equivalent | Not achievable (no Swift). Replace with Python/Rust test runner. | High |
| Mac ↔ Windows cross-platform proof runner | Shared Python proof script | High |
| Shared custody ledger | JSON schema shared between both repos | Medium |

---

## 4. Windows Advantages

The Windows lane has already exceeded the Mac side in some areas:

| Advantage | Detail |
|-----------|--------|
| **v2 receipt schema** | `schema-v2.json` with separate source/artifact/cleanup sections, 48 checks |
| **Operator runbook** | `WIN-RUNTIME-OPERATOR-RUNBOOK.md` — Mac side has no equivalent |
| **Anti-loop rules** | Formalized in `WINDOWS-AGENT-STARTUP-SEQUENCE.md` |
| **Environment baseline** | Machine-wide inventory with 24 dimensions |
| **Orphan/port discipline** | Automated checks before and after every lifecycle test |

---

## 5. Gap-Closing Sequence

| Order | Sprint | Target | Tier |
|-------|--------|--------|------|
| 1 | WIN-PACKET-VALIDATION-HOOK-1 | Pre-flight state verification | T1 |
| 2 | WIN-HARNESS-POSTFLIGHT-1 | Post-flight state verification | T1 |
| 3 | WIN-HARNESS-RECEIPT-1 | Automated receipt generation | T2 |
| 4 | WIN-HARNESS-CONTRACT-RUNNER-1 | Unified contract test runner | T2 |
| 5 | WIN-HARNESS-CROSS-PLATFORM-PROOF-1 | Shared proof scripts | T3 |

---

## 6. Non-Goals

- **Porting Swift code to Windows.** The Swift toolchain is unavailable. Any Mac-side Swift harness must remain Mac-only. Windows parity is achieved through equivalent (not identical) tools.
- **Replacing the Mac-side harness.** Windows parity means Windows agents work as safely as Mac agents, not that both sides use identical code.
- **Forcing shared tooling where platform differences are legitimate.** PowerShell scripts are idiomatic for Windows. Python scripts are idiomatic for cross-platform. Both are acceptable.
