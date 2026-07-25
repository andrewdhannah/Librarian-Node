# NODE-REFERENCE-ARCHITECTURE.md — Cross-Platform Node Specification

**Version:** 1.0.0
**Status:** Canonical
**Last Updated:** 2026-07-24

---

## Purpose

This document defines the canonical node specification that all Librarian Node implementations must satisfy. It establishes the architectural foundation for cross-platform governance, execution, and evidence generation.

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
Phase 1: Identity Loading
    │
    ▼
Phase 2: Governance Verification
    │
    ▼
Phase 3: Capability Loading
    │
    ▼
Phase 4: Environment Validation
    │
    ▼
Phase 5: Startup Receipt Generation
    │
    ▼
Phase 6: Enter Governed Mode
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

## Equivalence Requirements

All platform implementations must produce equivalent startup receipts. Equivalence is defined as:

**Deterministic Fields (Must Match):**
- `governance_commit`
- `identity_loaded`
- `governance_verified`
- `capabilities_loaded`
- `environment_validated`
- `checks_passed`
- `checks_failed`
- `status`

**Variable Fields (Expected to Differ):**
- `receipt_id`
- `node_id`
- `platform`
- `timestamp`

---

## References

- Startup Protocol: `contracts/startup/STARTUP-PROTOCOL.md`
- Startup Output Contract: `contracts/startup/STARTUP-OUTPUT-CONTRACT.md`
- Session Identity Contract: `contracts/startup/SESSION-IDENTITY-CONTRACT.md`
- Platform Adapter Boundary: `docs/PLATFORM-ADAPTER-BOUNDARY.md`
- Three-Way Equivalence Protocol: `docs/THREE-WAY-EQUIVALENCE-PROTOCOL.md`
