# Librarian Runtime Node

**Local model runtime custody node for The Librarian.**

A governed local runtime node that hosts and supervises model backends for
The Librarian project. Provides profile selection, backend health monitoring,
chat proxying, process restart, and structured refusal behavior — all within
a strict advisory-only authority boundary.

---

## Current Status

**7 completed sprints — runtime-node foundation proven. See the [Windows PC Sprint Roadmap](docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md) for the full plan.**

The Python router is the current behavioral reference implementation, verified
across 6 endpoint types and 8 verification cases. Full details in sprint records
under `docs/sprints/`.

Last sealed sprint: **REDUCED-OFFLOAD-FIT-1** — all 5 profiles verified at full
4096 context on RX 570 4 GB (phi-4 and qwen-coder at ngl=99; llama-3.2, qwen3,
and gemma-3 at ngl=80).

| Endpoint | Method | Behavior |
|----------|--------|----------|
| `/backend/status` | GET | Returns all profile states, active profile, counts |
| `/backend/profiles` | GET | Returns all 5 registered profiles with metadata |
| `/backend/health` | GET | Per-profile health, overall status |
| `/backend/select` (valid) | POST | Starts backend on assigned port |
| `/backend/select` (invalid) | POST | 403 `unknown_profile` structured refusal |
| `/backend/chat` (allowed) | POST | Proxies to backend, returns content |
| `/backend/chat` (authority) | POST | 403 `authority_required` structured refusal |
| `/backend/restart` | POST | Stops + starts backend, PID changes |

### Governance Principle

> Confidence was not allowed to become authority until runtime behavior was tested.

### Production Rule

> Python may prove the behavior. Rust should eventually own the production custody daemon.

### Agent Startup Sequence

Agents working on the Windows PC lane **must** follow the
[Windows Agent Startup Sequence](docs/operations/WINDOWS-AGENT-STARTUP-SEQUENCE.md)
before modifying any files. This matches the inspection discipline used by Mac agents.

---

## Runtime Profiles

Five model profiles are registered with per-profile ngl and context settings:

| Profile | Port | Size | Task Classes |
|---------|------|------|-------------|
| `phi-4` | 9120 | 2.32 GB | general_advisory, summarization_advisory |
| `qwen-coder` | 9121 | 1.76 GB | code_advisory, fallback_small_model |
| `llama-3.2` | 9122 | 2.16 GB | general_advisory |
| `qwen3` | 9123 | 2.33 GB | general_advisory, reasoning |
| `gemma-3` | 9124 | 2.32 GB | general_advisory |

**Current profile settings:**

| Profile | Context | ngl | Status | Sprint |
|---------|---------|-----|--------|--------|
| `phi-4` | 4096 | 99 | Verified safe | WIN-MODEL-CONTEXT-FIT-2 |
| `qwen-coder` | 4096 | 99 | Verified safe | WIN-MODEL-CONTEXT-FIT-2 |
| `llama-3.2` | 4096 | **80** | Verified safe at ngl=80 | REDUCED-OFFLOAD-FIT-1 |
| `qwen3` | 4096 | **80** | Verified safe at ngl=80 | REDUCED-OFFLOAD-FIT-1 |
| `gemma-3` | 4096 | **80** | Verified safe at ngl=80 | REDUCED-OFFLOAD-FIT-1 |

All 5 profiles are now verified at **4096 context** on the RX 570 4 GB.
Phi-4 and qwen-coder run at full GPU offload (ngl=99). Llama-3.2, qwen3, and
gemma-3 require reduced GPU offload (ngl=80) to fit on this GPU. See
[REDUCED-OFFLOAD-FIT-1](docs/sprints/REDUCED-OFFLOAD-FIT-1.md) for the
complete test matrix and evidence.

---

## Architecture

```
┌─────────────────────────────────────────────┐
│              Router (Python)                  │
│  ┌─────────┐  ┌──────────┐  ┌────────────┐  │
│  │ Profile  │  │  Refusal │  │  Evidence   │  │
│  │ Registry │  │   Layer  │  │   Writer    │  │
│  └────┬─────┘  └────┬─────┘  └────────────┘  │
│       │             │                         │
│  ┌────▼─────────────▼─────────────────────┐  │
│  │       ProcessManager (per profile)       │  │
│  │  start │ health │ identity │ stop        │  │
│  └───────────────┬──────────────────────────┘  │
└──────────────────┼──────────────────────────────┘
                   │
    ┌──────────────┼──────────────┐
    ▼              ▼              ▼
 llama-server   llama-server   llama-server
 (phi-4:9120)  (qwen-coder:   (llama-3.2:
                  9121)         9122)
```

- **Router** — Python `ThreadingHTTPServer` serving 6 endpoint types
- **Model profiles config** — JSON schema defining alias, port, model path, ngl, context, task classes, authority
- **Process supervisor** — `ProcessManager` per profile: start, health poll, identity verify, restart, stop
- **Health checks** — Polls `/health` every 15s, transitions state: `starting → healthy → degraded → failed`
- **Refusal layer** — 8 structured refusal conditions (unknown_profile, authority_required, profile_unavailable, etc.)
- **Evidence fixtures** — Verified responses captured as JSON fixtures for regression testing
- **Future service lifecycle** — WIN-SERVICE-LIFECYCLE-1 will wrap the router as a Windows service

---

## Roadmap

See the **[Windows PC Sprint Roadmap](docs/roadmap/WINDOWS-PC-SPRINT-ROADMAP.md)**
for the full three-layer plan:

| Layer | Focus | Status |
|-------|-------|--------|
| Layer 1 — Runtime Node Reliability | Profiles, operations, tooling | Active |
| Layer 2 — Portable Router / Native Daemon | Contract tests, Rust core, native service | Planned |
| Layer 3 — Windows Librarian Client/App | App architecture, shell, runtime integration, custody UI | Proposed |

**Completed sprints** are documented under `docs/sprints/`.

---

## Non-Goals

- Not the main Librarian application.
- Not a model repository or download manager.
- Not a general-purpose public inference server.
- Not an authority engine — does not grant approval, completion, or canonical status.
- Not a replacement for the Mac-hosted canonical Librarian repo.

---

## Safety / Custody Notes

- **Local-first runtime component.** This node serves the Librarian project
  but operates as an independent runtime.
- **Do not commit** generated logs, model files, indexes, or secrets to this repo.
  See `.gitignore` for the full exclusion policy.
- **Service installation must be owner-approved.** No Windows service mutation
  without explicit authorization (see WIN-SERVICE-LIFECYCLE-1 boundary).
- **Backend child processes must remain under router custody.** A known lifecycle
  gap exists: an orphan `llama-server.exe` was observed after earlier restart
  testing. This is a target for WIN-SERVICE-LIFECYCLE-1.
- **Config is local.** Copy `config/runtime-node.example.json` to
  `config/runtime-node.local.json` and edit for your environment. The local
  config file is gitignored.
