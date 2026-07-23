# Runtime Node Architecture

> Component architecture and contract summary for the Librarian Runtime Node.
>
> Root: `G:\openwork\librarian-runtime-node\`
> Router: `router/router.py`
> Config: `config/model-profiles.json`

---

## 1. Component Overview

```
┌───────────────────────────────────────────────────────────┐
│                  Librarian Runtime Node                     │
│                                                           │
│  ┌─────────────────────────────────────────────────────┐  │
│  │                 Router (Python)                       │  │
│  │  ThreadingHTTPServer on configurable port             │  │
│  │                                                       │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────┐  │  │
│  │  │  Handler      │  │  Refusal     │  │  Evidence   │  │  │
│  │  │  (6 endpoints)│  │  Layer       │  │  Writer     │  │  │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬─────┘  │  │
│  │         │                 │                   │        │  │
│  │  ┌──────▼─────────────────▼───────────────────▼─────┐  │  │
│  │  │           ProcessManager (per profile)            │  │  │
│  │  │  start(profile) → poll_health → verify_identity  │  │  │
│  │  │  stop() → restart() → get_status()               │  │  │
│  │  └──────────────────────┬──────────────────────────┘  │  │
│  └─────────────────────────┼──────────────────────────────┘  │
│                            │                                 │
│  ┌─────────────────────────┼──────────────────────────────┐  │
│  │  Profile Config          │                              │  │
│  │  (model-profiles.json)  │                              │  │
│  │  5 profiles: phi-4,     │                              │  │
│  │  qwen-coder, llama-3.2, │                              │  │
│  │  qwen3, gemma-3         │                              │  │
│  └─────────────────────────┘                              │  │
│                                                           │  │
│  ┌──────────────────────────────────────────────┐         │  │
│  │  Backend Processes (llama-server.exe × N)     │         │  │
│  │  Ports 9120-9124, each with --alias,          │         │  │
│  │  Vulkan GPU backend, ngl=99, context=1024     │         │  │
│  └──────────────────────────────────────────────┘         │  │
└───────────────────────────────────────────────────────────┘
```

---

## 2. Endpoint Contract Summary

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/backend/status` | GET | None | Returns all profile states, active profile, counts |
| `/backend/profiles` | GET | None | Returns all registered profiles with metadata |
| `/backend/health` | GET | None | Per-profile health, overall status |
| `/backend/select` | POST | Internal | Selects/starts a profile by alias; stops previously active |
| `/backend/chat` | POST | Refusal | Proxies chat to active backend; refuses authority-bearing requests |
| `/backend/restart` | POST | Internal | Stops and restarts a profile's backend process |

---

## 3. Profile Config Contract

Each profile in `config/model-profiles.json` follows this schema:

```json
{
  "alias": "string (unique identifier)",
  "model_path": "string (absolute path to GGUF)",
  "port": "integer (unique, 9120-9124 range)",
  "ngl": "integer (default 99)",
  "context": "integer (default 1024)",
  "task_classes": ["string array of allowed tasks"],
  "authority": "string (always 'advisory_only')"
}
```

---

## 4. Process Custody Model

The `ProcessManager` class is the custody unit:

```
start()
  ├── spawn llama-server.exe with --alias, -p, -c, -ngl
  ├── set state = "starting"
  ├── poll /health every 2s (up to timeout)
  │   └── on success: set state = "healthy", verify identity
  └── on failure: set state = "failed", return error

poll_health()  [called every 15s by background thread]
  ├── check process alive (poll())
  ├── GET /health
  │   └── if 200 + status=ok + model matches: set state = "healthy"
  ├── on identity mismatch: set state = "degraded"
  └── on 3 consecutive failures: set state = "failed"

stop()
  ├── terminate() process
  ├── wait() with 5s timeout
  └── kill() if still alive

restart()
  ├── stop()
  └── start()
```

---

## 5. Refusal Semantics

The router enforces 8 refusal conditions, all returning structured JSON with
`status: "refused"`, `reason`, and `detail`:

| Condition | Trigger |
|-----------|---------|
| `unknown_profile` | Profile alias not in registry |
| `authority_required` | Message contains authority-bearing terms (approve, deploy, authorize, confirm, grant, reject, override, escalate) |
| `profile_unavailable` | Profile exists but backend cannot start |
| `profile_busy` | Profile already selected for a different session |
| `no_active_profile` | Chat request with no selected profile |
| `identity_mismatch` | Backend reports wrong model identity |
| `task_class_mismatch` | Request task not in profile's allowed set |
| `backend_degraded` | Backend health check failed |

---

## 6. Known Lifecycle Gap

During WIN-ROUTER-IMPL-1 restart testing, an orphan `llama-server.exe` process
was observed after the router was killed without cleanly stopping its child
processes. This confirms that:

- The router's `stop()` methods work correctly when called explicitly.
- But if the router itself terminates unexpectedly, child backends may survive.
- WIN-SERVICE-LIFECYCLE-1 must address this through service-level process
  custody (e.g., NSSM job objects, process group management).

---

## 7. Future Rust Production Daemon Rationale

| Concern | Python (current) | Rust (target) |
|---------|-----------------|---------------|
| Process supervision | `subprocess.Popen` + polling | `std::process` + native signal handling |
| Concurrency | `ThreadingHTTPServer` (OS threads) | `tokio` async I/O |
| Structured config | JSON loaded at startup | Same + `serde` schema validation |
| Crash isolation | Single-process, GIL-bound | Process-level isolation, panic boundaries |
| Binary distribution | Requires Python runtime | Single portable binary |
| Windows service integration | Wrapper (NSSM) | Native Windows service via `windows-service` crate |
| Dependency footprint | Python stdlib + third-party | Cargo-managed, auditable |

The Python router is **correct for proving behavior** — rapid iteration,
no compile step, easy to inspect. The Rust router is **correct for owning
custody** — single binary, native OS integration, strong concurrency
guarantees, auditable dependency tree.
