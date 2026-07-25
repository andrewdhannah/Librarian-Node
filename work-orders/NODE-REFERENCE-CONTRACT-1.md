# NODE-REFERENCE-CONTRACT-1 — Cross-Platform Node Specification

**Status:** Approved
**Epic:** EPIC-PLATFORM-IMPLEMENTATION-EQUIVALENCE-1
**Phase:** 9 — Reference Architecture

---

## Objective

Define the canonical node specification that all platform implementations must satisfy, then create three platform implementations under the same governed contract.

**This sprint does NOT:**
- Create a new repository
- Port code between platforms
- Build application-level features
- Add new execution modes

**This sprint DOES:**
- Extract startup harness into shared contract
- Define mandatory node capabilities
- Define platform adapter boundary
- Create Windows node implementation
- Create Linux node implementation
- Create macOS node implementation
- Run three-way equivalence validation

---

## Architectural Principle

```
Librarian-Node Canonical Guidance
                 |
    +------------+------------+
    |            |            |
    ▼            ▼            ▼
macOS Node   Windows Node   Linux Node
    |            |            |
    +------------+------------+
                 |
                 ▼
         Same Contract
         Same Startup Harness
         Same Evidence Model
         Same Governance Model
         Same Capability Model
```

The goal is NOT:

```
macOS Librarian Node
        |
        copy ideas
        |
Windows/Linux nodes
```

The goal IS:

```
              Librarian-Node Canonical Guidance
                         |
        +----------------+----------------+
        |                |                |
        ▼                ▼                ▼
     macOS Node      Windows Node     Linux Node
```

---

## Repository Structure

```
Librarian-Node
│
├── contracts/
│   ├── startup/
│   │   ├── STARTUP-PROTOCOL.md
│   │   ├── STARTUP-OUTPUT-CONTRACT.md
│   │   └── SESSION-IDENTITY-CONTRACT.md
│   │
│   ├── governance/
│   ├── custody/
│   ├── receipts/
│   ├── capabilities/
│   └── extensions/
│
├── core/
│   ├── lifecycle/
│   ├── validation/
│   ├── qualification/
│   └── evidence/
│
├── runtime/
│   ├── execution-engine/
│   ├── work-packet-handling/
│   └── mcp-boundary/
│
├── adapters/
│   ├── windows/
│   │   ├── startup-windows.ps1
│   │   ├── node-identity.json
│   │   └── platform-adapter.md
│   │
│   ├── linux/
│   │   ├── startup-linux.sh
│   │   ├── node-identity.json
│   │   └── platform-adapter.md
│   │
│   └── macos/
│       ├── startup-macos.swift
│       ├── node-identity.json
│       └── platform-adapter.md
│
├── schemas/
│   ├── startup-receipt.schema.json
│   ├── execution-receipt.schema.json
│   └── governance-receipt.schema.json
│
├── scripts/
│   ├── qualification/
│   ├── evidence/
│   └── equivalence/
│
└── docs/
    ├── NODE-REFERENCE-ARCHITECTURE.md
    ├── PLATFORM-ADAPTER-BOUNDARY.md
    └── THREE-WAY-EQUIVALENCE-PROTOCOL.md
```

---

## Node Reference Specification

### Node Identity

Every node must provide:

```json
{
  "node_type": "librarian-runtime-node",
  "node_id": "<unique-node-identifier>",
  "authority": "owner-controlled",
  "platform": "<windows|linux|macos>",
  "governance_commit": "<canonical-governance-commit-sha>",
  "state": "UNREGISTERED|GOVERNED_NODE|GOVERNED_EXECUTION",
  "capabilities": ["<list-of-available-capabilities>"],
  "created_at": "<iso-8601-timestamp>"
}
```

### Startup Sequence

Every node must execute:

```
1. Load identity
2. Verify governance
3. Load capabilities
4. Validate environment
5. Produce startup receipt
6. Enter governed mode
```

### Required Services

Every node must provide:

- Governance verification
- Work packet intake
- Capability enforcement
- Custody tracking
- Evidence generation
- Receipt validation

### Platform Adapter Boundary

The shared layers define **what** must happen.

The adapters define **how** that operating system does it.

| Shared Contract | Windows Adapter | Linux Adapter | macOS Adapter |
|-----------------|-----------------|---------------|---------------|
| Startup sequence | PowerShell bootstrap | systemd startup | launchd startup |
| Identity loading | Windows registry/files | /etc/librarian/ | ~/Library/Librarian/ |
| Governance verification | Windows crypto APIs | OpenSSL | Security framework |
| Capability enforcement | Windows ACLs | Linux permissions | macOS entitlements |
| Evidence generation | Windows Event Log | syslog/journald | Unified logging |
| Receipt validation | Windows certificate store | OpenSSL verification | CryptoKit |

---

## Acceptance Gates

### SH-1 — Startup Contract Exists in Shared Repository

Verify:
- `contracts/startup/` directory exists
- `STARTUP-PROTOCOL.md` exists
- `STARTUP-OUTPUT-CONTRACT.md` exists
- `SESSION-IDENTITY-CONTRACT.md` exists
- All contracts are platform-neutral

### SH-2 — Startup Receipt Schema is Platform-Neutral

Verify:
- `schemas/startup-receipt.schema.json` exists
- Schema does not contain platform-specific fields
- Schema defines required fields for all platforms
- Schema is valid JSON Schema

