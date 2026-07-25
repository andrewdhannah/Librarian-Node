# Windows Platform Adapter

**Version:** 1.0.0
**Platform:** Windows
**Status:** Reference Implementation

---

## Purpose

This document describes the Windows-specific implementation of the Librarian Node startup protocol.

---

## Startup Implementation

### File: startup-windows.ps1

**Language:** PowerShell 5.0+
**Execution Policy:** Bypass (recommended)

### Implementation Details

| Contract Requirement | Windows Implementation |
|---------------------|------------------------|
| Identity loading | JSON file in node directory |
| Governance verification | JSON file validation |
| Capability loading | JSON file validation |
| Environment validation | PowerShell version check |
| Receipt generation | JSON file creation |
| Governed mode entry | State update |

### File Locations

| File | Purpose | Path |
|------|---------|------|
| node-identity.json | Node identity | `<NodePath>\node-identity.json` |
| governance-sync.json | Governance state | `<NodePath>\governance-sync.json` |
| capabilities.json | Node capabilities | `<NodePath>\capabilities.json` |
| startup-receipt-*.json | Startup receipt | `<OutputPath>\startup-receipt-*.json` |

---

## Validation

This adapter satisfies:

- ✅ SH-4: Windows startup behavior maps to contract
- ✅ STARTUP-PROTOCOL.md: All 6 phases implemented
- ✅ STARTUP-OUTPUT-CONTRACT.md: Receipt format compliant
- ✅ SESSION-IDENTITY-CONTRACT.md: Identity format compliant

---

## Usage

```powershell
.\startup-windows.ps1 -NodePath "G:\OpenWork\runtime-node" -OutputPath "G:\OpenWork\evidence\reference-architecture"
```

---

## References

- Startup Protocol: `contracts/startup/STARTUP-PROTOCOL.md`
- Startup Output Contract: `contracts/startup/STARTUP-OUTPUT-CONTRACT.md`
- Session Identity Contract: `contracts/startup/SESSION-IDENTITY-CONTRACT.md`
