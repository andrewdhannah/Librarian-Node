# Sprint Specification: WIN-RUST-PATH-RESTORE-1

**Status:** Active
**Date:** 2026-06-30
**Phase:** Phase 0b — Parallel Maintenance
**Dependencies:** WIN-RUST-TOOLCHAIN-DRIFT-TRIAGE-1, WIN-SPRINT-LEDGER-1

---

## 1. Purpose

Restore the missing `%USERPROFILE%\.cargo\bin\` rustup proxy shim directory so that `rustc`, `cargo`, and related Rust tooling are accessible from the shell PATH.

This clears the only actionable baseline drift finding (rust_version NOT_FOUND) identified in WIN-RUST-TOOLCHAIN-DRIFT-TRIAGE-1 and confirmed by `baseline-diff -Section rust_version`.

---

## 2. Background

The WIN-RUST-TOOLCHAIN-DRIFT-TRIAGE-1 read-only investigation found:

- Rust toolchain **1.96.0** is intact at:
  `%USERPROFILE%\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\`
- The rustup proxy shim directory **`%USERPROFILE%\.cargo\bin\`** is missing
- `rustup.exe` was also missing (it lives in `.cargo\bin\` in a rustup-managed installation)
- Without the proxy shims, `rustc`/`cargo` are invisible from PATH despite `%USERPROFILE%\.cargo\bin` being in PATH
- The baseline date recorded `rustc 1.96.0`; the actual toolchain binaries match

The sprint creates proxy wrapper `.cmd` scripts in `.cargo\bin\` that delegate to the stable toolchain binaries, restoring PATH-based access without requiring a rustup-init download.

---

## 3. Allowed Mutation Scope

| Path | Action |
|------|--------|
| `%USERPROFILE%\.cargo\bin\` | **Create** directory and proxy wrapper `.cmd` scripts |
| `docs/sprints/WIN-RUST-PATH-RESTORE-1.md` | **Create** — this sprint specification |
| `docs/receipts/WIN-RUST-PATH-RESTORE-1-RECEIPT.md` | **Create** — closeout receipt |
| `SESSION-HANDOFF.md` | Update |
| `project-state/sprint-ledger.json` | Update with new sprint entry |

---

## 4. Forbidden Actions

- No `rust-router` build
- No service start or stop
- No model workload
- No runtime/router code change
- No broad PATH/environment edits beyond restoring Rustup shim behavior
- No Rust installation or repair beyond proxy shim restoration

---

## 5. Acceptance Gates

| Gate | Description | Expected Result |
|------|-------------|-----------------|
| PR-01 | `.cargo\bin\` directory created with proxy shims | PASS |
| PR-02 | `rustc --version` returns 1.96.0 from PATH | PASS |
| PR-03 | `cargo --version` returns 1.96.0 from PATH | PASS |
| PR-04 | `baseline-diff -Section rust_version` reports OK | PASS |
| PR-05 | No pre-existing rust_version drifts remain | PASS |
| PR-06 | No service/model/runtime/environment files changed in repo | PASS |
| PR-07 | `pre-mutation-check.ps1` still passes on final tree | PASS |

---

## 6. Required Preflight Checks

1. HEAD matches `0942096`
2. Working tree is clean
3. `origin/main` is in sync
4. `pre-mutation-check.ps1` passes (11/11)
5. `baseline-diff -Section rust_version` reports NOT_FOUND (drift confirmed)
6. `docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md` exists
7. `project-state/sprint-ledger.json` exists and validates

---

## 7. Closeout Requirements

1. `%USERPROFILE%\.cargo\bin\` exists with proxy shims
2. `rustc` and `cargo` resolve from PATH with version 1.96.0
3. `baseline-diff -Section rust_version` reports OK (drift cleared)
4. Receipt
5. Sprint doc
6. Updated `project-state/sprint-ledger.json`
7. Updated `SESSION-HANDOFF.md`
8. Working tree clean; HEAD unchanged

---

## 8. Recommended Next Sprint

**WIN-HARNESS-ACTION-RECEIPTS-1** — Create granular action receipt generation for discrete harness actions, continuing the Phase 0a/0d harness hardening track.

---

## 9. References

- `docs/sprints/WIN-RUST-TOOLCHAIN-DRIFT-TRIAGE-1.md` (predecessor investigation)
- `docs/receipts/WIN-RUST-TOOLCHAIN-DRIFT-TRIAGE-1-RECEIPT.md`
- `docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md` (§13 rust_version)
- `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md` (§4 S-??? — maintenance)
- `project-state/sprint-ledger.json`
