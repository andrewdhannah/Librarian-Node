# ADR-PLATFORM-001 — Core / Node Authority Architecture

**Status:** Accepted  
**Date:** 2026-07-19  
**Supersedes:** ADR-NODE-001-DISTRIBUTED-LIBRARIAN-AUTHORITY-MODEL.md  
**Preceded By:** AIR-Q ADR (established qualification boundaries)  
**Discovery Artifacts:**
- `LIBRARIANOS-CORE-NODE-ARCHITECTURE-MAPPING.md`  
- `WINDOWS-NODE-ARCHITECTURE-DISCOVERY.md`  
- `CORE-NODE-DEPENDENCY-BOUNDARY-AUDIT.md`  
- `CORE-NODE-DEPENDENCY-BOUNDARY-AUDIT-REPORT.md`  
**Workspace:** G:\Models  
**Evidence Supporting Acceptance:**
- Crate separation completed: `librarian-contracts` (56 tests), `librarian-core` (580+ tests), `librarian-node` (85+ tests)
- Zero forbidden imports (Core→Node: 0, Node→Core: 0)
- Zero circular dependencies
- Zero cross-domain test contamination
- Node identity, trust/custody, operational maturity, registry governance, MCP contracts, owner authority all implemented

---

## 1. Context

LibrarianOS has evolved from a single-machine workflow into a distributed authority platform. This ADR establishes the authoritative architectural model for the entire platform.

### 1.1 Architectural Evolution

| Phase | State | Date |
|-------|-------|------|
| Discovery | Initial assumption: Core is conceptual, Node is implemented | 2026-07-15 |
| Audit | Found: Core exists in `canonical/` (19 submodules), co-located with Node | 2026-07-15 |
| Crate Separation | Model B implemented: `librarian-contracts`, `librarian-core`, `librarian-node` | 2026-07-15 |
| Platform Foundation | Node identity, trust/custody, registry governance, MCP contracts completed | 2026-07-19 |
| **This ADR** | **Formalize the full platform architecture** | **2026-07-19** |

### 1.2 What Changed Since ADR-NODE-001

ADR-NODE-001 focused on the Core/Node boundary. Since then:

**Completed:**
- Node identity and registration
- Trust and custody framework
- Operational maturity (evidence pipeline, lifecycle management)
- Registry governance
- MCP contract definitions
- Owner authority workflows
- Crate separation (Model B implemented)

**New architectural concerns:**
- Installability as a first-class layer
- Platform portability across machines
- Core authority activation
- Multi-node coordination (future)

---

## 2. Platform Architecture

### 2.1 Architectural Layers

