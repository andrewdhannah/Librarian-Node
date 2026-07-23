# WIN-RUST-TOOLCHAIN-DRIFT-TRIAGE-1

**Status:** ACTIVE — IN PROGRESS
**Previous sprint:** WIN-HARNESS-BASELINE-DIFF-1 (RATIFIED)
**Repo:** `librarian-runtime-node`
**Platform:** Windows (PowerShell 5.1)
**Date:** 2026-06-30

---

## Sprint Summary

Read-only investigation into the `rust_version NOT_FOUND` drift detected by
`baseline-diff.ps1`. The frozen baseline at
`docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md` recorded
`rustc 1.96.0` at `C:\Users\andre\.cargo\bin\rustc.exe`, but the baseline-diff
tool reports `NOT_FOUND`. Determine whether Rust is truly missing, or merely
unavailable from the current PowerShell session PATH.

No installation, no PATH repair, no environment variable modification.

---

## Scope

### In Scope
- `docs/sprints/WIN-RUST-TOOLCHAIN-DRIFT-TRIAGE-1.md` — This sprint doc
- Investigation findings in this conversation (no script written unless findings justify a triage script)

### Investigation points
1. `rustc` and `cargo` in current `$env:PATH`
2. `.cargo\bin` directory existence
3. Rust via rustup (if installed)
4. Rust via Visual Studio / MSVC toolchain
5. Alternative shell profiles (cmd, Developer Command Prompt)
6. Windows Registry detection (if any)
7. Previous build artifacts that would confirm Rust was present

### Out of Scope (Do Not)
- No Rust installation
- No PATH repair
- No environment modification
- No toolchain download
- No `rustup` install
- No service/model/runtime changes

---

## Starting Baseline

| Check | Value |
|-------|-------|
| Repo | `G:\OpenWork\librarian-runtime-node` |
| Branch | `main` |
| HEAD | `bacfbba` — `docs: record WIN-HARNESS-BASELINE-DIFF-1 process correction` |
| Working tree | Clean |
| Origin | Up to date |

---

## Investigation Procedure

1. Query `rustc --version` and `cargo --version` from current shell
2. Search `$env:PATH` for cargo/rust directories
3. Check standard install locations:
   - `$env:USERPROFILE\.cargo\bin\`
   - `$env:PROGRAMFILES\Rust\`
   - `$env:LOCALAPPDATA\Programs\`
4. Check rustup presence: `where.exe rustup`, `$env:RUSTUP_HOME`
5. Check MSVC toolchain for embedded Rust
6. Check Windows Registry for Rust uninstall entries
7. Check previous build evidence in `rust-router/target/`
8. Summarize findings

---

## Acceptance Gates

| Gate | Description |
|------|-------------|
| RT-001 | Rust availability determined (present/absent/partial) |
| RT-002 | If present, exact version(s) and path(s) recorded |
| RT-003 | If absent, mechanism of prior presence explained |
| RT-004 | No Rust toolchain files installed or modified |
| RT-005 | No PATH or environment changes |
| RT-006 | Findings presented for Owner review before commit |

---

## Boundary Adherence

| Boundary | Status |
|----------|--------|
| Read-only investigation only | Enforced |
| No Rust installation | Enforced |
| No PATH/environment repair | Enforced |
| No service/model/runtime | Enforced |
| No commit before Owner review | Enforced (process correction) |
