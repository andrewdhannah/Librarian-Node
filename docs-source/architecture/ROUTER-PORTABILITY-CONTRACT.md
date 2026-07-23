# Portable Router Contract

> Canonical API contract, profile schema, lifecycle state machine, and
> OS-wrapper boundary for the Librarian Runtime Node Router.
>
> Root: `G:\openwork\librarian-runtime-node\`
> Sprint: ROUTER-PORTABILITY-1
> Status: SPECIFICATION — first proven implementation on Windows

---

## Table of Contents

1. [Purpose](#1-purpose)
2. [Router Core vs OS Wrapper Boundary](#2-router-core-vs-os-wrapper-boundary)
3. [Canonical HTTP API](#3-canonical-http-api)
4. [Profile Registry Schema](#4-profile-registry-schema)
5. [Backend Launch Policy](#5-backend-launch-policy)
6. [Backend Lifecycle State Machine](#6-backend-lifecycle-state-machine)
7. [Health & Status Semantics](#7-health--status-semantics)
8. [Select / Chat / Restart Semantics](#8-select--chat--restart-semantics)
9. [Advisory Authority Boundary](#9-advisory-authority-boundary)
10. [Refusal & Error Semantics](#10-refusal--error-semantics)
11. [Runtime Receipt Format](#11-runtime-receipt-format)
12. [OS-Specific Runtime Wrapper](#12-os-specific-runtime-wrapper)
13. [Windows Implementation Map](#13-windows-implementation-map)
14. [Cross-Platform Acceptance Tests](#14-cross-platform-acceptance-tests)
15. [Future Implementation Path](#15-future-implementation-path)

---

## 1. Purpose

The Router is a **portable runtime-control contract**, not a single
implementation. The current Python router on Windows is the first proven
runtime limb. Router logic and networking contract must work regardless of
OS.

This document defines:

- What every Router implementation must provide.
- What lives in the portable core vs what lives in the OS wrapper.
- How the current Windows Python implementation maps to the contract.
- How future implementations (Rust, native daemon) can adopt the same contract.

---

## 2. Router Core vs OS Wrapper Boundary

```
┌────────────────────────────────────────────────────────────┐
│                  ROUTER CORE (portable)                      │
│                                                              │
│  • Canonical HTTP API (6 endpoints)                          │
│  • Profile registry (load, validate, serve)                  │
│  • Backend launch policy (select profile → spawn)            │
│  • Backend health checks (poll /health, identity verify)     │
│  • Lifecycle state machine (stopped → starting → healthy …)  │
│  • Advisory authority labeling (all responses advisory_only) │
│  • Refusal engine (8 structured refusal conditions)          │
│  • Runtime receipts (evidence/audit records)                 │
│                                                              │
│  Dependencies: HTTP server, JSON, process I/O, timer         │
│                                                              │
└───────────────────────────┬──────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────┐
│                OS-SPECIFIC RUNTIME WRAPPER                   │
│                                                              │
│  Responsibilities:                                           │
│  • Start / stop / monitor the Router process                 │
│  • Capture stdout/stderr to OS logging                      │
│  • Set working directory                                     │
│  • Configure listening port (default: 9130)                  │
│  • Ensure Router is restarted on crash (policy-dependent)   │
│  • Provide process-group custody so children are cleaned up  │
│                                                              │
│  Implementations:                                            │
│  │ Windows: NSSM service → PowerShell launcher → Python     │
│  │ macOS:   launchd plist → zsh launcher → Python (future)  │
│  │ Linux:   systemd service → bash launcher → Python         │
│  │ Dev:     manual `python router.py --port 9130`            │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

**Rule:** the OS wrapper must be a thin host adapter. It must not contain
router logic (health checks, refusal conditions, profile knowledge, state
machine). Forking or reimplementing router logic in the wrapper is a
contract violation.

---

## 3. Canonical HTTP API

### 3.1 Base URL

All endpoints are served under `/backend/` on the configured port.

Default port: `9130`

A legacy `/health` endpoint MAY be provided for compatibility but is not
part of the canonical contract.

### 3.2 Endpoint Table

