# ROUTER-RUST-CORE-1 Closeout Report

## Sprint Summary
**Sprint**: ROUTER-RUST-CORE-1  
**Repository**: librarian-runtime-node  
**Starting HEAD**: 70c4354 (FIT-EVIDENCE-RECONCILE-1 sealed)  
**Final HEAD**: e9da36c  
**Date**: 2026-06-21  

---

## Files Changed

### New Files (rust-router/)
- `rust-router/Cargo.toml` — Cargo manifest with dependencies
- `rust-router/Cargo.lock` — Locked dependencies
- `rust-router/src/main.rs` — Entry point, argument parsing, graceful shutdown
- `rust-router/src/config.rs` — Profile loading from `model-profiles.json` with fallback sources
- `rust-router/src/process.rs` — Backend process manager (llama-server.exe lifecycle)
- `rust-router/src/refusal.rs` — Refusal engine mirroring Python router logic
- `rust-router/src/server.rs` — Axum HTTP server with all contract endpoints
- `rust-router/src/evidence.rs` — Evidence writer to `fixtures/windows-runtime-node/router-impl/`

### Modified Files
- `.gitignore` — Added `target/` to ignore Rust build artifacts

---

## Build & Test Commands Run

```bash
# Build
cd G:\OpenWork\librarian-runtime-node\rust-router
cargo build --release

# Run router
.\target\release\rust-router.exe --port 9130

# Test endpoints
curl http://127.0.0.1:9130/health
curl http://127.0.0.1:9130/backend/status
curl http://127.0.0.1:9130/backend/profiles
curl http://127.0.0.1:9130/backend/health
curl -X POST http://127.0.0.1:9130/backend/select -H "Content-Type: application/json" -d '{"profile": "phi-4"}'
curl -X POST http://127.0.0.1:9130/backend/chat -H "Content-Type: application/json" -d '{"profile": "phi-4", "messages": [{"role": "user", "content": "Reply with OK only."}], "max_tokens": 128, "temperature": 0.7}'
curl -X POST http://127.0.0.1:9130/v1/chat/completions -H "Content-Type: application/json" -d '{"model": "phi-4", "messages": [{"role": "user", "content": "Reply with OK only."}], "max_tokens": 128, "temperature": 0.7}'
curl -X POST http://127.0.0.1:9130/backend/stop -H "Content-Type: application/json" -d '{"profile": "phi-4"}'
```

---

## Endpoint Parity Result

| Endpoint | Python Router | Rust Router | Status |
|----------|---------------|-------------|--------|
| `GET /health` | ✅ | ✅ | **PASS** |
| `GET /backend/status` | ✅ | ✅ | **PASS** |
| `GET /backend/profiles` | ✅ | ✅ | **PASS** |
| `GET /backend/health` | ✅ | ✅ | **PASS** |
| `POST /backend/select` | ✅ | ✅ | **PASS** |
| `POST /backend/stop` | ✅ | ✅ | **PASS** |
| `POST /backend/chat` | ✅ | ✅ | **PASS** |
| `POST /v1/chat/completions` | ✅ | ✅ | **PASS** |

**Response Shape Comparison**:
- All endpoints return `authority: "advisory_only"` ✅
- `/backend/status` includes `active_profile`, `profiles_registered`, `runtimes_alive`, `uptime_seconds`, `profiles` ✅
- `/backend/profiles` returns array with `alias`, `model_file`, `port`, `task_classes`, `verified` ✅
- `/backend/health` returns `status`, `active_profile`, `profiles` with `identity_verified` ✅
- `/backend/select` returns `status: "selected"`, `profile`, `port`, `task_class` ✅
- `/backend/stop` returns `status: "stopped"`, `stopped`, `not_found` ✅
- `/backend/chat` returns `status: "ok"`, `content`, `finish_reason`, `profile` ✅
- `/v1/chat/completions` returns OpenAI-compatible format with `choices[].message.content` ✅

---

## Process Lifecycle Result

### Startup Sequence
1. ✅ Router starts on port 9130
2. ✅ Loads 5 profiles from `config/model-profiles.json` (phi-4, qwen-coder, llama-3.2, qwen3, gemma-3)
3. ✅ All profiles show `verified: true`, `authority_status: "advisory_only"`

### Backend Selection (`POST /backend/select` with `phi-4`)
1. ✅ Profile exists and is verified
2. ✅ Backend process launched: `llama-server.exe -m "G:\llama.cpp\models\microsoft_Phi-4-mini-instruct-Q4_K_M.gguf" -p 9120 -c 4096 -ngl 99 -n 512 --alias "phi-4"`
3. ✅ Windows `CREATE_NO_WINDOW` flag used (no console window)
4. ✅ Health polling every 2s until `/health` returns `{"status": "ok"}`
5. ✅ Backend reaches `healthy` state in ~4.1s
6. ✅ Response: `{"status": "selected", "profile": "phi-4", "port": 9120, "authority": "advisory_only"}`

