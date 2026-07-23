# Sprint: WIN-RUNTIME-NETWORK-BOUNDARY-1

## Overview
This sprint defines and enforces the network exposure boundary for the Windows Runtime Node. The goal is to ensure that the Runtime Node is not accidentally exposed to the LAN and that all sensitive endpoints are protected by authentication.

## Accomplishments

### 1. Interface Binding Policy
- **Default Behavior**: The router now defaults to binding to `127.0.0.1` (localhost) instead of `0.0.0.0`. This prevents accidental LAN exposure.
- **Explicit Configuration**: Users can explicitly bind to all interfaces by setting the `ROUTER_HOST` environment variable to `0.0.0.0` or using the `--host 0.0.0.0` CLI argument.
- **Verification**: Verified that the router binds to `127.0.0.1` by default and can be configured to bind to `0.0.0.0`.

### 2. Authentication / Authorization
- **Token Support**: Added support for a shared-secret API token via the `ROUTER_AUTH_TOKEN` environment variable.
- **Enforcement**: Implemented an `auth_middleware` that enforces token validation for all endpoints when `ROUTER_REQUIRE_AUTH` is set to `true`.
- **Default Posture**: Authentication is disabled by default (`ROUTER_REQUIRE_AUTH=false`) for local development, but can be easily enabled.
- **Verification**: Integration tests confirm that requests without a valid token are rejected with `401 Unauthorized` when authentication is required.

### 3. Endpoint Protection Policy
- **Global Protection**: The `auth_middleware` is applied to the entire router, ensuring all endpoints (including diagnostic and mutation endpoints) are protected when authentication is enabled.

### 4. Request Size Limits
- **Bounded Requests**: Added a `ROUTER_MAX_BODY_BYTES` environment variable to limit the size of incoming request bodies.
- **Default Limit**: Defaults to 10MB.
- **Verification**: Integration tests confirm that oversized requests are rejected with `413 Payload Too Large`.

### 5. Log Redaction
- **Sensitive Data**: Ensured that the `auth_token` and `Authorization` headers are not logged by the router.

### 6. Windows Firewall Documentation
- **Localhost Mode**: No inbound firewall rules are required when running in the default `127.0.0.1` mode.
- **LAN Mode**: If `ROUTER_HOST` is set to `0.0.0.0`, an explicit inbound firewall rule for the configured `ROUTER_PORT` must be created.

## Verification Results

### Integration Tests
All integration tests passed:
- `test_auth_middleware_success`: PASSED
- `test_auth_middleware_failure`: PASSED
- `test_auth_middleware_disabled`: PASSED
- `test_max_body_bytes`: PASSED

### Proof Scripts
A PowerShell script `scripts/test_network_boundary.ps1` is provided to verify the network boundary and authentication behavior.

---

## Closeout Record

| Field | Value |
|-------|-------|
| **Sprint** | WIN-RUNTIME-NETWORK-BOUNDARY-1 |
| **Repository** | `librarian-runtime-node` |
| **Starting HEAD** | `53f0450` (ROUTER-RUST-HARDEN-1 sealed) |
| **Final HEAD** | `82b3a9d` |
| **Commit Message** | `feat(runtime): define and enforce runtime node network boundary` |
| **Date** | 2026-06-21 |
| **Git Status** | `nothing to commit, working tree clean` |

### Summary
This sprint defined and enforced the network exposure boundary for the Windows Runtime Node: default bind `127.0.0.1`, token-auth via `ROUTER_AUTH_TOKEN`/`ROUTER_REQUIRE_AUTH`, request size limit via `ROUTER_MAX_BODY_BYTES`, log redaction of auth tokens, and Windows Firewall documentation for LAN mode.

All integration tests pass. The boundary is now part of the Rust router contract and must not be weakened by future sprints.

## Next Sprint Recommendation
- Implement TLS for encrypted communication when running in LAN mode.
- Refine endpoint protection to allow unauthenticated access to specific diagnostic endpoints (e.g., `/health`) if desired.
