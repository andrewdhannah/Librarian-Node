# WIN-HARNESS-BASELINE-DIFF-1

**Status:** ACTIVE — IN PROGRESS
**Previous sprint:** WIN-HARNESS-CONTRACT-RUNNER-1 (SEALED)
**Repo:** `librarian-runtime-node`
**Platform:** Windows (PowerShell 5.1)
**Date:** 2026-06-30

---

## Sprint Summary

Build a baseline drift detection tool under `scripts/harness/` that compares current
machine environment state against the frozen baseline at
`docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md` and reports deviations
without repairing anything.

The four-tool harness core is now complete. This sprint adds environmental drift
detection — the ability to answer "has anything changed since the baseline was taken?"
before starting runtime/model work.

---

## Scope

### In Scope
- `scripts/harness/baseline-diff.ps1` — Baseline drift detection tool
- `docs/sprints/WIN-HARNESS-BASELINE-DIFF-1.md` — This sprint doc
- `docs/receipts/WIN-HARNESS-BASELINE-DIFF-1-RECEIPT.md` — Closeout receipt
- `SESSION-HANDOFF.md` — Update sprint table

### Baseline comparison sections (read-only)
1. **Service state** (section 20) — LibrarianRunTimeNode Stopped/Manual vs current
2. **Port state** (section 21) — Ports 9120-9125, 9130 free vs current
3. **Orphan process state** (section 22) — No orphans vs current
4. **Disk free space** (section 7) — C: 10.2 GB, G: 132.3 GB vs current
5. **Git state** — HEAD, origin sync, working tree clean vs current
6. **Toolchain versions** (sections 9-14) — PS, git, Python, Node, Rust vs current
7. **Baseline findings** (section 24) — Check if 8 findings still apply

### Out of Scope (Do Not)
- No service start or stop
- No model workload
- No runtime/router/model code change
- No environment repair
- No disk cleanup
- No firewall change
- No auto-start change
- No app work
- No broad agent autonomy
- No path corrections
- No toolchain installations

---

## Starting Baseline

| Check | Value |
|-------|-------|
| Repo | `G:\OpenWork\librarian-runtime-node` |
| Branch | `main` |
| HEAD | `df55713` — `feat(harness): implement WIN-HARNESS-CONTRACT-RUNNER-1 contract runner` |
| Working tree | Clean |
| Origin | Up to date |

---

## Durable State Verification (pre-work)

1. HEAD matches `df55713`
2. git status is clean
3. origin/main is in sync
4. SESSION-HANDOFF.md reads correctly
5. `scripts/harness/pre-mutation-check.ps1` passes (11/11)
6. `scripts/harness/postflight-check.ps1` parses cleanly
7. `scripts/harness/new-sprint-receipt.ps1` parses cleanly
8. `scripts/harness/run-contract-checks.ps1` parses cleanly and -List exits 0
9. `docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md` exists
10. `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md` exists

---

## Baseline Diff Tool Specification

### Purpose

`baseline-diff.ps1` reads the frozen baseline Markdown document, queries current
environment state for each section, and reports drift. It is **read-only** —
it never modifies state.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `-BaselinePath` | string | No | Path to baseline Markdown file. Defaults to `docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md` |
| `-ListSections` | switch | No * | List available comparison sections without running them |
| `-Section` | string[] | No * | Run specific section(s) by name |
| `-All` | switch | No * | Compare all sections |
| `-Json` | switch | No | Emit structured JSON drift report to stdout |
| `-RepoRoot` | string | No | Auto-detected from script location |
| `-Quiet` | switch | No | Suppress human-readable output |
| | | | *One of `-ListSections`, `-Section`, or `-All` is required |

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All compared sections show no drift (or `-ListSections` mode) |
| 1 | Any compared section shows drift, or unknown section name |

### Comparison Sections

