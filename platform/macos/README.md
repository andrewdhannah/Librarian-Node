# macOS Platform Adapter

**Version:** 1.0.0
**Platform:** macOS
**Status:** Reference Implementation

---

## Purpose

macOS-specific implementation artifacts for the Librarian Node. These are platform adapters — not part of the portable governance layer.

## Architecture

```
macOS Runtime
     |
     | launchd / sysctl / Unified Logging
     v
macOS Adapter (RuntimeAdapter impl)
     |
     | ProcessEvent
     v
RuntimeSupervisor (governance core)
     |
     | ResidencyState + Evidence + Receipt
     v
GovernanceDb
```

## Adapter Mapping

| macOS Concern | Adapter |
|---------------|---------|
| Service manager | launchd |
| Process discovery | sysctl |
| File paths | `~/Library/Librarian/` |
| Logging | Unified logging (`os_log`) |
| Security | Security framework / CryptoKit |
| Startup language | Swift (startup-macos.swift) |

## Contents

| File | Purpose |
|------|---------|
| `startup-macos.swift` | 6-phase startup protocol implementation |
| `node-identity.json` | macOS node identity |
| `capabilities.json` | macOS node capabilities |
| `platform-adapter.md` | Platform adapter documentation |

## Non-Goals

- No macOS-specific governance concepts
- No macOS-specific receipt types
- No macOS-specific evidence categories
- No macOS-specific lifecycle states
- No macOS-specific residency states

## Build Target

```
macOS 10.14+ (Mojave)
Swift 5.0+
```

## Equality Status

macOS is a first-class platform in the Librarian-Node architecture. All three platforms (Windows, Linux, macOS) implement the same governed contract and produce equivalent startup receipts.

## Evidence

macOS platform evidence is collected under `evidence/phase0/reference-architecture/`.
