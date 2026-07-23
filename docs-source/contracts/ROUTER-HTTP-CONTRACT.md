# Router HTTP Contract (Frozen)

> Frozen contract for the Librarian Runtime Node HTTP API.
> Captures the exact externally visible behavior of the Rust router
> at commit `63780bf` before any native daemon changes.
>
> Sprint: ROUTER-CONTRACT-TESTS-1
> Status: FROZEN — changes to router behavior must not violate this contract
> Authority: advisory_only

---

## Table of Contents

1. [Base URL & Transport](#1-base-url--transport)
2. [Authentication](#2-authentication)
3. [CORS](#3-cors)
4. [Endpoint Inventory](#4-endpoint-inventory)
5. [Response Envelope](#5-response-envelope)
6. [Endpoint Specifications](#6-endpoint-specifications)
    - [6.1 GET /backend/status](#61-get-backendstatus)
    - [6.2 GET /backend/profiles](#62-get-backendprofiles)
    - [6.3 GET /backend/health](#63-get-backendhealth)
    - [6.4 GET /v1/models](#64-get-v1models)
    - [6.5 POST /backend/select](#65-post-backendselect)
    - [6.6 POST /backend/stop](#66-post-backendstop)
    - [6.7 POST /v1/chat/completions](#67-post-v1chatcompletions)
7. [Error Responses](#7-error-responses)
8. [Safety Boundaries](#8-safety-boundaries)
9. [Response Shape Schemas](#9-response-shape-schemas)
10. [Contract Invariants](#10-contract-invariants)

---

## 1. Base URL & Transport

| Property | Value |
|----------|-------|
| Base URL | `http://127.0.0.1:9130` |
| Protocol | HTTP 1.1 |
| Host | `127.0.0.1` (localhost only) |
| Port | `9130` |
| Default Max Body | 10,485,760 bytes (10 MB) |

## 2. Authentication

| Mode | Behavior |
|------|----------|
| **Disabled (default)** | `require_auth: false` — all requests pass through without header checks |
| **Enabled** | `require_auth: true`, `ROUTER_AUTH_TOKEN` must be set |

**Token verification (when enabled):**
- The `Authorization` header is compared as an **exact string match** against the configured token
- **Not** a "Bearer \<token\>" check — the raw header value must equal the configured token exactly
- Missing header → **401** with no body
- Invalid header → **401** with no body
- Valid header → normal processing

**Environment variables:**
- `ROUTER_AUTH_TOKEN` — the token value to require
- `ROUTER_REQUIRE_AUTH` — set to `"true"` or `"1"` to enable

## 3. CORS

The router applies a permissive CORS layer to all routes:
```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, OPTIONS
Access-Control-Allow-Headers: *
```

## 4. Endpoint Inventory

Seven endpoints are part of the frozen contract:

| # | Method | Path | Auth | Description |
|---|--------|------|------|-------------|
| 1 | GET | `/backend/status` | Optional | Router + all profile runtime statuses |
| 2 | GET | `/backend/profiles` | Optional | List all registered model profiles |
| 3 | GET | `/backend/health` | Optional | Per-profile health with identity checks |
| 4 | GET | `/v1/models` | Optional | OpenAI-compatible model listing |
| 5 | POST | `/backend/select` | Optional | Select/start a model backend |
| 6 | POST | `/backend/stop` | Optional | Stop one or all model backends |
| 7 | POST | `/v1/chat/completions` | Optional | OpenAI-compatible chat endpoint |

## 5. Response Envelope

**Every response MUST contain:**
```json
"authority": "advisory_only"
```

This is a hard contract invariant. No response may omit this field and no
response may claim any other authority level.

## 6. Endpoint Specifications

### 6.1 GET /backend/status

**Purpose:** Returns aggregate router and all profile runtime statuses.

**Request:** None (no query parameters, no body)

**Success (200):**
```json
{
  "status": "ok | degraded",
  "active_profile": "alias | null",
  "profiles_registered": 5,
  "runtimes_alive": 0,
  "uptime_seconds": 183,
  "authority": "advisory_only",
  "profiles": {
    "<alias>": {
      "alias": "string",
      "state": "stopped|starting|healthy|degraded|failed",
      "pid": "integer|null",
      "port": 9120,
      "uptime_seconds": "integer|null",
      "health_fail_count": 0,
      "error": null
    }
  }
}
```

**Shape requirements:**
- `status` — overall router status: `"ok"` when ≥1 backend healthy, `"degraded"` otherwise
- `active_profile` — first healthy backend alias, or `null` if none healthy
- `profiles_registered` — total profiles loaded from config
- `runtimes_alive` — count of backends in `"healthy"` state
- `uptime_seconds` — seconds since router process started
- `profiles` — object keyed by alias, each value is a profile status object
- `authority` — always `"advisory_only"`

**Error responses:** None (always 200)

---

### 6.2 GET /backend/profiles

**Purpose:** Lists all registered model profiles with metadata. 
Does not expose full filesystem paths.

**Request:** None

**Success (200):**
```json
{
  "authority": "advisory_only",
  "profiles": [
    {
      "alias": "phi-4",
      "task_classes": ["general_advisory", "summarization_advisory"],
      "verified": true,
      "port": 9120,
      "model_file": "microsoft_Phi-4-mini-instruct-Q4_K_M.gguf"
    }
  ]
}
```

**Shape requirements:**
- `profiles` — array of profile summary objects
- Each profile has: `alias` (string), `task_classes` (string[]), `verified` (bool), `port` (int), `model_file` (string — filename only, never path)
- `authority` — always `"advisory_only"`
- No full filesystem paths (model_path, binary path, etc.) are exposed

**Error responses:** None (always 200)

---

### 6.3 GET /backend/health

**Purpose:** Returns per-profile health status with identity verification.

**Request:** None

**Success (200):**
```json
{
  "status": "ok | degraded",
  "active_profile": "alias | null",
  "profiles": {
    "<alias>": {
      "status": "ok | degraded",
      "state": "healthy|stopped|starting|degraded|failed",
      "identity_verified": true | false,
      "port": 9120
    }
  },
  "authority": "advisory_only"
}
```

**Shape requirements:**
- `status` — overall: `"ok"` when all backends healthy, `"degraded"` otherwise
- `active_profile` — first healthy backend, or `null`
- `profiles` — object keyed by alias with per-profile health
- `authority` — always `"advisory_only"`

**Error responses:** None (always 200)

---

### 6.4 GET /v1/models

**Purpose:** OpenAI-compatible model listing endpoint.

**Request:** None

**Success (200):**
```json
{
  "object": "list",
  "data": [
    {
      "id": "phi-4",
      "object": "model",
      "created": 1712345678,
      "owned_by": "librarian-runtime-node",
      "permission": [],
      "root": "phi-4",
      "parent": null
    }
  ],
  "authority": "advisory_only"
}
```

**Shape requirements:**
- `object` — always `"list"`
- `data` — array of model objects
- Each model has: `id` (alias), `object` (`"model"`), `created` (unix timestamp),
  `owned_by` (always `"librarian-runtime-node"`), `permission` (empty array),
  `root` (same as `id`), `parent` (null)
- `authority` — always `"advisory_only"`
- No filesystem paths or model file metadata exposed

**Error responses:** None (always 200)

---

### 6.5 POST /backend/select

**Purpose:** Select and start a model profile backend.

**Request body:**
```json
{
  "profile": "phi-4",           // required
  "task_class": "general_advisory",  // optional
  "context": 4096                    // optional
}
```

**Success (200):**
```json
{
  "status": "selected",
  "profile": "phi-4",
  "port": 9120,
  "authority": "advisory_only",
  "task_class": null
}
```

**Refusal — unknown profile (403):**
```json
{
  "status": "refused",
  "reason": "unknown_profile",
  "detail": "No profile registered with alias '__nonexistent__'",
  "authority": "advisory_only",
  "timestamp": "2026-06-23T05:20:13.105927100+00:00"
}
```

**Refusal — invalid task_class (403):**
```json
{
  "status": "refused",
  "reason": "unknown_profile",
  "detail": "Task class '__bogus__' not declared for profile 'phi-4'. Declared: [...]",
  "authority": "advisory_only",
  "timestamp": "..."
}
```

**Unprocessable — missing profile field (422):**
```
"Failed to deserialize the JSON body into the target type: missing field `profile` at line 1 column 2"
```

**Content type error — invalid JSON (400):**
```
"Failed to parse the request body as JSON: ..."
```

**Return codes:**
| Status | Condition |
|--------|-----------|
| 200 | Profile selected, backend process created/started |
| 403 | Refusal: unknown profile, invalid task_class |
| 422 | Missing required `profile` field |
| 400 | Invalid or unparseable JSON body |
| 413 | Request body exceeds 10 MB limit |

---

### 6.6 POST /backend/stop

**Purpose:** Stop one or all running model backends.

**Request body:**
```json
{
  "profile": "phi-4"    // optional — omit to stop all
}
```

**Success — profiles stopped (200):**
```json
{
  "status": "stopped",
  "stopped": ["phi-4"],
  "not_found": [],
  "authority": "advisory_only"
}
```

**Error — no backends running (400):**
```json
{
  "status": "error",
  "detail": "No backends running"
}
```

**Return codes:**
| Status | Condition |
|--------|-----------|
| 200 | Backend(s) stopped successfully |
| 400 | No backends running |

---

### 6.7 POST /v1/chat/completions

**Purpose:** OpenAI-compatible chat completions endpoint.
Proxies to the selected backend's `/v1/chat/completions`.

**Request body:**
```json
{
  "model": "phi-4",
  "messages": [{"role": "user", "content": "Hello"}],
  "max_tokens": 256,
  "temperature": 0.7
}
```

**Success — proxy response (200):** Passes through the backend's full
OpenAI-compatible response verbatim.
```json
{
  "choices": [
    {
      "message": {"content": "Hello!"},
      "finish_reason": "stop"
    }
  ]
}
```

**Error — no active backend (503):**
```json
{
  "error": "No active backend. Use /backend/select first.",
  "authority": "advisory_only"
}
```

**Error — empty messages field (400):**
```json
{
  "error": "Missing 'messages' field"
}
```

**Error — backend proxy failure (502):**
```json
{
  "error": "Backend request failed: ..."
}
```

**Content type error — invalid JSON (400):**
```
"Failed to parse the request body as JSON: expected ident at line 1 column 2"
```

**Return codes:**
| Status | Condition |
|--------|-----------|
| 200 | Successful proxy to backend (backend response passed through) |
| 400 | Missing/empty `messages` field |
| 502 | Backend unreachable or returned error |
| 503 | No active backend selected |
| 413 | Request body exceeds 10 MB limit |

---

## 7. Error Responses

### 7.1 Summary Table

| Condition | HTTP Status | Body |
|-----------|-------------|------|
| Auth disabled — no token needed | 200 | Normal response |
| Auth enabled — missing token | **401** | **Empty body** (bare status code) |
| Auth enabled — invalid token | **401** | **Empty body** (bare status code) |
| Auth enabled — valid token | 200 | Normal response |
| Invalid JSON body | 400 | Text description of parse error |
| Missing required field | 422 | Serde deserialization error message |
| Body too large (>10 MB) | 413 | `"Failed to buffer the request body: length limit exceeded"` |
| Unknown endpoint | 404 | (Not explicitly tested) |
| Refusal (select/chat) | 403 | Structured JSON refusal |
| Backend proxy error | 502 | JSON error object |
| Backend not selected | 503 | JSON error object |

### 7.2 Structured Refusal Schema

All refusal responses (403) follow this envelope:
```json
{
  "status": "refused",
  "reason": "condition_name",
  "detail": "human-readable explanation",
  "authority": "advisory_only",
  "timestamp": "ISO-8601 UTC"
}
```

### 7.3 Refusal Conditions

| Condition | Trigger |
|-----------|---------|
| `unknown_profile` | Profile alias not in registry |
| `unverified_profile` | Profile verified_status != "verified" |
| `identity_mismatch` | Backend health model ≠ expected alias |
| `context_exceeds_verified` | Requested context > verified max |
| `authority_required` | User message contains authority-bearing keywords |
| `file_mutation_forbidden` | User message requests file mutation |
| `runtime_unhealthy` | Backend not in healthy state |
| `autonomous_action_forbidden` | User requests autonomous action |
| `task_class_check_needed` | Requested task_class not in profile's declared classes |

---

## 8. Safety Boundaries

### 8.1 Body Size Limit

Maximum request body: **10,485,760 bytes** (10 MB)
- Bodies exceeding this limit receive HTTP **413** with no JSON body processed.
- The limit is enforced by axum's `DefaultBodyLimit` layer.

### 8.2 No Secret Leakage

The router must not leak the following in any response:
- API keys or tokens (auth, backend, or any)
- Full filesystem paths (model paths, binary paths, config paths)
- Credentials or passwords
- Environment variable values

Profiles endpoint (`/backend/profiles`) exposes only the `model_file` filename,
never the full `model_path`.

### 8.3 No Model Execution Required

Contract tests are designed to run without requiring model execution:
- GET endpoints are fully testable without any active backends.
- POST /backend/select is testable with invalid profiles.
- POST /backend/stop is testable with no backends.
- POST /v1/chat/completions is testable without selected backends.
- Refusal conditions do not require model execution.

### 8.4 Advisory Authority Boundary

All responses carry `"authority": "advisory_only"`. The router is a
dispatcher, not a decision-maker. It does not grant approval, confirm
facts, make autonomous decisions, or mutate source files.

---

## 9. Response Shape Schemas

### GET /backend/status
```
{
  status: string("ok"|"degraded"),
  active_profile: string|null,
  profiles_registered: integer,
  runtimes_alive: integer,
  uptime_seconds: integer,
  authority: "advisory_only",
  profiles: {
    "<alias>": {
      alias: string,
      state: string("stopped"|"starting"|"healthy"|"degraded"|"failed"),
      pid: integer|null,
      port: integer,
      uptime_seconds: integer|null,
      health_fail_count: integer,
      error: string|null
    }
  }
}
```

### GET /backend/profiles
```
{
  authority: "advisory_only",
  profiles: [{
    alias: string,
    task_classes: string[],
    verified: boolean,
    port: integer,
    model_file: string
  }]
}
```

### GET /backend/health
```
{
  status: string("ok"|"degraded"),
  active_profile: string|null,
  profiles: {
    "<alias>": {
      status: string("ok"|"degraded"),
      state: string,
      identity_verified: boolean,
      port: integer
    }
  },
  authority: "advisory_only"
}
```

### GET /v1/models
```
{
  object: "list",
  data: [{
    id: string,
    object: "model",
    created: integer,
    owned_by: "librarian-runtime-node",
    permission: [],
    root: string,
    parent: null
  }],
  authority: "advisory_only"
}
```

---

## 10. Contract Invariants

The following must never be violated by any router implementation change:

1. **Every response carries `"authority": "advisory_only"`.**
2. **GET /backend/status, /backend/profiles, /backend/health, /v1/models always return 200.**
3. **Unknown profiles always return 403 with reason `unknown_profile`.**
4. **Missing required fields return 4xx (422 or 400).**
5. **Auth failures (when enabled) return 401 with no body.**
6. **Bodies over 10 MB return 413.**
7. **No full filesystem paths are leaked in responses.**
8. **The profiles endpoint exposes model_file as filename only.**
9. **The chat pass-through endpoints (v1/chat/completions) never trigger
    long-running model generation in contract tests (503/502 is acceptable
    when no backend is selected).**
10. **The /backend/stop endpoint gracefully handles the no-backends case.**
11. **The router does not require authentication by default.**
12. **Auth token comparison is exact (not Bearer-prefixed).**

---

*Last updated: 2026-06-23*
*Sprint: ROUTER-CONTRACT-TESTS-1*
*Frozen at: librarian-runtime-node 63780bf, TheLibrarian-main 1e32002*