| Section Key | Baseline Source | Current Source |
|-------------|----------------|----------------|
| `service_state` | §20 — Service LibrarianRunTimeNode Stopped/Manual | `Get-Service` |
| `port_state` | §21 — Ports 9120-9125, 9130 free | `netstat -ano` |
| `orphan_processes` | §22 — No orphan llama-server, rust-router, python router | `Get-CimInstance Win32_Process` |
| `disk_free_space` | §7 — C: 10.2 GB, G: 132.3 GB | `Get-CimInstance Win32_LogicalDisk` |
| `git_head` | §1 — HEAD 08a8602 | `git rev-parse HEAD` |
| `git_origin` | §1 — Ahead of origin by 20 | `git rev-parse origin/main` |
| `ps_version` | §9 — 5.1.19041.7417 | `$PSVersionTable` |
| `python_version` | §11 — 3.14.3 | `python --version` |
| `node_version` | §12 — 24.14.0 | `node --version` |
| `rust_version` | §13 — 1.96.0 | `rustc --version` |
| `baseline_findings` | §24 — 8 findings | Runtime check of each finding's current validity |

### JSON Output Schema

```json
{
  "tool_id": "WIN-HARNESS-BASELINE-DIFF-1",
  "version": "1.0.0",
  "timestamp": "2026-06-30T00:00:00-04:00",
  "baseline": "docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md",
  "baseline_date": "2026-06-29",
  "summary": {
    "total": 11,
    "matched": 7,
    "drifted": 4,
    "errors": 0,
    "overall": "DRIFT | CLEAN"
  },
  "sections": [
    {
      "key": "service_state",
      "display": "Service State",
      "baseline": "Stopped / Manual",
      "current": "Stopped / Manual",
      "drifted": false,
      "detail": null
    }
  ]
}
```

---

## Acceptance Gates

| Gate | Description | Result |
|------|-------------|--------|
| BD-001 | `scripts/harness/baseline-diff.ps1` exists | |
| BD-002 | Script parses cleanly under PowerShell 5.1 | |
| BD-003 | `-ListSections` mode exits 0 and shows all 11 sections | |
| BD-004 | `-All` mode runs all sections and reports drift correctly | |
| BD-005 | `-Section` with unknown section name exits 1 | |
| BD-006 | JSON output is deterministic with key/drifted detail | |
| BD-007 | No service start/stop performed | |
| BD-008 | No model workload performed | |
| BD-009 | No runtime/router/model code changed | |
| BD-010 | `pre-mutation-check.ps1` still passes on final sealed tree | |
| BD-011 | `postflight-check.ps1` passes with changed-file allowlist | |
| BD-012 | Next sprint documented | |

---

## Boundary Adherence

| Boundary | Status |
|----------|--------|
| Only `scripts/harness/` mutated | `baseline-diff.ps1` |
| Only sprint/receipt docs mutated | sprint doc, receipt, SESSION-HANDOFF.md |
| No service mutation | Enforced by design — never starts/stops services |
| No runtime/model code change | Zero runtime, router, or model files touched |
| No environment repair | Tool is read-only — queries state, reports drift, never modifies |

---

## Usage Notes

### List available comparison sections

```powershell
.\scripts\harness\baseline-diff.ps1 -ListSections
```

### Compare all sections

```powershell
.\scripts\harness\baseline-diff.ps1 -All
```

### Compare specific sections

```powershell
.\scripts\harness\baseline-diff.ps1 -Section service_state,port_state
```

### JSON output (programmatic)

```powershell
.\scripts\harness\baseline-diff.ps1 -All -Json
```

### Quiet JSON for capture

```powershell
$drift = .\scripts\harness\baseline-diff.ps1 -All -Json -Quiet
```

### Integration with contract runner

```powershell
.\scripts\harness\baseline-diff.ps1 -All  # check for drift
.\scripts\harness\run-contract-checks.ps1 -AllSafe  # run validation checks
```

---

## Next-Sprint Suggestion

After this sprint, the recommended next sprint is:

**WIN-SPRINT-LEDGER-1** — Sprint ledger convention. The harness now has pre-flight,
post-flight, receipt generation, contract runner, and baseline drift detection. The
next gap is a formal sprint ledger — a machine-parseable record of all sprints with
their status, HEADs, and findings, enabling automated sprint tracking and audit.

Alternative: **WIN-AGENT-HARNESS-CLEANUP-1** if C: drive space (Finding F-001 — 10.2 GB
critical) needs attention before deeper runtime/model work.

See `docs/planning/WIN-PC-REMAINING-SPRINTS-PLAN.md` for the full remaining sprint map.