```
┌─────────────────────────────────────────────────────────────┐
│                         PLATFORM                             │
│  Installability · Portability · Configuration · Discovery    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────┐     ┌─────────────────────┐       │
│  │    LIBRARIAN CORE    │     │   LIBRARIAN NODE     │       │
│  │                     │     │                     │       │
│  │  Canonical Authority │     │  Execution Authority │       │
│  │                     │     │                     │       │
│  │  • Registry         │     │  • Runtime DB        │       │
│  │  • Qualification     │     │  • Residency         │       │
│  │  • Governance        │     │  • Process Mgmt      │       │
│  │  • Owner Decisions   │     │  • Evidence Pipeline │       │
│  │  • Evidence Accept   │     │  • Hardware Mgmt     │       │
│  │  • Capability Policy │     │  • Model Execution   │       │
│  │  • Provenance        │     │  • Local Recovery    │       │
│  │  • Release Authority │     │                     │       │
│  └──────────┬──────────┘     └──────────┬──────────┘       │
│             │                           │                   │
│             └───────────┬───────────────┘                   │
│                         │                                   │
│              ┌──────────▼──────────┐                        │
│              │  SHARED CONTRACTS    │                        │
│              │                     │                        │
│              │  • QualificationReq │                        │
│              │  • EvidencePacket   │                        │
│              │  • ResidencyStatus  │                        │
│              │  • NodeIdentity     │                        │
│              │  • TrustState       │                        │
│              │  • Receipt          │                        │
│              └──────────┬──────────┘                        │
│                         │                                   │
│              ┌──────────▼──────────┐                        │
│              │     MCP BRIDGE       │                        │
│              │                     │                        │
│              │  Agent ↔ Core       │                        │
│              │  Transport Layer    │                        │
│              │                     │                        │
│              │  • Proposal Submit  │                        │
│              │  • Evidence Submit  │                        │
│              │  • Receipt Submit   │                        │
│              └─────────────────────┘                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Installability Layer

Installability is a first-class architectural concern, not an implementation detail.

**Responsibilities:**
- Cross-platform package distribution (Windows, macOS, Linux)
- Dependency management (Rust crates, system libraries, runtime binaries)
- Configuration migration across versions
- Uninstallation and cleanup
- Version compatibility verification

**Constraints:**
- Must not require manual file placement
- Must support both development and production installations
- Must handle platform-specific dependencies (Vulkan, CUDA, Metal)
- Must preserve user data across upgrades

### 2.3 Core / Node / MCP Responsibilities

| Layer | Responsibilities | Authority |
|-------|-----------------|-----------|
| **Core** | Canonical state, qualification, governance, owner decisions, evidence acceptance, capability policy, provenance, release | Canonical authority |
| **Node** | Model execution, runtime lifecycle, hardware management, evidence generation, local operational state, residency management, local recovery | Execution authority |
| **MCP Bridge** | Agent ↔ Core transport, tool exposure, proposal/evidence/receipt pattern | Transport layer |
| **Shared Contracts** | Packet types, validation schemas, cross-boundary DTOs | Truth boundary |

### 2.4 Authority Boundaries

```
Core Commands → Node Executes → Node Reports → Core Validates
     │              │              │              │
     │              │              │              │
     └──────────────┴──────────────┴──────────────┘
                    │
            Packet Contracts
         (QualificationRequest,
          EvidencePacket,
          ResidencyStatus)
```

**Authority rules:**
- Core may create work packets, accept evidence, make qualification decisions, seal sprint state, issue authority grants
- Core may not execute models directly, control hardware, manage processes, modify Node runtime state
- Node may execute approved work packets, generate and export evidence, report state to Core, maintain local operational state, recover locally from failures
- Node may not create canonical truth, approve capability classification, modify governance rules, seal evidence as canonical, override Core decisions, claim canonical authority

### 2.5 Deployment Boundaries

| Boundary | Enforcement | Scope |
|----------|-------------|-------|
| **Crate graph** | Compile-time (Rust workspace) | Core ↔ Node separation |
| **Packet contracts** | Type-level (sealed types) | Core ↔ Node communication |
| **MCP protocol** | Runtime (stdio/HTTP) | Agent ↔ Core interaction |
| **Network** | Future (when multi-node) | Core ↔ Node (Model C) |

---

## 3. Decisions

### Decision 1: Platform Architecture is Core / Node / MCP / Contracts

LibrarianOS is a platform with four architectural layers:
1. **Platform** — Installability, portability, configuration, discovery
2. **Core** — Canonical authority (registry, qualification, governance, owner decisions)
3. **Node** — Execution authority (runtime, residency, evidence, hardware)
4. **MCP Bridge** — Agent ↔ Core transport (proposal/evidence/receipt pattern)
5. **Shared Contracts** — Truth boundary (packet types, validation schemas)

### Decision 2: Model B is the Target Architecture

| Model | Status | Notes |
|-------|--------|-------|
| A – Logical Separation | **Legacy / temporary** | Was the starting point; no longer acceptable |
| B – Workspace Separation | **Target architecture** | Implemented and enforced |
| C – Distributed Services | **Future evolution** | When multi-node operation is required |

Model B provides compile-time authority protection without operational overhead. The crate boundary is the invariant.

### Decision 3: Installability is a First-Class Architectural Concern

Installability is not an implementation detail. It must be:
- Designed before implementation
- Tested across platforms
- Versioned with the platform
- Documented as an architectural decision

### Decision 4: Packet Contracts are the Authority Boundary

The sealed packet types are the authority boundary between Core and Node:

**Core → Node: QualificationRequest**
- Model identity, execution configuration, execution constraints
- Does not contain: capability authority, approval state, canonical truth mutation

**Node → Core: EvidencePacket**
- Model identity binding, execution identity, lease lifecycle, execution metrics, lifecycle events, release verification
- Does not contain: approval, capability claims, routing authority

**Node → Core: ResidencyStatus (query/response)**
- Active leases, active runs, draining state, VRAM status
- Does not contain: capability status, role assignments, qualification decisions

All packets enforce `assert_no_capability_data()` at the type level.

### Decision 5: MCP is Agent→Core Transport, Not Core→Node Transport

```
Agent/Human
     |
     v
    MCP                          ← transport layer (Agent ↔ Core)
     |
     v
