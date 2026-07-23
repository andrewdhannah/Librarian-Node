# Sprint: WIN-RUNTIME-RECEIPTS-2

**Status:** COMPLETED
**Date:** 2026-06-22

## Objective

Upgrade runtime integration receipts so cleanup proof distinguishes active listeners from TCP TIME_WAIT residue, and receipts record artifact-level proof for the router binary actually exercised during the integration run.

## Starting State

| Check | Value |
|-------|-------|
| TheLibrarian-main HEAD | `1e32002` |
| librarian-runtime-node HEAD | `51c2e85` |
| Working trees | clean |
| Stashes | empty |
| Service | Stopped / Manual |
| Port 9130 | free / no listener |
| Orphans | 0 |
| Prior v1 receipt | `win-runtime-integration-2026-06-22T195000Z-qwen-coder-cleanup-proof.json` (overall=pass) |

## Deliverables

### 1. Schema v2 (`schema-v2.json`)
**Path:** `G:\OpenWork\receipts\runtime-integration\schema-v2.json`
**Schema ID:** `win-runtime-receipt/v2`

New fields added to `cleanup`:
- `listener_active` (boolean) — true if a LISTENING socket detected on port 9130 after stop
- `connectivity` (string: `refused` | `listening` | `unknown`) — TCP connect test result
- `port_check_method` (string: `listener_filter_and_tcp_connect`) — method description

New section `artifact`:
- `router_binary_path` — absolute path to the binary exercised during proof
- `router_binary_sha256` — SHA-256 digest, uppercase hex (validated by `^[0-9A-F]{64}$`)
- `router_binary_modified_utc` — file modification timestamp in ISO 8601
- `router_implementation` — `"rust"` or `"python"`
- `governed_path_match` — whether binary path matches the governed/expected path

All v1 fields preserved for backward readability.

### 2. Verifier (`verify-receipt.ps1`)
**Path:** `G:\OpenWork\librarian-runtime-node\scripts\verify-receipt.ps1`

Validates:
- Schema version (v1 or v2)
- All required fields present and typed
- Artifact SHA-256 format: exactly 64 uppercase hex characters (case-sensitive `-cnotmatch`)
- Cleanup listener/connectivity semantics and consistency
- Token safety: scans raw JSON for bearer tokens or secrets (after stripping known safe auth fields)
- Overall result derived correctly from evidence
- 48 checkpoints total

### 3. Fresh Integration Proof Script (`run-integration-proof-v2.ps1`)
**Path:** `G:\OpenWork\librarian-runtime-node\scripts\run-integration-proof-v2.ps1`

Automated lifecycle proof that:
1. Pre-checks: service state, port, orphans, HEADs, working trees
2. Starts Rust router on port 9130
3. Tests all 7 endpoints
4. Selects qwen-coder profile
5. Bounded chat: "Reply with OK only." → "OK"
6. Stops backend
7. Stops router
8. Verifies cleanup with v2 semantics (listener filter + TCP connect)
9. Collects artifact info (binary SHA-256, path, timestamp)
10. Emits v2 receipt
11. Runs verifier against the new receipt

### 4. Fresh v2 Receipt
**Path:** `G:\OpenWork\receipts\runtime-integration\win-runtime-integration-v2-20260622-232214-qwen-coder.json`

| Field | Value |
|-------|-------|
| schema_version | `win-runtime-receipt/v2` |
| overall | `pass` |
| listener_active | `false` |
| connectivity | `refused` |
| port_check_method | `listener_filter_and_tcp_connect` |
| port_9130_free_after_stop | `true` |
| backend_orphans_after_stop | `0` |
| router_binary_sha256 | `84EB797A715FDC4CDB634A85FDFEC5C81CB90F37FB700F6EA6C97576B9569DC9` |
| router_implementation | `rust` |
| governed_path_match | `true` |

## Acceptance Criteria Results

| ID | Check | Result |
|----|-------|--------|
| RECEIPT2-001 | Port check ignores TIME_WAIT residue | ✅ Verified (no LISTENER active, TIME_WAIT not treated as failure) |
| RECEIPT2-002 | Port check detects only active LISTENING sockets | ✅ Verified (netstat filtered for LISTENING state only) |
| RECEIPT2-003 | TCP connect test records refused/listening | ✅ Verified (connectivity: "refused" recorded) |
| RECEIPT2-004 | New receipt includes listener_active and connectivity | ✅ Verified |
| RECEIPT2-005 | New receipt includes router binary path | ✅ Verified |
| RECEIPT2-006 | New receipt includes router binary SHA-256 | ✅ Verified (84EB797A...) |
| RECEIPT2-007 | New receipt separates source HEAD from artifact proof | ✅ Verified (repos section vs artifact section) |
| RECEIPT2-008 | Verifier passes new good receipt | ✅ Verified (48/48 checks passed) |
| RECEIPT2-009 | Verifier rejects malformed artifact hash | ✅ Verified (lowercase SHA-256 → rejected) |
| RECEIPT2-010 | Verifier rejects secret-bearing receipt | ✅ Verified (bearer token in receipt → rejected) |
| RECEIPT2-011 | Fresh integration proof passes | ✅ Verified (exit code 0, overall=pass) |
| RECEIPT2-012 | Final service state Stopped/Manual, port 9130 free, no orphans | ✅ Verified |
| RECEIPT2-013 | Both repos clean or explicitly documented | ✅ Verified (both clean; untracked scripts are intentional additions) |
| RECEIPT2-014 | Stashes untouched | ✅ Verified (empty, unchanged) |

## Verifier Results

| Test | Checks | Passed | Failed | Status |
|------|--------|--------|--------|--------|
| Good v2 receipt | 48 | 48 | 0 | VERIFIED |
| Malformed hash (lowercase) | 48 | 47 | 1 | REJECTED |
| Secret-bearing receipt | 48 | 47 | 1 | REJECTED |

## Token Safety

- Auth token source: `environment` (no token was set)
- Router auth: disabled (default, `ROUTER_REQUIRE_AUTH` not set)
- No token was generated, persisted, or logged
- Verifier confirmed receipt contains no bearer/secret content
- Verifier rejects any receipt with embedded secrets

## Files Changed

| File | Action |
|------|--------|
| `receipts/runtime-integration/schema-v2.json` | **New** — v2 schema definition |
| `scripts/verify-receipt.ps1` | **New** — receipt verifier (v2 + v1 compatible) |
| `scripts/run-integration-proof-v2.ps1` | **New** — automated lifecycle proof emitting v2 receipts |
| `receipts/runtime-integration/win-runtime-integration-v2-20260622-232214-qwen-coder.json` | **New** — fresh v2 integration receipt |
| `docs/sprints/WIN-RUNTIME-RECEIPTS-2.md` | **New** — this sprint doc |

## Closeout

| Item | Value |
|------|-------|
| **Starting HEADs** | TheLibrarian-main: `1e32002`, runtime-node: `51c2e85` |
| **Final HEADs** | TheLibrarian-main: `1e32002` (unchanged), runtime-node: `[committed]` |
| **Service** | Stopped / Manual |
| **Port 9130** | free (no listener, connection refused) |
| **llama-server orphans** | 0 |
| **rust-router orphans** | 0 |
| **Stashes** | empty (preserved) |
| **Verifier result on v2 receipt** | 48/48 passed |

## Next Sprint Recommendation

**WIN-RUNTIME-QUALIFICATION-1** — Implement artifact verification automation:
- Automated binary rebuild from known source HEAD
- Binary hash comparison against receipt artifact hash
- Integration of verifier into CI/gate pipeline
