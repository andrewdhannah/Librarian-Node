# EPIC-NODE-IDENTITY-AND-CAPABILITY-FOUNDATION

**Status:** Planned  
**Preceded By:** EPIC-CORE-NODE-CRATE-SEPARATION (complete)  
**Repository:** `G:\openwork\librarian-runtime-node\`  
**Design Constraint:** No architecture changes. No Core activation. No MCP implementation. No cloud integration.

---

## 1. Objective

Turn `librarian-node` from an internal runtime into a deployable Node with formal identity and capability advertisement.

The Node currently exists as "Big Pickle" by convention — an unnamed machine running a Rust binary. After this epic, the Node will have:

- A persistent identity that survives restarts and hardware changes
- A capability manifest that describes what it can do
- Node contract types living in the neutral `librarian-contracts` layer
- Endpoints that Core will eventually query for discovery and routing

**Guiding principle:** Prepare the Node to be governed by Core without making the Node depend on Core.

**Platform neutrality:** Windows is the first implementation target. All new types and logic live in platform-agnostic modules. Platform-specific code lives behind an adapter boundary.

---

## 2. Scope

### In Scope

- Node identity generation, persistence, and lifecycle
- Capability manifest schema, detection, and advertisement
- `/node/identity` and `/node/capabilities` endpoints
- Move Node contract types into `librarian-contracts`
- Node state machine (UNREGISTERED → REGISTERED → CONNECTED → AUTHORIZED → EXECUTING → EVIDENCE_PENDING → RECONCILING)
- Tests for all new modules
- Platform adapter pattern for hardware detection

### Not In Scope

- Core activation or runtime
- MCP implementation or bridge activation
- Session protocol (next epic)
- Evidence reconciliation (next epic)
- Node registration handshake with Core (future)
- CLI or installer (future epic)
- SDK (future epic)
- Cloud model integration
- Mac or Linux platform adapters (future)

---

## 3. Deliverables

### 3.1 Node Identity Service

New module: `librarian-node/src/node/`

Responsibilities:

| Function | Description |
|----------|-------------|
| `generate_node_id()` | Create a persistent UUID on first start |
| `load_node_identity()` | Load identity from disk on subsequent starts |
| `get_node_identity()` | Return the current NodeIdentity struct |
| `get_node_status()` | Return a NodeStatus struct (identity + state) |

**NodeIdentity struct (→ `librarian-contracts`):**

```rust
pub struct NodeIdentity {
    pub node_id: String,           // Persistent UUID
    pub display_name: String,      // Human-readable name (default: hostname)
    pub platform: String,          // "windows", "linux", "macos"
    pub runtime_version: String,   // librarian-node crate version
    pub contract_version: String,  // "1" — increments on breaking contract changes
    pub first_seen_at: String,     // ISO 8601
}
```

**Key invariant:** The identity belongs to the Node installation, not the computer hostname. Hardware can change, models can change, the Node identity remains.

### 3.2 Capability Manifest

New submodule: `librarian-node/src/node/capabilities.rs`

Responsibilities:

| Function | Description |
|----------|-------------|
| `detect_capabilities()` | Auto-detect models, hardware, tooling at startup |
| `get_capability_manifest()` | Return current CapabilityManifest |
| `capabilities_endpoint()` | HTTP handler for GET /node/capabilities |

**CapabilityManifest struct (→ `librarian-contracts`):**

```rust
pub struct CapabilityManifest {
    pub node_id: String,
    pub capabilities: Vec<Capability>,
}

pub struct Capability {
    pub capability_type: String,   // "llm.inference", "qualification", "evidence-generation"
    pub runtime: Option<String>,   // "llama.cpp", etc.
    pub models: Option<Vec<ModelDescriptor>>,
    pub available: bool,
}

