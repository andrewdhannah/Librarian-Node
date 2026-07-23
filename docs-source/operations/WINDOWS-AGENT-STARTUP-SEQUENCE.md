# Windows Agent Startup Sequence

**Status:** Active
**Scope:** Windows PC agent sessions working on Librarian runtime-node or Windows Librarian tasks

---

## Purpose

This document defines the expected startup sequence for agents working on the Windows PC lane.

The goal is to match the discipline of the Mac startup sequence: every agent must establish repository state, environment state, sprint scope, and custody boundaries before making changes.

Eventually this should be handled by The Librarian directly. Until then, agents must run this sequence manually at the start of each sprint/session.

---

## Core Rule

No agent should modify files before completing startup inspection.

Startup inspection must answer:

- What repo am I in?
- What branch and HEAD am I on?
- Is the working tree clean?
- What sprint am I executing?
- What files are in scope?
- What services/processes are already running?
- What must not be touched?
- What validation command applies?

---

## Standard Windows Startup Checklist

### 1. Identify working directory

```powershell
Get-Location
```

**Expected runtime-node repo:**

```
G:\OpenWork\librarian-runtime-node
```

**Expected main Librarian repo:**

```
G:\OpenWork\TheLibrarian-main
```

If the path is unexpected, stop and report.

### 2. Record git state

```powershell
git status --short
git rev-parse --short HEAD
git branch --show-current
git log --oneline -5
```

**Rules:**
- If dirty, classify the dirty files before changing anything.
- Do not stash blindly.
- Do not commit unrelated changes.
- Do not mix sprint scopes.

### 3. Verify runtime-node service state

For runtime-node work:

```powershell
Get-Service LibrarianRunTimeNode -ErrorAction SilentlyContinue
```

Record:
- service exists or missing
- status
- startup type if available
- whether Manual startup is preserved

The service should remain **Manual** unless a sprint explicitly changes that policy.

### 4. Verify elevation when needed

For service control work:

```powershell
([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
  [Security.Principal.WindowsBuiltInRole]::Administrator
)
```

If this returns `False`, do not attempt service registration or service mutation.

### 5. Check active runtime processes

```powershell
Get-CimInstance Win32_Process |
  Where-Object {
    $_.CommandLine -like "*librarian-runtime-node*" -or
    $_.Name -eq "llama-server.exe"
  } |
  Select-Object ProcessId, ParentProcessId, Name, CommandLine
```

**Rules:**
- Do not kill unrelated Python, llama, OpenWork, browser, editor, shell, or system processes.
- Only terminate processes proven to belong to the runtime-node sprint.
- Record PIDs before and after lifecycle tests.

### 6. Check relevant ports

**Router port:**
```powershell
netstat -ano | findstr ":9130"
```

**Known backend ports:**
```powershell
netstat -ano | findstr ":9120"
netstat -ano | findstr ":9121"
netstat -ano | findstr ":9122"
netstat -ano | findstr ":9123"
netstat -ano | findstr ":9124"
```

**Interpretation:**
- `LISTENING` means active process owns the port.
- `TIME_WAIT` is normal TCP cleanup and is not by itself a failed cleanup.
- Record PID for any `LISTENING` entry.

### 7. Verify ignored binaries remain untracked

Runtime-node rule: `runtime/bin/nssm.exe` must remain ignored/untracked.

```powershell
git status --short --ignored
```

**Do not commit:**
- model binaries
- nssm.exe
- local logs
- generated backend logs
- secrets
- machine-local cache files

### 8. Inspect sprint-relevant docs

For runtime-node work, inspect recent sprint docs:

```powershell
Get-ChildItem docs\sprints | Sort-Object LastWriteTime -Descending | Select-Object -First 10
```

Common docs to review:
- `docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md`
- `docs/operations/WINDOWS-AGENT-STARTUP-SEQUENCE.md`
- `docs/architecture/ROUTER-PORTABILITY-CONTRACT.md`
- `docs/architecture/RUNTIME-NODE-ARCHITECTURE.md`
- `docs/sprints/<most-recent-sprint>.md`
- `SESSION-HANDOFF.md`, if present

### 9. Confirm sprint scope

Before changing files, state:

1. sprint name
2. starting HEAD
3. files likely to change
4. files explicitly out of scope
5. acceptance criteria
6. validation command

### 10. Validate after changes

Runtime-node validation depends on sprint type.

**Service/router lifecycle:**
```powershell
Start-Service LibrarianRunTimeNode
Start-Sleep -Seconds 8
Invoke-RestMethod http://127.0.0.1:9130/backend/status
Stop-Service LibrarianRunTimeNode
Start-Sleep -Seconds 8
```

**Router endpoint check:**
```powershell
Invoke-RestMethod http://127.0.0.1:9130/backend/status
Invoke-RestMethod http://127.0.0.1:9130/backend/profiles
Invoke-RestMethod http://127.0.0.1:9130/backend/health
```

**Main Librarian Swift validation:**
```powershell
swift test
```

If Swift is unavailable on Windows, record:
> Harness not run: Swift toolchain unavailable in current Windows environment.
> Manual audit performed.

Do not record as harness `PASS`.

---

## Closeout Requirements

Every Windows agent sprint must report:

| Field | Required |
|-------|----------|
| Repo path | Yes |
| Starting HEAD | Yes |
| Final HEAD | Yes |
| Branch | Yes |
| Files changed | Yes |
| Service state | Yes, if runtime-related |
| Process/orphan check | Yes, if runtime-related |
| Validation command | Yes |
| Validation result | Yes |
| Git status | Yes |
| Actual commit message | Yes |
| Sprint status | Yes |

