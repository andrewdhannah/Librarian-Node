# Current Architecture

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  

---

## 1. Overview

The Windows Runtime Node is an execution authority component within the Librarian platform. It is responsible for model execution, runtime lifecycle, hardware management, evidence generation, and local operational state.

---

## 2. Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         PLATFORM                             │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐    ┌─────────────────────┐        │
│  │    LIBRARIAN CORE    │    │   LIBRARIAN NODE    │        │
│  │                     │    │    (YOU ARE HERE)    │        │
│  │  Canonical Authority │    │                     │        │
│  │                     │    │  Execution Authority │        │
│  │  • Registry         │    │  • Runtime DB        │        │
│  │  • Qualification    │    │  • Residency         │        │
│  │  • Governance       │    │  • Process Mgmt      │        │
│  │  • Owner Decisions  │    │  • Evidence Pipeline │        │
│  │  • Evidence Accept  │    │  • Hardware Mgmt     │        │
│  └──────────┬──────────┘    │  • Model Execution   │        │
│             │               └──────────┬──────────┘        │
│             └───────────┬──────────────┘                   │
│                         │                                   │
│              ┌──────────▼──────────┐                        │
│              │  SHARED CONTRACTS    │                        │
│              │                     │                        │
│              │  • QualificationReq │                        │
│              │  • EvidencePacket   │                        │
│              │  • ResidencyStatus  │                        │
│              │  • NodeIdentity     │                        │
│              │  • TrustState       │                        │
│              └──────────┬──────────┘                        │
│                         │                                   │
│              ┌──────────▼──────────┐                        │
│              │     MCP BRIDGE       │                        │
│              │  Agent ↔ Core       │                        │
│              └─────────────────────┘                        │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Component Responsibilities

### Node

| Responsibility | Implementation | Status |
|----------------|----------------|--------|
| Model execution | llama-server.exe | ✅ |
| Runtime lifecycle | BackendProcess (Stopped/Starting/Healthy/Degraded/Failed) | ✅ |
| 8-state residency | Unloaded/Loading/Ready/Running/Draining/Unloading/VerifyingRelease/Failed | ✅ |
| Evidence recording | EvidenceWriter | ✅ |
| Operational DB | 6 tables: local_models, runtime_profiles, hardware_profiles, job_leases, runtime_runs, lifecycle_evidence | ✅ |
| Health monitoring | GET /health, GET /backend/health | ✅ |
| Process supervision | Child process lifecycle | ✅ |
| Evidence export | Evidence packet → intake boundary → manifest → export bundle | ✅ |
| Custody ledger | PowerShell-based chain-of-custody | ✅ |

### Core

| Responsibility | Implementation | Status |
|----------------|----------------|--------|
| Canonical DB | Model identity, task packs, validator packs | ✅ |
| Packet contracts | QualificationRequest, EvidencePacket, ResidencyStatus | ✅ |
| Bridge client | Mac→Windows HTTP evidence retrieval | ✅ |
| Qualification engine | Runner, stages, validator engine | ✅ |
| Capability evidence | 13 modules, benchmark adapters | ✅ |
| Sprint governance | Authorization, receipts, validation | ✅ |
| Release management | Trust packages, provenance | ✅ |
| Comparative analysis | Analyzer, audit, finding, roster | ✅ |

---

## 4. Crate Architecture

The Rust workspace is organized as three crates:

```
librarian-contracts
     ^        ^
     |        |
librarian-core  librarian-node
```

```
librarian-contracts/  — Shared packet types, validation schemas, cross-boundary DTOs
librarian-core/      — Canonical authority (Core responsibilities)
librarian-node/      — Execution authority (Node responsibilities)
```

**Dependency direction:**
- Core may depend on Contracts
- Node may depend on Contracts
- Core must not depend on Node (compile error if attempted)
- Node must not depend on Core (compile error if attempted)

---

## 5. State Machines

### BackendState (Process Level)

```
Stopped → Starting → Healthy → Degraded → Failed
         ↘ Failed ↗
```

### ResidencyState (Lease Level)

```
Unloaded → Loading → Ready → Running → Draining → Unloading → VerifyingRelease → Unloaded
                                              ↘ Failed ↗
```

### LifecycleState (Platform Level)

```
ABSENT → INSTALL → INITIALIZE → QUALIFY → IDENTITY → READY → DISCOVERED → CANDIDATE → ADMITTED → OPERATIONAL
```

---

## 6. Deployment

| Property | Value |
|----------|-------|
| Machine | DESKTOP-ISNJ51B ("Big Pickle") |
| CPU | i5-3570K |
| GPU | RX 570, 4GB VRAM |
| OS | Windows 10 Pro Workstations |
| Rust toolchain | Latest stable |
| Runtime | llama-server.exe |
| Vulkan | Yes |

---

## 7. References

- ADR-PLATFORM-001 — Core / Node Authority Architecture
- ADR-PLATFORM-002 — Platform Lifecycle
- ARCHITECTURAL-BOUNDARY-MAP — Code organization
- DATA-FLOW.md — Data flow documentation
- DEPENDENCY-MAP.md — Dependency documentation