Librarian Core                   ← authority layer
     |
     | QualificationRequest / EvidencePacket / ResidencyStatus
     v
Librarian Node                   ← execution layer (HTTP/REST bridge)
```

MCP tools follow proposal-and-apply model:
- `project_proposal_submit` — Propose a change
- `project_evidence_submit` — Return evidence
- `project_receipt_submit` — Return action receipts

Generic file-write MCP tools must not be exposed on canonical paths.

### Decision 6: Nodes Require Formal Identity

A Node must have a persistent identity established at registration and verified on each connection:

```rust
NodeIdentity {
    node_id: String,           // e.g., "win-bigpickle-rx570-001"
    hostname: String,          // e.g., "DESKTOP-ISNJ51B"
    hardware_profile: Ref,     // references hardware_profiles.hw_profile_id
    runtime_version: String,   // e.g., "librarian-node-v1"
    supported_models: Vec,     // list of local_models.model_id
    capabilities: Vec,         // e.g., ["llama.cpp", "gguf", "local-inference"]
    trust_state: Enum,         // registered | verified | quarantined | revoked
    registered_at: Timestamp,
    last_seen_at: Timestamp,
}
```

Node identity is not capability authority. A node reporting "I can run MiniCPM5" does not mean "MiniCPM5 is approved." Only Core, through qualification + Owner decision, can establish capability authority.

### Decision 7: Node Lifecycle States

```
UNREGISTERED
     |
     | registration handshake
     v
REGISTERED
     |
     | connection established
     v
CONNECTED
     |
     | authority grant received
     v
AUTHORIZED
     |
     | work packet received
     v
EXECUTING
     |
     | evidence exported
     v
EVIDENCE_PENDING
     |
     | core reconciliation
     v
RECONCILING
     |
     | accepted / rejected
     v
CONNECTED            (normal cycle repeats)

Failure transitions:
     any state → FAILED → (manual or automatic recovery) → CONNECTED
     any state → QUARANTINED (security violation detected)
     any state → REVOKED (authority withdrawn)
```

### Decision 8: Offline Operation is Intentional

The Node is intentionally autonomous when disconnected from Core.

**Offline Allowed:**
- Load model from local inventory
- Execute tasks from cached work packets
- Collect execution evidence
- Record lifecycle events
- Recover locally from failures

**Offline Not Allowed:**
- Create or modify qualification state
- Change authority grants
- Accept new governance policy
- Seal decisions as canonical
- Claim Core authority

Offline evidence enters a **Pending Evidence** queue. On reconnection:
```
Pending Evidence → Core Validation → Accepted / Rejected
```

### Decision 9: Architectural Boundary Map Before Workspace Separation

Before physical crate extraction, the following governance steps are required:

```
Current Monolith
     ↓
Architectural Boundary Map    ← identify what moves where
     ↓
Workspace Plan                ← define crate structure
     ↓
Migration Plan                ← define extraction order
     ↓