### SH-3 — macOS Startup Behavior Maps to Contract

Verify:
- `adapters/macos/startup-macos.swift` exists
- macOS startup follows STARTUP-PROTOCOL.md
- macOS startup produces valid startup receipt
- macOS startup satisfies all required checks

### SH-4 — Windows Startup Behavior Maps to Contract

Verify:
- `adapters/windows/startup-windows.ps1` exists
- Windows startup follows STARTUP-PROTOCOL.md
- Windows startup produces valid startup receipt
- Windows startup satisfies all required checks

### SH-5 — Linux Startup Behavior Maps to Contract

Verify:
- `adapters/linux/startup-linux.sh` exists
- Linux startup follows STARTUP-PROTOCOL.md
- Linux startup produces valid startup receipt
- Linux startup satisfies all required checks

### SH-6 — Startup Equivalence Validation Passes

Verify:
- All three platforms produce equivalent startup receipts
- Governance decisions are identical across platforms
- Capability declarations are identical across platforms
- Evidence structure is identical across platforms

---

## Required Deliverables

### 1. Shared Contracts

Create `contracts/startup/`:

| File | Purpose |
|------|---------|
| STARTUP-PROTOCOL.md | Defines startup sequence and requirements |
| STARTUP-OUTPUT-CONTRACT.md | Defines startup receipt format |
| SESSION-IDENTITY-CONTRACT.md | Defines node identity requirements |

### 2. Receipt Schemas

Create `schemas/`:

| File | Purpose |
|------|---------|
| startup-receipt.schema.json | Platform-neutral startup receipt schema |
| execution-receipt.schema.json | Platform-neutral execution receipt schema |
| governance-receipt.schema.json | Platform-neutral governance receipt schema |

### 3. Platform Adapters

Create `adapters/`:

| Directory | Files |
|-----------|-------|
| windows/ | startup-windows.ps1, node-identity.json, platform-adapter.md |
| linux/ | startup-linux.sh, node-identity.json, platform-adapter.md |
| macos/ | startup-macos.swift, node-identity.json, platform-adapter.md |

### 4. Documentation

Create `docs/`:

| File | Purpose |
|------|---------|
| NODE-REFERENCE-ARCHITECTURE.md | Defines node reference architecture |
| PLATFORM-ADAPTER-BOUNDARY.md | Defines adapter boundary |
| THREE-WAY-EQUIVALENCE-PROTOCOL.md | Defines three-way equivalence validation |

### 5. Equivalence Scripts

Create `scripts/equivalence/`:

| File | Purpose |
|------|---------|
| three-way-equivalence.ps1 | Validate three-way equivalence |
| platform-adapter-validation.ps1 | Validate platform adapter compliance |

---

## Evidence Format

### Startup Receipt (Platform-Neutral)

```json
{
  "receipt_id": "<unique-receipt-id>",
  "node_id": "<node-identifier>",
  "platform": "<windows|linux|macos>",
  "governance_commit": "<commit-sha>",
  "startup_phase": "complete",
  "identity_loaded": true,
  "governance_verified": true,
  "capabilities_loaded": true,
  "environment_validated": true,
  "checks_passed": 6,
  "checks_failed": 0,
  "status": "GOVERNED_EXECUTION",
  "timestamp": "<iso-8601-timestamp>"
}
```

### Three-Way Equivalence Receipt

```json
{
  "equivalence_id": "<unique-equivalence-id>",
  "platforms": ["windows", "linux", "macos"],
  "governance_equivalent": true,
  "startup_equivalent": true,
  "capability_equivalent": true,
  "evidence_equivalent": true,
  "divergences_detected": 0,
  "overall_result": "THREE_WAY_EQUIVALENT",
  "timestamp": "<iso-8601-timestamp>"
}
```

---

## Dependencies

- CROSS-PLATFORM-EQUIVALENCE-1 complete
- Windows node operational
- Linux node operational
- macOS node operational (or simulated)
- Identical governance deployed to all nodes
- Equivalence verification framework operational

---

## Estimated Effort

1 sprint (~1 week)

---

## Key Files

| Path | Description |
|------|-------------|
| contracts/startup/STARTUP-PROTOCOL.md | Startup protocol |
| contracts/startup/STARTUP-OUTPUT-CONTRACT.md | Startup output contract |
| contracts/startup/SESSION-IDENTITY-CONTRACT.md | Session identity contract |
| schemas/startup-receipt.schema.json | Startup receipt schema |
| adapters/windows/startup-windows.ps1 | Windows startup adapter |
| adapters/linux/startup-linux.sh | Linux startup adapter |
| adapters/macos/startup-macos.swift | macOS startup adapter |
| docs/NODE-REFERENCE-ARCHITECTURE.md | Node reference architecture |
| docs/PLATFORM-ADAPTER-BOUNDARY.md | Platform adapter boundary |
| docs/THREE-WAY-EQUIVALENCE-PROTOCOL.md | Three-way equivalence protocol |
| scripts/equivalence/three-way-equivalence.ps1 | Three-way equivalence validation |

---

## Governance Model

All changes follow the Librarian governance process:

**Proposal - Impact Analysis - Invariant Review - Owner Authorization - Implementation - Certification**

Evidence is append-only. State may change; evidence does not.
