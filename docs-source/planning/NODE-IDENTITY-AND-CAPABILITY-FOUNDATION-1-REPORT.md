# Sprint Report: NODE-IDENTITY-AND-CAPABILITY-FOUNDATION-1

**Date:** 2026-07-15  
**Repository:** `G:\openwork\librarian-runtime-node\`

---

## 1. Files Created

| Path | Description |
|------|-------------|
| `librarian-contracts/src/node/mod.rs` | Module declarations and re-exports for node contract types |
| `librarian-contracts/src/node/identity.rs` | `NodeIdentity`, `NodeStatus` structs (Serialize/Deserialize) |
| `librarian-contracts/src/node/capabilities.rs` | `CapabilityManifest`, `Capability`, `ModelDescriptor` structs |
| `librarian-contracts/src/node/hardware.rs` | `HardwareProfile` struct for capability advertisement |
| `librarian-contracts/src/node/state.rs` | `NodeState` enum with `as_str()` |
| `librarian-node/src/node/mod.rs` | Module declarations and re-exports |
| `librarian-node/src/node/identity_service.rs` | `NodeIdentityService` — load/create/persist identity |
| `librarian-node/src/node/capabilities.rs` | `detect_capabilities()` — build capability manifest from DB |
| `librarian-node/src/node/hardware.rs` | `detect_hardware()` — detect CPU/GPU/RAM via platform adapter |
| `librarian-node/src/node/state.rs` | `NodeStateMachine` — state transitions with validation |
| `librarian-node/src/platform/mod.rs` | `HardwareDetector` trait + `create_detector()` factory |
| `librarian-node/src/platform/windows.rs` | `WindowsHardwareDetector` using WMI queries |

## 2. Files Modified

| Path | Change |
|------|--------|
| `librarian-contracts/src/lib.rs` | Added `pub mod node;` |
| `librarian-node/src/lib.rs` | Added `pub mod node;` and `pub mod platform;` |
| `librarian-node/src/server.rs` | Added `node_identity_service` and `node_state` to `AppState`; added `GET /node/identity`, `GET /node/status`, `GET /node/capabilities` handlers and routes |
| `librarian-node/src/main.rs` | Initializes `NodeIdentityService` and `NodeStateMachine` on startup, transitions to `Registered` |
| `librarian-node/tests/integration_test.rs` | Added `node_identity_service` and `node_state` fields to test `setup_app()`; added 4 new endpoint tests |

## 3. Architecture Impact

- **New `node/` module** in `librarian-node` provides identity lifecycle, capability detection, hardware detection, and state machine.
- **New `platform/` module** provides the `HardwareDetector` trait for platform-agnostic hardware detection. Windows implementation uses WMI.
- **New `librarian-contracts/src/node/` module** provides neutral contract types (`NodeIdentity`, `CapabilityManifest`, `HardwareProfile`, `NodeState`) that both Core and Node can use without runtime dependencies.
- **AppState** gains `node_identity_service` (persistent identity) and `node_state` (lifecycle state machine).
- **Existing modules untouched:** `db/`, `evidence/`, `config.rs`, `models/`, `process.rs`, `refusal.rs`, `residency/`, `runtime_state/`, `operator/`.

## 4. Dependency Impact

**No new external crate dependencies.** All new types use:
- `serde` / `serde_json` — already in workspace
- `uuid` — already in workspace
- `chrono` — already in workspace
- `std::process::Command` for WMI queries (Windows)
- `std::env::consts::OS` for platform detection

## 5. API Additions

| Endpoint | Method | Returns | Description |
|----------|--------|---------|-------------|
| `/node/identity` | GET | `NodeIdentity` | Persistent node identity (UUID, hostname, platform, version) |
| `/node/status` | GET | `NodeStatus` | Identity + state (unregistered/registered) + uptime + last state change |
| `/node/capabilities` | GET | `CapabilityManifest` | Detected capabilities (inference, hardware, runtime, qualification, evidence-generation, concurrency) |

## 6. Test Results

```
All workspace tests: 0 failures

librarian-node unit tests:         86 passed
librarian-node integration tests:  18 passed  (14 existing + 4 new)
librarian-contracts tests:         56 passed
librarian-core tests:             580 passed
All other test suites:            254+ passed
```

New test coverage:
- **Identity:** creation generates UUID, persistence round-trip, corrupted file regeneration, display name defaults to hostname
- **Capabilities:** detection with empty DB, detection with model data, serialization round-trip
- **Hardware:** detection returns expected fields (best-effort)
- **State:** initial state, valid transitions, invalid transitions rejected, `set_state` bypasses validation, last_change timestamp updates
- **API:** `/node/identity` returns valid JSON with all fields, `/node/status` returns identity+state+uptime, `/node/capabilities` returns all expected capability types, auth middleware applies to node endpoints

## 7. Boundary Verification

| Invariant | Status |
|-----------|--------|
| **Core boundary unchanged** | `librarian-node/Cargo.toml` does not list `librarian-core` as a dependency ✅ |
| **Node does not gain authority** | All new endpoints are read-only; no new POST/PUT/DELETE endpoints added ✅ |
| **No MCP implementation** | No MCP types, bridge activation, or session protocol introduced ✅ |
| **No bootstrap/installer work** | No CLI changes, no installer logic ✅ |
| **No session protocol** | No session state, routing, or packet exchange added ✅ |
| **Contracts remain neutral** | `librarian-contracts/Cargo.toml` unchanged; no DB, runtime, or platform deps added ✅ |
| **Behavioral preservation** | All 14 existing integration tests still pass ✅ |

## 8. Gates Table

| Gate | Criteria | Status |
|------|----------|--------|
| NODE-ID-1 | Identity exists and persists | ✅ `test_identity_persistence` confirms create → reload → match |
| NODE-ID-2 | Node reports identity through API | ✅ `test_node_identity_endpoint_returns_valid_json` passes |
| NODE-CAP-1 | Node produces capability manifest | ✅ `test_node_capabilities_endpoint_returns_manifest` passes |
| NODE-CAP-2 | Hardware profile generation works | ✅ `test_detect_hardware_returns_expected_fields` passes; manifest includes hardware capability |
| NODE-CONTRACT-1 | Shared contracts correctly placed | ✅ `cargo build -p librarian-contracts --release` succeeds; all types have Serialize/Deserialize |
| NODE-TEST-1 | All existing and new tests pass | ✅ `cargo test --workspace` — 0 failures |