| Method | Path               | Auth Needed | Request Body         | Response Status | Description                              |
|--------|--------------------|-------------|----------------------|-----------------|------------------------------------------|
| GET    | `/backend/status`  | No          | —                    | 200             | Router overview + all profile states     |
| GET    | `/backend/profiles`| No          | —                    | 200             | Registered profiles with metadata        |
| GET    | `/backend/health`  | No          | —                    | 200             | Per-profile health, overall status       |
| POST   | `/backend/select`  | No          | `{"profile":"..."}`  | 200 / 403       | Select & start a profile backend         |
| POST   | `/backend/chat`    | No          | `{"profile":"...", "messages":[...]}` | 200 / 403 / 502 | Proxy chat to active backend   |
| POST   | `/backend/restart` | No          | `{"profile":"..."}`  | 200 / 500       | Stop & restart a profile's backend       |

### 3.3 Response Envelope

Every response MUST contain an `"authority": "advisory_only"` field.

**Success (200):**
```json
{
  "status": "ok",
  "authority": "advisory_only",
  ...endpoint-specific fields
}
```

**Refusal (403):**
```json
{
  "status": "refused",
  "reason": "string",
  "detail": "human-readable explanation",
  "authority": "advisory_only",
  "timestamp": "ISO-8601 utc"
}
```

**Client Error (400):**
```json
{
  "error": "Missing 'profile' field"
}
```

**Upstream Error (502/503):**
```json
{
  "status": "error",
  "error": "Backend request failed: ...",
  "profile": "alias",
  "authority": "advisory_only"
}
```

### 3.4 CORS

Implementations SHOULD respond to `OPTIONS` with:
```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, OPTIONS
Access-Control-Allow-Headers: Content-Type
```

---

## 4. Profile Registry Schema

Each profile describes one model backend. Profiles are loaded from a JSON
file at startup. The schema:

```json
{
  "_meta": {
    "description": "string",
    "generated": "ISO-date",
    "source": "sprint or tool that generated this file",
    "authority": "advisory_only"
  },
  "defaults": {
    "backend": "string (e.g. vulkan)",
    "ngl": "integer (GPU layers, default 99)",
    "context": "integer (context window, default 1024)",
    "max_tokens": "integer (default 512)",
    "binary": "filesystem path to llama-server binary",
    "gguf_root": "filesystem path to model directory"
  },
  "profiles": [
    {
      "alias": "string (unique profile identifier)",
      "model_file": "string (GGUF filename)",
      "model_path": "string (absolute filesystem path)",
      "gguf_size_gb": "float",
      "backend": "string",
      "ngl": "integer",
      "context": "integer",
      "port": "integer (unique per profile)",
      "launch_command": "string (diagnostic/display only — not executed verbatim)",
      "task_classes": ["string array of allowed tasks"],
      "verified_status": "string (verified | unverified)",
      "evidence_path": "string (path to fit-test evidence)",
      "authority_status": "string (always advisory_only)",
      "limitations": "string (known constraints)",
      "known_behavior": "string (observed output characteristics)",
      "test_cells": ["string array of fit test labels"]
    }
  ]
}
```

### 4.1 Required Fields for the Router Core

The router core MUST read at minimum: `alias`, `model_path`, `port`,
`ngl`, `context`, `task_classes`, `verified_status`, `authority_status`.

### 4.2 Profile Port Convention

Ports SHOULD be assigned sequentially from a base range:
- phi-4: 9120
- qwen-coder: 9121
- llama-3.2: 9122
- qwen3: 9123
- gemma-3: 9124

Each profile occupies exactly one port. No two profiles share a port.

---

## 5. Backend Launch Policy

The router core is responsible for:

1. **Spawn** — Launch `llama-server` (or equivalent backend binary) as a
   child process with the profile's model path, port, context size, GPU
   layers, and alias.
2. **Wait for health** — Poll the backend `/health` endpoint until it
   responds `{"status": "ok"}` or a timeout elapses (default: 180 s).
3. **Identity verify** — Confirm the backend's health-reported model alias
   matches the profile alias. If not, mark as `degraded`.
4. **Refuse on failure** — If the backend cannot start or identity does not
   match, set state to `failed` and refuse routing to it.