pub struct ModelDescriptor {
    pub model_id: String,
    pub quantization: Option<String>,
    pub family: Option<String>,
}
```

**Detection logic (platform-adapter pattern):**

```
detect_capabilities()
    |
    ├── query available models from local_models DB table
    ├── query hardware profile from hardware_profiles table
    ├── detect qualification stages available (smoke, primitive_probes)
    ├── detect evidence-generation capability (always true if runtime is running)
    └── assemble CapabilityManifest
```

The detection logic lives in platform-agnostic code. Platform-specific hardware queries (Vulkan device discovery, driver version) live behind a `HardwareDetector` trait in `platform/`.

### 3.3 Node State Machine

New submodule: `librarian-node/src/node/state.rs`

Implement the node lifecycle defined in ADR-NODE-001:

```
UNREGISTERED

     |  node identity generated
     v

REGISTERED

     |  (future: connected to Core)
     v

CONNECTED

     |  (future: authority grant received)
     v

AUTHORIZED

     |  (future: work packet received)
     v

EXECUTING

     |  evidence generated
     v

EVIDENCE_PENDING

     |  (future: core reconciliation)
     v

RECONCILING
```

States that are operational now:
- `UNREGISTERED` — Node first start, no identity yet
- `REGISTERED` — Identity exists, node is operational standalone
- (future states are reserved but the enum and transition validation exist)

```rust
pub enum NodeState {
    Unregistered,
    Registered,
    Connected,        // future
    Authorized,       // future
    Executing,        // future
    EvidencePending,  // future
    Reconciling,      // future
    Failed,
}
```

Transition validation:
```rust
pub fn validate_transition(from: &NodeState, to: &NodeState) -> Result<(), StateTransitionError>;
```

### 3.4 Node API Endpoints

New HTTP endpoints on the existing `librarian-node` server:

| Endpoint | Method | Returns | Description |
|----------|--------|---------|-------------|
| `/node/identity` | GET | `NodeIdentity` | Persistent node identity |
| `/node/status` | GET | `NodeStatus` | Identity + state + uptime |
| `/node/capabilities` | GET | `CapabilityManifest` | Current capability advertisement |

**NodeStatus struct (→ `librarian-contracts`):**

```rust
pub struct NodeStatus {
    pub identity: NodeIdentity,
    pub state: String,           // current NodeState as string
    pub uptime_seconds: u64,
    pub last_state_change: String,
}
```

### 3.5 Contract Types → `librarian-contracts`

Move the following types into `librarian-contracts/src/node/`:

```
librarian-contracts/src/
    ├── lib.rs              (add pub mod node;)
    ├── packets/            (existing)
    ├── bridge/             (existing)
    └── node/               (NEW)
        ├── mod.rs
        ├── identity.rs     → NodeIdentity, NodeStatus
        ├── capabilities.rs → CapabilityManifest, Capability, ModelDescriptor
        └── state.rs        → NodeState (enum only, no runtime logic)
```

**The contracts crate must remain neutral.** It contains only:
- Struct definitions (serde Serialize/Deserialize)
- Validation methods (validate() functions)
- Hash computation (compute_hash())

It does NOT contain:
- Database queries
- Hardware detection
- Runtime logic
- Platform-specific code

### 3.6 Platform Adapter Pattern

Create `librarian-node/src/platform/` with:

```rust
/// Trait for platform-specific hardware detection.
pub trait HardwareDetector {
    fn detect_gpu(&self) -> Vec<GpuInfo>;
    fn total_ram_mb(&self) -> u64;
    fn platform_name(&self) -> String;
}