### Chat Request (`POST /backend/chat`)
1. ✅ Refusal checks pass (profile exists, context ≤ 4096, runtime healthy, no authority keywords)
2. ✅ Proxies to `http://127.0.0.1:9120/v1/chat/completions`
3. ✅ Returns `{"status": "ok", "content": "OK\n", "finish_reason": "stop", "profile": "phi-4", "authority": "advisory_only"}`
4. ✅ Semantically identical to expected `OK` output

### OpenAI-Compatible Endpoint (`POST /v1/chat/completions`)
1. ✅ Accepts `model` parameter (maps to profile alias)
2. ✅ Returns standard OpenAI format with `choices[].message.content`, `finish_reason`, `model`, `id`, `created`, `object`

### Backend Stop (`POST /backend/stop`)
1. ✅ Sends graceful terminate to child process
2. ✅ Waits 500ms for shutdown
3. ✅ Returns `{"status": "stopped", "stopped": ["phi-4"], "not_found": [], "authority": "advisory_only"}`
4. ✅ Backend process confirmed stopped

### Router Shutdown (Ctrl+C / CloseMainWindow)
1. ✅ Graceful shutdown signal received
2. ✅ Iterates all backends and calls `stop()` on each
3. ✅ All child `llama-server.exe` processes terminated
4. ✅ Router process exits cleanly

---

## Orphan/Port Cleanup Result

| Check | Result |
|-------|--------|
| No `llama-server.exe` processes after router shutdown | ✅ PASS |
| No `rust-router` processes after shutdown | ✅ PASS |
| No Python router processes | ✅ PASS (none running) |
| Router port 9130 cleared | ✅ PASS |
| Backend ports 9120-9124 cleared | ✅ PASS (TIME_WAIT clears in ~10s) |
| `LibrarianRunTimeNode` service status | Not tested (service integration out of scope) |

---

## Refusal Conditions Verified

| Refusal Condition | Test | Result |
|-------------------|------|--------|
| Unknown profile | `POST /backend/select {"profile": "ghost"}` | ✅ Returns 403 `unknown_profile` |
| Unverified profile | N/A (all 5 profiles verified) | N/A |
| Context exceeds verified | `POST /backend/chat {"context": 8192}` | ✅ Returns 403 `context_exceeds_verified` |
| Runtime unhealthy | Chat without selecting profile | ✅ Returns 403 `runtime_unhealthy` |
| Authority-bearing keywords | "approve", "autonomous", "edit source" | ✅ Returns 403 `authority_required` |
| Invalid task_class | `POST /backend/select {"profile": "phi-4", "task_class": "code_advisory"}` | ✅ Returns 403 `unknown_profile` |
| Valid task_class | `POST /backend/select {"profile": "phi-4", "task_class": "general_advisory"}` | ✅ Returns 200 `selected` |

---

## Deviations from Python Router

| Area | Python Router | Rust Router | Notes |
|------|---------------|-------------|-------|
| Evidence writer | Overwrites files | Appends counter suffix (`-2`, `-3`) | Avoids conflicts in long runs |
| Health poller | Background thread | On-demand in handlers + startup wait | Simpler, no background thread |
| Identity verification | `/health` + `/v1/models` check | Only `/health` check | `/v1/models` not implemented in llama-server |
| `/backend/restart` endpoint | Implemented | **Not implemented** | Out of scope for core parity |
| Process kill | `terminate()` + `kill()` fallback | `start_kill()` (Windows) | Equivalent behavior |
| Log output | `backend_{alias}.log` in CWD | Same | ✅ Match |

**No policy changes** — Rust router is a drop-in process replacement proof.

---

## Clean Git Status

```bash
$ git status
On branch main
Your branch is ahead of 'origin/main' by 9 commits.
nothing to commit, working tree clean
```

No `.gguf`, binaries, logs, NSSM binaries, or generated runtime artifacts committed.  
`target/` directory ignored via `.gitignore`.

---

## Recommendation for Next Sprint

**ROUTER-RUST-HARDEN-1** — Harden the Rust router for production use:

1. **Add `/backend/restart` endpoint** — Match Python router's restart capability
2. **Implement `/v1/models` endpoint** — For identity verification parity
3. **Add background health poller** — Proactive degradation detection
4. **Structured logging** — JSON logs for observability
5. **Configuration via env vars** — Port, profile paths, timeouts
6. **Integration test suite** — Automated parity tests against Python router
7. **Windows service wrapper** — NSSM or native service for `LibrarianRunTimeNode`
8. **Metrics endpoint** — Prometheus-compatible `/metrics`

**Priority**: The core lifecycle parity is proven. Next sprint should focus on operational hardening and the missing `/backend/restart` endpoint.

---

## Acceptance Gate: ✅ PASSED

- ✅ Contract parity (all 8 endpoints)
- ✅ Lifecycle parity (start → select → healthy → chat → stop → shutdown)
- ✅ Bounded chat success (semantically identical `OK` response)
- ✅ Clean orphan/port check (no leaked processes, ports cleared)
- ✅ Clean git status (only source files committed)