### 5.1 Launch Command Construction

The router core constructs the launch command from structured fields, not
by parsing a pre-built command string. The pre-built `launch_command` in
the profile config is for diagnostic/display purposes only.

Canonical argument mapping:
```
-m <model_path>      (model GGUF file)
-p <port>            (listening port)
-c <context>         (context window size)
-ngl <ngl>           (GPU layers)
-n <max_tokens>      (max tokens to generate)
--alias <alias>      (model identity label)
```

Additional flags (e.g. `--no-mmap`, `--mlock`, `--threads`) MAY be added
by the router core but MUST be explicitly declared in the launch logic.

### 5.2 Process Custody

The spawned backend MUST be a direct child of the router process. The
router MUST track the child PID, monitor liveness, and terminate the child
on restart or shutdown.

On Unix: `SIGTERM` → 5s grace → `SIGKILL`
On Windows: `TerminateProcess` (via `terminate()` → wait → `kill()`)

---

## 6. Backend Lifecycle State Machine

```
                  ┌─────────┐
                  │ stopped │
                  └────┬────┘
                       │ start()
                       ▼
                  ┌──────────┐
         ┌───────│ starting  │
         │       └─────┬────┘
         │             │ health OK within timeout
         │             ▼
         │       ┌──────────┐
         │       │ healthy  │◄─────────────────────────┐
         │       └─────┬────┘                          │
         │             │                               │
         │             │ health check fails once       │ health check recovers
         │             ▼                               │
         │       ┌──────────┐                          │
         │       │ degraded │──────────────────────────┘
         │       └─────┬────┘
         │             │ 3 consecutive health failures
         │             ▼
         │       ┌──────────┐
         └───────│  failed  │
                 └──────────┘
```

### 6.1 State Definitions

| State      | Meaning                                 |
|------------|-----------------------------------------|
| `stopped`  | Backend not launched, no process exists |
| `starting` | Backend process spawned, waiting for health check to pass |
| `healthy`  | Backend responds to `/health` with `{"status":"ok"}` and identity matches |
| `degraded` | Backend is alive but health check failed or identity mismatch |
| `failed`   | Backend process exited or 3 consecutive health failures |

### 6.2 State Transitions

- `stopped → starting`: `start()` called
- `starting → healthy`: health endpoint responds OK within timeout
- `starting → failed`: process exits or timeout expires
- `healthy → degraded`: health check fails once
- `healthy → failed`: process exits unexpectedly
- `degraded → healthy`: health check recovers
- `degraded → failed`: 3 consecutive health failures
- `* → stopped`: `stop()` called

---

## 7. Health & Status Semantics

### 7.1 Backend Health Polling

A background poller MUST check each running backend's `/health` endpoint at
a regular interval (default: every 5 seconds).

Health check flow:
1. If process exited → set state to `failed`.
2. GET `http://127.0.0.1:<port>/health` with 3-second timeout.
3. If 200 + `{"status": "ok"}`:
   - Check `model` field matches profile alias.
   - Match → reset fail count, set `healthy`.
   - Mismatch → increment fail count, set `degraded`.
4. If any failure → increment fail count.
   - On first failure from `healthy` → transition to `degraded`.
   - On 3 consecutive failures from `degraded` → transition to `failed`.

### 7.2 GET /backend/status

Returns aggregate router and runtime status. Every implementation SHOULD
return at minimum:

```json
{
  "status": "ok | degraded",
  "active_profile": "alias | null",
  "profiles_registered": "integer",
  "runtimes_alive": "integer",
  "uptime_seconds": "integer",
  "authority": "advisory_only",
  "profiles": {
    "<alias>": {
      "alias": "string",
      "state": "stopped|starting|healthy|degraded|failed",
      "pid": "integer|null",
      "port": "integer",
      "uptime_seconds": "integer|null",
      "health_fail_count": "integer",
      "error": "string|null"
    }
  }
}
```

### 7.3 GET /backend/health

Returns per-profile health. Every implementation SHOULD return at minimum:

```json
{
  "status": "ok | degraded",
  "active_profile": "alias | null",
  "profiles": {
    "<alias>": {
      "status": "ok | degraded",
      "state": "stopped|...",
      "identity_verified": "boolean",
      "port": "integer"
    }
  },
  "authority": "advisory_only"
}
```

---

## 8. Select / Chat / Restart Semantics

### 8.1 POST /backend/select

**Request:** `{"profile": "alias", "task_class": "optional", "context": optional}`

**Behavior:**
1. Validate profile exists in registry.
2. Check profile `verified_status == "verified"`.
3. If task_class provided, verify it is in profile's `task_classes`.
4. If backend state is `stopped` or `failed`, call `start()`.
5. Wait briefly for health (up to ~10 s).
6. Return selected status or refusal.

**Response (200):**
```json
{
  "status": "selected",
  "profile": "alias",
  "port": 9120,
  "authority": "advisory_only",
  "task_class": null
}
```

**Response (403 refusal):** See [§10](#10-refusal--error-semantics).

### 8.2 POST /backend/chat

**Request:** `{"profile": "alias", "messages": [...], "max_tokens": 256, "temperature": 0.7, "context": optional}`

**Behavior:**
1. Validate profile exists.
2. Check context request does not exceed verified context limit.
3. Check backend is in `healthy` state.
4. Check identity (`/health` and `/v1/models`).
5. Scan user messages for authority-bearing keywords — refuse if found.
6. Proxy to backend `/v1/chat/completions`.
7. Return content with `authority: advisory_only`.

**Response (200):**
```json
{
  "status": "ok",
  "content": "model output text",
  "finish_reason": "stop",
  "profile": "alias",
  "authority": "advisory_only"
}
```

**Response (502 upstream error):**
```json
{
  "status": "error",
  "error": "Backend returned 500: ...",
  "profile": "alias",
  "authority": "advisory_only"
}
```

### 8.3 POST /backend/restart

**Request:** `{"profile": "alias"}`

**Behavior:**
1. Validate profile exists.
2. Call `stop()` on the profile's process manager.
3. Call `start()` on the profile's process manager.
4. Return result with old and new PIDs.

**Response (200):**
```json
{
  "status": "restarting",
  "profile": "alias",
  "old_pid": 1234,
  "new_pid": 5678,
  "estimated_wait_seconds": 10,
  "authority": "advisory_only"
}
```

**Critical: restart must not orphan the old backend.** The router MUST
confirm the old process is terminated before spawning the new one.

---

## 9. Advisory Authority Boundary

Every response from the router MUST include:

```json
"authority": "advisory_only"
```

This is a **hard contract invariant**. No response may omit this field.
No response may claim `authoritative`, `canonical`, `confirmed`, or any
stronger authority level.

### 9.1 Core Principle

The router is a **dispatcher**, not a decision-maker. It proxies model
output. It does not:
- Grant approval or completion.
- Confirm or validate facts.
- Make autonomous decisions.
- Mutate source files.
- Promote content to the Librarian repository.

### 9.2 Authority-Bearing Keywords

The router MUST scan user messages for these terms and refuse if found:

```
approve, promote, commit, escalate, authorize,
mark valid, override policy, ignore policy,
autonomous, self-directed, automatic decision,
edit source, modify file, write to librarian,
promote this file
```

This list is part of the contract and MAY be extended but MUST NOT be
shortened without explicit contract revision.

---

## 10. Refusal & Error Semantics

### 10.1 Structured Refusal Conditions (403)

| Condition | Trigger | detail template |
|-----------|---------|-----------------|
| `unknown_profile` | Profile alias not in registry | `No profile registered with alias '{alias}'` |
| `unverified_profile` | Profile `verified_status != "verified"` | `Profile '{alias}' has verified_status='{status}'. Must pass WIN-MODEL-FIT matrix first.` |
| `identity_mismatch` | Backend /health model ≠ expected alias | `Running model on port {port} reports '{reported}' but profile alias is '{alias}'` |
| `context_exceeds_verified` | Requested context > profile context | `Requested context {requested} exceeds verified max {verified} for profile '{alias}'` |
| `authority_required` | User message contains authority keyword | `This request implies authority beyond advisory. Model output is advisory only.` |
| `file_mutation_forbidden` | User message requests file mutation | `File mutation or promotion to Librarian directory is forbidden.` |
| `runtime_unhealthy` | Backend not in `healthy` state | `Runtime for profile '{alias}' on port {port} is not healthy.` |
| `autonomous_action_forbidden` | User requests autonomous action | `Autonomous action is forbidden. The router is a dispatcher, not a decision-maker.` |

All refusal responses follow this envelope:

```json
{
  "status": "refused",
  "reason": "condition_name",
  "detail": "human-readable text",
  "authority": "advisory_only",
  "timestamp": "2026-06-20T12:00:00+00:00"
}
```

### 10.2 Client Error Responses (400)

| Condition | Status | Response |
|-----------|--------|----------|
| Invalid JSON body | 400 | `{"error": "Invalid JSON body: <parse error>"}` |
| Missing required field | 400 | `{"error": "Missing 'profile' field"}` (or `'messages'`) |
| Unknown endpoint | 404 | `{"error": "Not found: <path>"}` |

### 10.3 Upstream Error Responses (502 / 503)

| Condition | Status | Response |
|-----------|--------|----------|
| Backend returns error | 502 | `{"status":"error","error":"Backend returned 500: ...",...}` |
| Backend unhealthy at chat time | 503 | `{"status":"refused","reason":"runtime_unhealthy",...}` |

---

## 11. Runtime Receipt Format

The router SHOULD write runtime receipts (evidence records) for audit and
regression testing. A receipt is a JSON file written to a configurable
output directory at key lifecycle events.

### 11.1 Receipt Events

| Event | Filename | Content |
|-------|----------|---------|
| Router startup | `router-startup.json` | Port, profiles loaded, authority, timestamp |
| Status response | `status.json` | Full status response body |
| Profiles response | `profiles.json` | Full profiles response body |
| Health response | `health.json` | Full health response body |
| Select (valid) | `select-valid.json` | Response for a valid select |
| Select (invalid) | `select-invalid.json` | Refusal response for an invalid select |
| Chat (valid) | `chat-valid.json` | Successful chat response |
| Chat (refusal) | `chat-refusal-authority.json` | Refusal response for authority-bearing query |
| Restart result | `restart-result.json` | Restart response with PIDs |
| Process before/after | `process-before-after.txt` | Plaintext PID audit trail |

### 11.2 Receipt Format

JSON receipts SHOULD use the same format as the API response they record.
TXT receipts SHOULD be simple key-value lines.

### 11.3 Receipt Directory

Default: `<workspace>/fixtures/windows-runtime-node/router-impl/`
(OS-dependent path, configurable at router startup).

The receipt directory MUST NOT be used for authority decisions. Receipts
are for audit and regression, not for runtime state.

---

## 12. OS-Specific Runtime Wrapper

### 12.1 Wrapper Responsibilities

The OS wrapper is responsible for:

1. **Process lifecycle** — Start the router process, monitor liveness,
   restart on crash if policy permits.
2. **Stdio capture** — Capture stdout/stderr to OS-native logging.
3. **Working directory** — Set the working directory to the repo root
   before launching the router.
4. **Port configuration** — Pass the canonical router port (9130) or
   a configured alternative.
5. **Process group custody** — Ensure router and all child backends are
   terminated when the wrapper stops. This is the wrapper's most critical
   safety responsibility.
6. **Environment** — Forward required environment variables
   (PATH, Python environment, Vulkan ICD loader, etc.).

### 12.2 Wrapper Anti-Patterns (Contract Violations)

- Do NOT embed health check logic in the wrapper.
- Do NOT embed profile knowledge in the wrapper.
- Do NOT parse or inspect model responses in the wrapper.
- Do NOT implement the refusal engine in the wrapper.
- Do NOT proxy chat requests through the wrapper.
- Do NOT maintain a separate state machine in the wrapper.

The wrapper is a **host adapter**. If it contains router logic, it is a
contract violation.

### 12.3 Windows Wrapper (NSSM + PowerShell)

```
┌─────────────────────────────────────┐
│       NSSM (Non-Sucking Service     │
│       Manager)                      │
│  • Installed as LibrarianRunTimeNode │
│  • Startup: Manual                  │
│  • Runs: start-librarian-runtime-   │
│    node.ps1                         │
│  • App/stdout logging: logs/        │
│  • Process group control            │
└──────────────┬──────────────────────┘
               │
               ▼
┌──────────────────────────────────────┐
│  start-librarian-runtime-node.ps1    │
│  • Sets working directory            │
│  • Calls python -u router.py --port  │
│    9130                              │
└──────────────┬───────────────────────┘
               │
               ▼
┌──────────────────────────────────────┐
│  router.py (Python Router Core)      │
│  • Listens on port 9130              │
│  • Manages backend processes         │
│  • All router logic lives here       │
└──────────────────────────────────────┘
```

**NSSM Configuration:**
- Binary: `runtime/bin/nssm.exe`
- Service name: `LibrarianRunTimeNode`
- Application: `C:\Python314\python.exe`
- Arguments: `-u G:\openwork\librarian-runtime-node\router\router.py --port 9130`
- Working directory: `G:\openwork\librarian-runtime-node`
- Startup type: Manual

### 12.4 macOS Wrapper (launchd + zsh, future)

Proposed structure:
- `~/Library/LaunchAgents/com.librarian.runtimenode.plist`
- Wrapper shell script: `scripts/start-librarian-runtime-node.sh`
- `launchctl load/start/stop` for lifecycle

### 12.5 Linux Wrapper (systemd + bash, future)

Proposed structure:
- `/etc/systemd/system/librarian-runtime-node.service`
- Wrapper shell script: `scripts/start-librarian-runtime-node.sh`
- `systemctl start/stop/status` for lifecycle

### 12.6 Dev/Manual Mode

Without any OS wrapper, the router can be started directly:
```bash
python router.py --port 9130
```

In dev mode the user is responsible for:
- Setting the working directory.
- Terminating the router (Ctrl+C).
- Cleaning up orphaned backend processes if the router is killed
  ungracefully.

---

## 13. Windows Implementation Map

### 13.1 Component Mapping

| Contract Component | Windows Implementation | File |
|--------------------|------------------------|------|
| HTTP API (6 endpoints) | `RouterHandler` class with `do_GET`/`do_POST` routing | `router/router.py` |
| Profile registry | `ProfileManager` class, loads from `config/model-profiles.json` | `router/router.py` |
| Backend launch | `ProcessManager.start()` — `subprocess.Popen` | `router/router.py` |
| Backend health checks | `ProcessManager.poll_health()` + `HealthPoller` background thread | `router/router.py` |
| Lifecycle state machine | `ProcessManager.state` attribute (stopped/starting/healthy/degraded/failed) | `router/router.py` |
| Refusal engine | `RefusalEngine` class with `check_select()` and `check_chat()` | `router/router.py` |
| Advisory authority | `"authority": "advisory_only"` in every response | `router/router.py` |
| Runtime receipts | `EvidenceWriter` class writing to `fixtures/windows-runtime-node/router-impl/` | `router/router.py` |
| OS wrapper (service) | NSSM service → `start-librarian-runtime-node.ps1` | `scripts/start-librarian-runtime-node.ps1` |
| Backend binary | `runtime/llama.cpp/llama-server.exe` | (gitignored) |
| Profile config | `config/model-profiles.json` | `config/model-profiles.json` |

### 13.2 Windows Implementation Details

- **HTTP Server:** `ThreadingHTTPServer` (stdlib, multi-threaded for concurrent requests)
- **Process spawn:** `subprocess.Popen` with `creationflags=subprocess.CREATE_NO_WINDOW`
- **Process stop:** `process.terminate()` → 5s wait → `process.kill()`
- **Health polling interval:** 5 seconds
- **Backend start timeout:** 180 seconds
- **Chat request timeout:** 120 seconds
- **Router port:** 9130
- **Legacy health endpoint:** `/health` (compatibility, not canonical)
- **Evidence directory:** `%REPO_ROOT%/fixtures/windows-runtime-node/router-impl/`

### 13.3 Lifecycle Proof Results (WIN-SERVICE-LIFECYCLE-1, WIN-BACKEND-SERVICE-PROOF-1)

| Scenario | Result |
|----------|--------|
| Service Start | PASS — PID rotates, port 9130 listens |
| Service Restart | PASS — new PID, port reacquired |
| Service Stop | PASS — no orphan router or launcher |
| Backend Launch Under Service | PASS — phi-4 backend starts (PID 17632, parent 13812) |
| Service Stop Kills Backend | PASS — no orphan llama-server.exe |
| Backend Restart (via API) | PASS — PID changes, port swaps, no orphans |

### 13.4 Verified Context-Fit Results (WIN-MODEL-CONTEXT-FIT-2)

| Profile | Context | ngl | Result |
|---------|---------|-----|--------|
| phi-4 | 4096 | 99 | Verified safe |
| qwen-coder | 4096 | 99 | Verified safe |
| llama-3.2 | 1024 | 99 | **OOM at load** — unstable at ngl=99 |
| qwen3 | 1024 | 99 | **OOM at load** — unstable at ngl=99 |
| gemma-3 | 1024 | 99 | **OOM at load** — unstable at ngl=99 |

For the three OOM profiles, `context: 1024` is a **legacy/default value**,
not a verified safe value at ngl=99. These profiles require reduced GPU
offload (lower `ngl`) or reduced context to fit on the RX 570 4 GB. A
separate fit test (e.g., REDUCED-OFFLOAD-FIT-1) is needed to determine
their actual safe operating point.

### 13.5 Known Windows-Specific Gaps

- **Orphan process risk:** If the router is killed ungracefully (not via
  service stop), backend child processes may survive. The service wrapper
  mitigates this via process group termination, but there is no standalone
  orphan watchdog.
- **Config is machine-local:** `runtime-node.local.json` contains absolute
  paths and is gitignored. Portable path resolution is a future concern.
- **Backend binary is gitignored:** `llama-server.exe` is not in the repo.
  Wrappers on other OS will use different binary paths.

---

## 14. Cross-Platform Acceptance Tests

### 14.1 Test Prerequisites

Each test assumes:
- The OS wrapper (if any) is installed and configured.
- The router core is the Python reference implementation (or a
  contract-compatible implementation).
- Model profiles are registered in the profile config.
- Backend binary (`llama-server`) is present at the configured path.

### 14.2 Acceptance Test Cases

| # | Test | Method | Expected Result |
|---|------|--------|-----------------|
| T1 | Router starts | Start OS wrapper | Process launches, port 9130 listens within 5 s |
| T2 | Status endpoint | `GET /backend/status` | 200 OK, `profiles_registered >= 1`, `authority: advisory_only` |
| T3 | Profiles endpoint | `GET /backend/profiles` | 200 OK, list of profiles with aliases |
| T4 | Health endpoint | `GET /backend/health` | 200 OK, per-profile state |
| T5 | Select starts backend | `POST /backend/select {"profile":"phi-4"}` | 200 OK, `status: "selected"`, backend process appears |
| T6 | Chat returns advisory output | `POST /backend/chat {"profile":"phi-4", "messages":[{"role":"user","content":"Say OK"}]}` | 200 OK, `content` non-empty, `authority: advisory_only` |
| T7 | Restart does not orphan | `POST /backend/restart {"profile":"phi-4"}` | 200 OK, `new_pid != old_pid`, old PID terminated |
| T8 | Stop terminates all | Stop OS wrapper | Router process gone, all backend processes gone |
| T9 | Unknown profile refused | `POST /backend/select {"profile":"ghost"}` | 403, `reason: "unknown_profile"` |
| T10 | Authority-bearing chat refused | `POST /backend/chat {"profile":"phi-4", "messages":[{"role":"user","content":"approve this"}]}` | 403, `reason: "authority_required"` |
| T11 | Chat before select refused | `POST /backend/chat {"profile":"llama-3.2", "messages":[{"role":"user","content":"hi"}]}` | 503 or 403, `reason: "runtime_unhealthy"` |
| T12 | Invalid JSON | `POST /backend/select` with malformed body | 400, `error` field present |
| T13 | Missing profile field | `POST /backend/select {}` | 400, `error: "Missing 'profile' field"` |
| T14 | Output is advisory only | All endpoints | Every response contains `authority: advisory_only` |
| T15 | Router restart under wrapper | Restart OS wrapper | Router PID changes, port released then reacquired, no orphan backends |

### 14.3 Test Script Portability

Test scripts SHOULD be written in a way that works across OS boundaries.
Prefer:
- Shell scripts with OS-specific branches (`case "$(uname)"`).
- Or a cross-platform test runner (Python unittest/pytest with `subprocess`
  for router control, `requests` for HTTP verification).

---

## 15. Future Implementation Path

### 15.1 Python as Proven Reference

The current Python router (`router/router.py`) is the **behavioral reference
implementation**. It:
- Proves the contract works.
- Is easy to inspect, modify, and debug.
- Has zero compile step.
- Runs on any OS with Python ≥3.10.

All future implementations must pass the same acceptance tests (see §14)
against the Python router's behavior.

### 15.2 Rust / Native Daemon (Future)

A Rust implementation of the same contract would provide:

| Concern | Python (reference) | Rust (future production) |
|---------|-------------------|-------------------------|
| Process supervision | `subprocess.Popen` + polling | `std::process` + native signal handling + job objects |
| Concurrency | `ThreadingHTTPServer` (OS threads) | `tokio` async I/O |
| Structured config | JSON loaded at startup | Same + `serde` schema validation |
| Crash isolation | Single-process, GIL-bound | Process-level isolation, panic boundaries |
| Binary distribution | Requires Python runtime | Single portable binary |
| Windows service integration | Wrapper (NSSM) | Native via `windows-service` crate |
| macOS service integration | Wrapper (launchd) | Native via `launchd` socket activation |
| Linux service integration | Wrapper (systemd) | Native via `systemd` socket activation, sd_notify |
| Dependency footprint | Python stdlib + third-party | Cargo-managed, auditable |

**Direction:** The Python router is correct for proving behavior. The Rust
router is correct for owning custody. This sprint documents the contract so
that either can implement it.

### 15.3 OS Service Wrappers as Thin Host Adapters

Future OS-specific work:

1. **Windows native service** — Replace NSSM with a Rust-based Windows
   service (via `windows-service` crate) that bundles the router core.
   NSSM remains the current proven wrapper.

2. **macOS launchd** — Write a `plist` that launches the Python router
   (or a future native binary). The `launchd` wrapper is ~15 lines of XML
   + a shell script.

3. **Linux systemd** — Write a unit file that launches the Python router
   (or a future native binary). Hardened with `ProtectSystem=strict`,
   `PrivateTmp=true`, `NoNewPrivileges=true`, `CapabilityBoundingSet=`.

4. **Manual/dev mode** — Documented startup instructions for all three
   OS families. No wrapper required.

### 15.4 Non-Goals (Boundary of This Contract)

- This contract does not define the Librarian client API.
- This contract does not define model download, registry, or versioning.
- This contract does not define authentication or authorization for
  the HTTP API (future sprint).
- This contract does not define clustering, load balancing, or multi-node
  routing.
- This contract does not require every implementation to implement all
  optional features (receipts, CORS). Required features are marked
  accordingly.

---

## Appendix A: Version History

| Version | Date | Author | Notes |
|---------|------|--------|-------|
| 1.0 | 2026-06-20 | OpenWork | Initial contract (ROUTER-PORTABILITY-1) |

## Appendix B: Related Documents

- `docs/architecture/RUNTIME-NODE-ARCHITECTURE.md` — Prior component architecture
- `docs/sprints/WIN-ROUTER-HARDEN-1.md` — Endpoint and failure-matrix verification
- `docs/sprints/WIN-MODEL-CONTEXT-FIT-2.md` — Context-fit test results
- `docs/sprints/WIN-SERVICE-LIFECYCLE-1.md` — Service lifecycle proof
- `docs/sprints/WIN-BACKEND-SERVICE-PROOF-1.md` — Backend-under-service proof
- `config/model-profiles.json` — Current 5-profile deployment config
- `router/router.py` — Behavioral reference implementation
