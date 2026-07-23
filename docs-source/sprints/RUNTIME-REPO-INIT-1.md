# RUNTIME-REPO-INIT-1: Librarian Runtime Node Repository Initialization

**Status:** COMPLETE
**Date:** 2026-06-19
**Local path:** `G:\openwork\librarian-runtime-node\`
**Remote:** `https://github.com/andrewdhannah/librarian-runtime-node.git`

---

## Purpose

Initialize `librarian-runtime-node` as its own GitHub-backed repository with
a proper project identity, README, custody structure, and first baseline import.

This establishes an independent Git home for the Windows runtime node, separate
from the canonical Librarian repo at `G:\openwork\thelibrarian\`.

---

## Starting State

- **Git:** Not initialized (no `.git` directory)
- **Working tree:** 7 directories, 1 existing README (minimal/operational), router code, config, scripts
- **Large files:** `runtime\llama.cpp\llama-server.exe` (65.8 MB) — excluded from git
- **Model files:** None present (`models/` contains only `.keep`)
- **Secrets:** None present (`config/runtime-node.local.json` has machine-local paths, gitignored)
- **Process state:** 1 orphan `llama-server.exe` (PID 948) from prior restart testing

---

## Files Created / Updated

| File | Action | Content |
|------|--------|---------|
| `.gitignore` | Created | Model, cache, log, secret, build, and IDE exclusions |
| `README.md` | Replaced | Full project identity — status, endpoints, profiles, architecture, roadmap, non-goals |
| `docs/architecture/RUNTIME-NODE-ARCHITECTURE.md` | Created | Component overview, endpoint contract, profile config, process custody, refusal semantics, lifecycle gap, Rust rationale |
| `docs/sprints/RUNTIME-REPO-INIT-1.md` | Created | This sprint report |
| `SESSION-HANDOFF.md` | Created | Cross-session state for next owner |
| `docs/architecture/` | Created | Architecture doc directory |
| `docs/sprints/` | Created | Sprint doc directory |
| `fixtures/windows-runtime-node/router-impl/` | Created | Evidence fixture directory (files committed from thelibrarian) |

---

## Git Initialization

```powershell
git init
git branch -M main
git remote add origin https://github.com/andrewdhannah/librarian-runtime-node.git
```

---

## Ignored Artifact Policy

The `.gitignore` excludes:
- Model files: `*.gguf`, `*.safetensors`, `*.bin`, `*.pt`, `*.pth`, `models/`
- Cache/index: `.cache/`, `.zvec/`, `indexes/`, `*.zvec`, `*.zvecdb`
- Python cache: `__pycache__/`, `*.pyc`, `.venv/`, `venv/`
- Logs/temp: `logs/`, `*.log`, `tmp/`, `temp/`
- Secrets: `.env`, `*.secret`, `secrets/`, `config/*.local.json`
- Windows build: `*.exe`, `*.dll`, `*.pdb`, `runtime/bin/`, `runtime/router/`
- Binary: `runtime/llama.cpp/llama-server.exe`
- Local evidence: `fixtures/windows-runtime-node/router-impl/*.json`

---

## Acceptance Checklist

| Criterion | Status |
|-----------|--------|
| Repo has proper name/identity in README | ✅ PASS |
| README explains status, endpoints, governance, roadmap, non-goals | ✅ PASS |
| `.gitignore` excludes models, caches, logs, secrets, generated indexes | ✅ PASS |
| Existing router/config/evidence files preserved | ✅ PASS |
| No service install or service-state mutation | ✅ PASS |
| No model files committed | ✅ PASS |
| No secrets committed | ✅ PASS |
| `docs/sprints/RUNTIME-REPO-INIT-1.md` exists | ✅ PASS |
| `SESSION-HANDOFF.md` identifies WIN-SERVICE-LIFECYCLE-1 as next startup-only sprint | ✅ PASS |
| Git status clean after commit | ✅ PASS — `cf6e62a`, working tree clean |
| Push result reported | ✅ PASS — pushed to `origin/main` via HTTPS |

---

## Closeout

```
Sprint:               RUNTIME-REPO-INIT-1
Status:               COMPLETE
Final HEAD:           cf6e62a
Working tree:         Clean (35 files)
Harness result:       N/A (repo init — no runtime verification)
Stash state:          N/A
Next sprint:          WIN-SERVICE-LIFECYCLE-1 (startup only)
```

## Notes

- SSH key not configured on this machine; remote uses HTTPS.
- Push succeeded to `origin/main` via HTTPS.
- The orphan `llama-server.exe` (PID 948) remains as a known lifecycle gap.
- Evidence files from WIN-ROUTER-IMPL-1 remain at `G:\openwork\thelibrarian\fixtures\windows-runtime-node\router-impl\` and were imported here as reference copies.
