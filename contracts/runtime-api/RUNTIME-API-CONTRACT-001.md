# RUNTIME-API-CONTRACT-001.md — Runtime API Contract

**Version:** 1.0.0
**Status:** Canonical (owner-approved lock, 2026-08-06)
**Sealed suite ID:** `RUNTIME-API-SCHEMA-001`
**Last Updated:** 2026-08-06

---

## 0. Purpose

This contract defines the observation boundary through which consumers query a
Librarian Rust node's validated runtime state. It is the migration boundary for
RUST-MIGRATION-M0B: **the contract is the boundary, not the code.**

The success criterion is not "the Rust node has HTTP endpoints." It is:

> A consumer can query a Rust node and receive **trustworthy evidence-backed
> runtime state**.

The API is a **governed projection of state** — it reports what the sealed
startup protocol proved; it never executes or authorizes anything.

---

## 1. Scope

### 1.1 The API exposes

- Runtime state exposure — the current lifecycle state of the node
- Health observation — liveness of the servable runtime
- Receipt retrieval — the canonical startup receipt (RECEIPT-SCHEMA-001)
- Governed availability status — whether the node is in governed execution

### 1.2 The API explicitly does NOT

- Execute or trigger startup (startup runs once, before the API binds)
- Schedule work
- Mutate capabilities
- Change authority or governance state
- Implement MCP behavior
- Accept any state-changing request

### 1.3 Boundary shape (correct)

```
Request
  |
  v
Read validated runtime state (sealed StartupOutcome)
  |
  v
Return status/evidence
```

### 1.4 Boundary shape (forbidden)

```
Request
  |
  v
Trigger startup
  |
  v
Modify authority state
```

---

## 2. Lifecycle State Model

### 2.1 State transitions

```
INITIALIZING
      |
      v
STARTUP_COMPLETE
      |
      v
SERVABLE_RUNTIME

Failure path:

INITIALIZING
      |
      v
STARTUP_FAILED          → process exits before API bind (no serving)
```

`runtime_state` wire values (SCREAMING_SNAKE_CASE):

| State | Meaning | Observable via API |
|-------|---------|--------------------|
| `INITIALIZING` | Node process started; 6-phase protocol executing | No (pre-bind) |
| `STARTUP_COMPLETE` | All startup checks passed; receipt sealed | Yes, transitional |
| `SERVABLE_RUNTIME` | Node available to consumers | Yes |
| `STARTUP_FAILED` | One or more startup checks failed | No (process exits pre-bind) |

### 2.2 SERVABLE_RUNTIME preconditions (all required)

The API MUST NOT report `SERVABLE_RUNTIME` unless **all** of:

1. A startup receipt exists (sealed `StartupOutcome` in memory)
2. The receipt status is `GOVERNED_EXECUTION`
3. Governance verification passed (`governance_verified == true`)
4. Runtime state is observable (receipt + status retrievable without side effects)

### 2.3 Availability invariant

A node is not available because the process exists. A node is available because
**startup succeeded ∧ receipt exists ∧ governance state is valid ∧ runtime
state can be observed.**

`STARTUP_COMPLETE → SERVABLE_RUNTIME` is the formal transition gate
(RUST-M0B-3). The router binds and serves only after the transition.

### 2.4 State contract alignment

The lifecycle states above are runtime-lifecycle values. Wire representation is
provided by the `RuntimeLifecycleState` contract type in `librarian-contracts`
(see contract surface manifest `CONTRACT-SURFACE-MANIFEST-001`).

> **`RuntimeLifecycleState` is an observational contract representation derived
> from runtime state. It does not authorize transitions. Lifecycle transitions
> remain owned by the startup/runtime state machine.**

This prevents future code from doing:

```
API request
  |
  v
change RuntimeLifecycleState
```

The API may read `RuntimeLifecycleState`; it MUST NOT write it.

---

## 3. Endpoint Surface

