# PLATFORM-ADAPTER-BOUNDARY.md — Platform Adapter Boundary

**Version:** 1.0.0
**Status:** Canonical
**Last Updated:** 2026-07-24

---

## Purpose

This document defines the boundary between shared node contracts and platform-specific adapters. It establishes what must be implemented in shared contracts and what can be implemented in platform-specific adapters.

---

## Architectural Principle

```
Shared Contracts (Platform-Neutral)
        |
        ▼
Platform Adapters (Platform-Specific)
        |
        ▼
Platform Execution (OS-Specific)
```

The shared layers define **what** must happen.

The adapters define **how** that operating system does it.

---

## Contract/Adapter Boundary

### Shared Contracts (Must Be Platform-Neutral)

| Contract | Purpose | Platform-Neutral |
|----------|---------|------------------|
| STARTUP-PROTOCOL.md | Defines startup sequence | ✅ Yes |
| STARTUP-OUTPUT-CONTRACT.md | Defines startup receipt format | ✅ Yes |
| SESSION-IDENTITY-CONTRACT.md | Defines node identity format | ✅ Yes |
| Governance contracts | Defines governance rules | ✅ Yes |
| Custody contracts | Defines custody tracking | ✅ Yes |
| Evidence contracts | Defines evidence generation | ✅ Yes |

### Platform Adapters (Must Be Platform-Specific)

| Adapter | Purpose | Platform-Specific |
|---------|---------|-------------------|
| startup-windows.ps1 | Windows startup implementation | ✅ Yes |
| startup-linux.sh | Linux startup implementation | ✅ Yes |
| startup-macos.swift | macOS startup implementation | ✅ Yes |
| node-identity.json | Platform-specific identity | ✅ Yes |
| platform-adapter.md | Platform-specific documentation | ✅ Yes |

---

## Boundary Rules

### Rule 1: Shared Contracts Must Not Contain Platform-Specific Logic

Shared contracts must not contain:
- Platform-specific file paths
- Platform-specific command syntax
- Platform-specific APIs
- Platform-specific dependencies

### Rule 2: Platform Adapters Must Implement Shared Contracts

Platform adapters must:
- Implement all phases of STARTUP-PROTOCOL.md
- Produce receipts conforming to STARTUP-OUTPUT-CONTRACT.md
- Load identities conforming to SESSION-IDENTITY-CONTRACT.md
- Satisfy all validation rules in shared contracts

### Rule 3: Platform Adapters Must Not Modify Shared Contracts

Platform adapters must not:
- Modify shared contract definitions
- Add platform-specific requirements to shared contracts
- Override shared contract validation rules

### Rule 4: Platform Adapters Must Produce Equivalent Outcomes

Platform adapters must produce:
- Identical deterministic fields in receipts
- Equivalent governance decisions
- Equivalent capability declarations
- Equivalent evidence structure

---

## Platform-Specific Implementations

### Windows Adapter

| Contract Requirement | Windows Implementation |
|---------------------|------------------------|
| Identity loading | Windows registry / file system |
| Governance verification | Windows crypto APIs |
| Capability enforcement | Windows ACLs |
| Environment validation | Windows system info |
| Evidence generation | Windows Event Log |
| Receipt validation | Windows certificate store |

**File Locations:**
- Identity: `%APPDATA%\Librarian\node-identity.json`
- Governance: `%APPDATA%\Librarian\governance-sync.json`
- Capabilities: `%APPDATA%\Librarian\capabilities.json`
- Evidence: `%APPDATA%\Librarian\evidence\`
- Receipts: `%APPDATA%\Librarian\receipts\`

### Linux Adapter

| Contract Requirement | Linux Implementation |
|---------------------|----------------------|
| Identity loading | /etc/librarian/ or ~/librarian/ |
| Governance verification | OpenSSL |
| Capability enforcement | Linux permissions |
| Environment validation | /proc/cpuinfo, uname |
| Evidence generation | syslog/journald |
| Receipt validation | OpenSSL verification |

**File Locations:**
- Identity: `/etc/librarian/node-identity.json`
- Governance: `/etc/librarian/governance-sync.json`
- Capabilities: `/etc/librarian/capabilities.json`
- Evidence: `/var/librarian/evidence/`
- Receipts: `/var/librarian/receipts/`

### macOS Adapter

| Contract Requirement | macOS Implementation |
|---------------------|----------------------|
| Identity loading | ~/Library/Librarian/ |
| Governance verification | Security framework |
| Capability enforcement | macOS entitlements |
| Environment validation | sysctl, sw_vers |
| Evidence generation | Unified logging |
| Receipt validation | CryptoKit |

**File Locations:**
- Identity: `~/Library/Librarian/node-identity.json`
- Governance: `~/Library/Librarian/governance-sync.json`
- Capabilities: `~/Library/Librarian/capabilities.json`
- Evidence: `~/Library/Librarian/evidence/`
- Receipts: `~/Library/Librarian/receipts/`

---

## Equivalence Validation

### Deterministic Fields (Must Match Across Platforms)

| Field | Windows | Linux | macOS |
|-------|---------|-------|-------|
| `governance_commit` | Same | Same | Same |
| `identity_loaded` | Same | Same | Same |
| `governance_verified` | Same | Same | Same |
| `capabilities_loaded` | Same | Same | Same |
| `environment_validated` | Same | Same | Same |
| `checks_passed` | Same | Same | Same |
| `checks_failed` | Same | Same | Same |
| `status` | Same | Same | Same |

### Variable Fields (Expected to Differ Across Platforms)

| Field | Windows | Linux | macOS |
|-------|---------|-------|-------|
| `receipt_id` | WINDOWS-STARTUP-* | LINUX-STARTUP-* | MACOS-STARTUP-* |
| `node_id` | WINPC-* | LINUX-* | MACOS-* |
| `platform` | windows | linux | macos |
| `timestamp` | Different | Different | Different |

---

## References

- Node Reference Architecture: `NODE-REFERENCE-ARCHITECTURE.md`
- Three-Way Equivalence Protocol: `THREE-WAY-EQUIVALENCE-PROTOCOL.md`
- Startup Protocol: `contracts/startup/STARTUP-PROTOCOL.md`
- Startup Output Contract: `contracts/startup/STARTUP-OUTPUT-CONTRACT.md`
- Session Identity Contract: `contracts/startup/SESSION-IDENTITY-CONTRACT.md`