// Windows implementation
#[cfg(target_os = "windows")]
pub mod windows;
```

Initially only the Windows adapter is implemented. The trait definition in platform-agnostic code ensures Mac/Linux adapters can be added later without changing the node core.

---

## 4. Acceptance Gates

| Gate | Criteria | Verification |
|------|----------|-------------|
| G-ID-1 | Node identity persists across restarts | Start node → generate identity → restart → identity matches |
| G-ID-2 | `GET /node/identity` returns valid `NodeIdentity` | HTTP 200, valid JSON matching schema |
| G-ID-3 | `GET /node/status` returns identity + state + uptime | HTTP 200, state is "unregistered" or "registered" |
| G-CAP-1 | `GET /node/capabilities` returns `CapabilityManifest` | HTTP 200, includes at least "evidence-generation" capability |
| G-CAP-2 | Capability detection finds available models | Models from `local_models` table appear in manifest |
| G-CAP-3 | Capability detection finds hardware profile | Hardware profile from `hardware_profiles` appears |
| G-CONTRACT-1 | `librarian-contracts` builds with node module | `cargo build -p librarian-contracts --release` |
| G-CONTRACT-2 | Node contract types are neutral | No DB, runtime, or platform dependencies |
| G-STATE-1 | Node state machine validates transitions | Valid transitions pass, invalid transitions fail |
| G-STATE-2 | State survives restart | Node starts → state "registered" after identity check |
| G-PLATFORM-1 | HardwareDetector trait compiles on all platforms | `cargo check` on Windows, Linux, macOS |
| G-PLATFORM-2 | Windows adapter detects GPU or returns empty | No crash, graceful empty response on systems without GPU |
| G-TEST | All new modules have test coverage | `cargo test -p librarian-node` includes identity, capabilities, state tests |

---

## 5. File Map

### New Files

```
librarian-node/src/
    node/
        mod.rs              → module declarations, re-exports
        identity.rs         → NodeIdentityService
        capabilities.rs     → Capability detection and manifest
        state.rs            → NodeState enum and transition validation
    platform/
        mod.rs              → HardwareDetector trait
        windows.rs          → Windows HardwareDetector implementation
```

### Modified Files

```
librarian-node/src/
    server.rs               → add /node/identity, /node/status, /node/capabilities routes
    main.rs                 → initialize NodeIdentityService on startup

librarian-contracts/src/
    lib.rs                  → add `pub mod node;`
    node/
        mod.rs              → re-exports
        identity.rs         → NodeIdentity, NodeStatus structs
        capabilities.rs     → CapabilityManifest, Capability, ModelDescriptor structs
        state.rs            → NodeState enum
```

### Test Files (new)

```
librarian-node/tests/
    test_node_identity.rs   → identity persistence, endpoint response
    test_node_capabilities.rs → manifest detection, endpoint response
    test_node_state.rs      → state machine transitions
```

---

## 6. Dependencies

No new external crate dependencies. All new types use:
- `serde` / `serde_json` (serialization)
- `uuid` (node ID generation)
- `chrono` (timestamps)
- `std::time` (uptime tracking)

---

## 7. Invariants to Preserve

| Invariant | Enforcement |
|-----------|-------------|
| Node does not depend on Core | `librarian-node/Cargo.toml` does not list `librarian-core` |
| Contracts remain neutral | `librarian-contracts/Cargo.toml` has no DB, runtime, or platform deps |
| No Core activation | No Core endpoints, services, or state modifications in this epic |
| No MCP implementation | Deferred to session protocol epic |
| Behavioral preservation | Existing 85+ node tests continue to pass |
| Platform neutrality | Hardware detection behind trait; platform adapters are swappable |

---

## 8. Completion Criteria

The epic is complete when:

- [ ] Node identity is generated on first start and persists across restarts
- [ ] `GET /node/identity` returns valid identity
- [ ] `GET /node/status` returns identity + state + uptime
- [ ] `GET /node/capabilities` returns capability manifest with detected models
- [ ] Node contract types live in `librarian-contracts` and build independently
- [ ] Node state machine validates transitions correctly
- [ ] Platform adapter trait compiles; Windows adapter detects GPU or returns empty gracefully
- [ ] All existing node tests still pass
- [ ] `cargo build --release` succeeds across workspace
- [ ] `cargo test --workspace` passes with 0 failures