Minimal and read-only. **No mutation endpoints exist in this contract.**

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/health` | Liveness; whether the servable runtime is up |
| `GET` | `/runtime/status` | Governed availability status with provenance |
| `GET` | `/runtime/receipt` | The canonical startup receipt (RECEIPT-SCHEMA-001) |

Semantics:

| Condition | Response |
|-----------|----------|
| `SERVABLE_RUNTIME` | `200 OK` on all three endpoints |
| Runtime not servable (defensive; normally unreachable due to pre-bind exit) | `503 Service Unavailable` |
| Unknown path | `404 Not Found` |
| State-changing verb on any endpoint (POST/PUT/DELETE/PATCH) | `405 Method Not Allowed` with `Allow: GET` |
| Malformed request (non-GET routing, body present) | `400 Bad Request` |

Failure behavior: on `STARTUP_FAILED`, the process exits before binding — there
is no API to serve. The `503` branch exists only as a defensive contract
statement for unexpected runtime conditions (e.g., sealed state unavailable).

### 3.1 `/health` semantics (explicit)

`GET /health` is **runtime health according to governed availability state** —
NOT a "process is alive" liveness probe.

| `runtime_state` | `health` | HTTP |
|-----------------|----------|------|
| `SERVABLE_RUNTIME` | `ok` | `200` |
| `INITIALIZING` | `unhealthy` | `503` |
| `STARTUP_FAILED` | `unhealthy` | `503` |
| `STARTUP_COMPLETE` (transitional, pre-bind) | `unhealthy` | `503` |

`/health` must not become a weaker bypass path: a node is healthy only when it
is in governed availability state.

---

## 4. Response Contracts

All responses are JSON and carry provenance: the caller must be able to answer
*what node answered, what state was observed, and which receipt proves it.*

### 4.1 `GET /health`

```json
{
  "node_id": "WINPC-BIG-PICKLE",
  "runtime_state": "SERVABLE_RUNTIME",
  "health": "ok",
  "observed_at": "2026-08-06T22:40:00Z"
}
```

- `health`: `ok` when `runtime_state == SERVABLE_RUNTIME`; `unavailable` otherwise
- `observed_at`: RFC 3339 UTC timestamp of the observation (variable field)

### 4.2 `GET /runtime/status`

```json
{
  "node_id": "WINPC-BIG-PICKLE",
  "runtime_state": "SERVABLE_RUNTIME",
  "startup_receipt_id": "WINDOWS-STARTUP-20260807-022831",
  "governance_commit": "6be76216a8048492526c4ca0ae751b6d2d507185",
  "startup_status": "GOVERNED_EXECUTION",
  "checks_passed": 6,
  "checks_failed": 0,
  "observed_at": "2026-08-06T22:40:00Z"
}
```

The deterministic fields (`runtime_state`, `startup_receipt_id`,
`governance_commit`, `startup_status`, `checks_passed`, `checks_failed`) derive
from the sealed startup receipt and MUST match the M0A evidence receipts
field-for-field. `observed_at` is the only variable field.

### 4.3 `GET /runtime/receipt`

Returns the full canonical startup receipt (RECEIPT-SCHEMA-001), exactly the
sealed `StartupReceipt` — no additional wrapping. Deterministic facts MUST
match the M0A evidence receipts.

**Retrieval semantics (evidence-backed):**

> The runtime API returns the receipt **it observed from the sealed startup
> outcome**. It does not regenerate, transform, or normalize receipts.

A regenerated receipt would create a second evidence event:

```
Correct:
Startup
  |
  v
Receipt A
  |
  v
API returns Receipt A

Incorrect:
Startup
  |
  v
Receipt A
              API request
              |
              v
              Generate Receipt B
```

The receipt returned by this endpoint is the exact receipt sealed by the
startup protocol (same `receipt_id`, same timestamp, same bytes).

---

## 5. Authority Restrictions

### 5.1 The Runtime API

- MAY observe runtime state
- MAY report runtime state
- MAY expose evidence (receipts, deterministic facts)

### 5.2 The Runtime API MUST NOT

- Authorize any action
- Mutate governance state
- Mutate capability state
- Bypass the startup lifecycle
- Trigger or re-run startup
- Create, revoke, or alter authority

### 5.3 Enforcing invariant

The API is an **observation boundary, not an execution authority**. This
preserves the existing invariants:

```
Observation ≠ Authority
Registry     ≠ Authority
Recovery     ≠ Authorization
```

Enforcement: the endpoint handlers are read-only projections over a sealed
`StartupOutcome`; there is no write path, no state mutation, and no code path
that invokes the startup engine from a request.

---

## 6. Equivalence Rules

- `GET /runtime/status` and `GET /runtime/receipt` deterministic fields MUST
  match the M0A evidence receipts (`evidence/phase0/rust-core/m0/`) for the
  same node and governance commit.
- `observed_at` is a variable field (RFC 3339 UTC).
- Responses are not byte-identical artifacts; they are governed projections of
  the same sealed state (equivalence is semantic, per
  `docs/architecture/THREE-WAY-EQUIVALENCE-PROTOCOL.md`).

---

## 7. References

- M0B work order: `work-orders/RUST-MIGRATION-M0B.md`
- M0A completion (sealed): `work-orders/RUST-MIGRATION-M0A-COMPLETION.md`
- Startup receipt contract: `contracts/startup/STARTUP-OUTPUT-CONTRACT.md` (RECEIPT-SCHEMA-001)
- Startup protocol: `contracts/startup/STARTUP-PROTOCOL.md`
- Conformance spec §9 divergence protocol: `docs/architecture/RUST-CORE-CONFORMANCE-SPECIFICATION.md`