Compile-time Separation       ← execute extraction
```

This reduces the risk of accidentally breaking contracts while extracting components.

---

## 4. Consequences

### Positive

1. **Platform-level thinking** — Installability, portability, and discovery are architectural concerns, not afterthoughts
2. **Compiler-enforced authority separation** — Dependency violations are build failures
3. **Clear deployment model** — Core and Node may be separate processes; the crate boundary is the invariant
4. **Contracts crate is authoritative** — Single source of truth for cross-boundary types
5. **MCP is properly scoped** — Transport for agent interaction, not authority
6. **Node identity enables discovery** — Nodes can be registered, discovered, and tracked
7. **Offline model is explicit** — Designed with pending evidence and reconciliation
8. **Governance before extraction** — Boundary map → workspace plan → migration plan → separation

### Negative

1. **Platform layer adds scope** — Installability requires cross-platform testing
2. **Crate split requires effort** — File movement, dependency updates, test updates
3. **Shared migration coordination** — Canonical DB and operational DB migrations require explicit coordination
4. **Dual-binary testing complexity** — Integration tests spanning Core and Node require both crates

### Risks

1. **Model C deferred** — Multi-node operation requires network separation, which is deliberately deferred
2. **Migration drift** — Without shared migration runner, DBs could drift. Mitigation: shared migration verification in CI
3. **Platform scope creep** — Installability must remain focused on distribution, not feature development

---

## 5. Compliance

| Entity | Must | Must Not |
|--------|------|----------|
| Platform layer | Handle installability, portability, configuration | Contain business logic |
| Core crate | Depend only on contracts + external crates | Depend on node crate |
| Node crate | Depend only on contracts + external crates | Depend on core crate |
| Contracts crate | Contain only schemas, DTOs, validation | Contain DB, runtime, or UI logic |
| MCP bridge | Communicate via stdio/HTTP + MCP protocol | Import Node internals |
| Bridge client | Communicate via HTTP + packet contracts | Import Node internals |
| Evidence export | Produce EvidencePacket via bridge | Assert canonical authority on packet |
| MCP tools | Implement proposal/evidence/receipt pattern | Expose generic file_write on canonical paths |
| Node operator | Report state accurately | Claim canonical authority |

---

## 6. Decision Summary

| Question | Decision |
|----------|----------|
| What is the platform architecture? | **Core / Node / MCP / Shared Contracts** with Installability layer |
| Is Core/Node a new architecture? | **No.** Existing architecture discovered and formalized. |
| Should Model B be the target? | **Yes.** Implemented and enforced. |
| Should Installability be first-class? | **Yes.** Designed before implementation. |
| Should the compiler enforce separation? | **Yes.** Forbidden dependencies = build failure. |
| Should MCP be the Core/Node boundary? | **No.** MCP is Agent↔Core transport only. |
| Should packet contracts be the authority boundary? | **Yes.** QualificationRequest + EvidencePacket + ResidencyStatus. |
| Should Nodes have formal identity? | **Yes.** Node registry with registration handshake. |
| Should Nodes operate offline? | **Yes.** Pending evidence queue + reconciliation. |
| Can Nodes make canonical decisions? | **No.** Core-only. Enforced at crate boundary. |
| What comes before workspace separation? | **Boundary map → workspace plan → migration plan.** |

---

## 7. References

1. ADR-NODE-001-DISTRIBUTED-LIBRARIAN-AUTHORITY-MODEL.md — Original Core/Node ADR
2. AIR-Q ADR — Established qualification boundaries
3. `LIBRARIANOS-CORE-NODE-ARCHITECTURE-MAPPING.md` — Discovery mapping
4. `WINDOWS-NODE-ARCHITECTURE-DISCOVERY.md` — Windows implementation inventory
5. `CORE-NODE-DEPENDENCY-BOUNDARY-AUDIT.md` — Dependency extraction analysis
6. `custody-ledger-spec.md` — Node role contract, authority separation
7. `MODEL-QUALIFICATION-ROUTER-AUTHORITY-MAP.md` — Ownership matrix
8. `WIN-MULTINODE-MCP-DOCUMENT-CUSTODY-NOTES.md` — MCP custody design

---

## 8. Revision History

| Date | Change | Author |
|------|--------|--------|
| 2026-07-15 | ADR-NODE-001 created | Architecture review |
| 2026-07-19 | Superseded by ADR-PLATFORM-001 | Platform evolution |