**Sprint status** must be one of:
- `SEALED`
- `COMPLETE / DOCS ONLY`
- `MANUAL-AUDITED`
- `BLOCKED`
- `NEEDS FIX`

---

## Non-Negotiable Boundaries

Agents must not:

1. commit model binaries
2. commit `nssm.exe`
3. silently change service startup to `Automatic`
4. kill unrelated processes
5. mix unrelated dirty files into a sprint
6. treat model output as authority
7. bypass Owner approval
8. bypass validation
9. mark unrun harnesses as `PASS`
10. call legacy/default settings verified if tests showed OOM or failure

---

## Anti-Loop Rules

These rules exist because Windows/runtime sprints tend to combine many failure modes at once: cross-repo state, service lifecycle, process cleanup, auth setup, temporary secrets, Python/Rust overlap, long-running agent execution, and proof scripts that can themselves mutate. Without explicit anti-loop discipline, agents will retry the same failing command, broaden scope after a timeout, rewrite working code to fix unrelated failures, or commit generated cache files.

### Core anti-loop rules

1. **Two-strike rule.** Stop after two failed attempts at the same command, test, or code path. After the second failure, do not retry. Record the failure, form a hypothesis, and change the smallest possible thing.
2. **No scope broadening after timeout.** If a workflow times out, do not add new commands, new files, or new hypotheses to recover. Restore state first, then re-plan in a fresh, narrower step.
3. **Do not rewrite working code to fix unrelated failures.** If a recent change is not the cause of a failure, do not edit it. Edit only the closest thing to the failure.
4. **Record before retry.** Before any retry, record: command, failure mode, hypothesis, smallest next action. If you cannot articulate the hypothesis, you are not ready to retry.
5. **Service-state restore beats forward progress.** If the service/router/backend state becomes ambiguous, restore first: stop service/router/backend, free port 9130, confirm no `llama-server` orphans. Then re-plan.
6. **One repo at a time.** Never mutate both repos in the same sprint unless the sprint explicitly requires it. Mixing repos is the most common cause of "this is bigger than the original scope".
7. **No cache files in commits.** Never commit `__pycache__/`, `*.pyc`, `*.pyo`, `.pytest_cache/`, build artifacts, model files, or other generated content. Add to `.gitignore` instead.
8. **No pre-existing-dirty sweep.** Never sweep pre-existing modified or untracked files into a sprint commit without explicit scope. Treat pre-existing dirt as prior context; either seal it in its own docs-only commit, or leave it.
9. **Generate temporary tokens locally.** If auth/token setup blocks a proof, generate a temporary local token. Do not ask the Owner to paste secrets. The receipt records `token_source: "environment"` and `token_logged: false` — never the token value.
10. **Receipts may report partial/fail.** Do not edit evidence to force pass. If the receipt's `overall` is `partial` or `fail`, the receipt is doing its job. Honest records are more valuable than green checks.

### Cross-repo and pre-existing state

- If a sprint starts with pre-existing dirty files in either repo, treat them as out-of-scope unless the Owner explicitly assigns them.
- If a sprint needs to seal pre-existing dirty files, do so in a separate docs-only commit, never bundled with code changes.
- If `git status` shows files you did not intend to change, stop. Restore them, or ask before committing.

### Process and service discipline

- The Rust router and the Python router both leave orphan risks. Always check for `rust-router.exe`, `python.exe` (router), and `llama-server.exe` after closing.
- Port 9130 must be free at sprint closeout. The ad-hoc router used for proof runs is not a permanent service; kill it before declaring done.
- The Windows service `LibrarianRunTimeNode` must remain `Stopped` / `Manual` unless the Owner explicitly approves changing it.
- Never call `nssm set` or modify service parameters during a proof run. Use the ad-hoc router binary; leave NSSM alone.

### Token discipline

- Generate temporary tokens locally (e.g., `[System.Guid]::NewGuid().ToString("N")`) and never print them.
- Do not write the token to any repo file. If you must persist across bash invocations, use a temp file outside the repo (e.g., `$env:TEMP\run_token.txt`) and delete it after the proof.
- Do not log the token. The proof and verifier must both treat the token as opaque.
- The receipt must record only `token_source`, `token_logged`, `missing_token_status`, and `invalid_token_status` — never the token value.

### Evidence discipline

- The receipt's `result.overall` must be derived from recorded evidence, not manually asserted. If the proof says all 7 endpoints passed, the receipt must say so. If cleanup left 1 orphan, the receipt must say so.
- The verifier's `--reject` mode exists for a reason. Run a rejection test (malformed or secret-bearing receipt) at least once per sprint to prove the verifier actually rejects.
- Do not "fix" a partial/fail verdict by changing what the proof checks. Fix the underlying issue or change the schema only if the prior schema was wrong.

### Commit hygiene

- One sprint = one coherent commit (or one docs commit + one code commit for cross-cutting sprints).
- Commit messages should reference the sprint ID, list the substantive changes, and note any intentionally-out-of-scope items.
- Never commit `__pycache__/`, build artifacts, or model files. Add to `.gitignore` first.

---

## Future Direction

The Librarian should eventually own this startup sequence directly.

**Target future behavior:**
1. Agent opens sprint.
2. The Librarian runs environment inspection.
3. The Librarian records repo/service/process state.
4. The Librarian builds a governed work packet.
5. Agent receives bounded task capsule.
6. Agent returns receipt.
7. The Librarian validates and records closeout.

Until that exists, this startup sequence is the manual operating contract.
