# macOS Platform Adapter

**Version:** 1.0.0
**Platform:** macOS
**Status:** Reference Implementation

---

## Purpose

This document describes the macOS-specific implementation of the Librarian Node startup protocol.

---

## Startup Implementation

### File: startup-macos.swift

**Language:** Swift 5.0+
**Execution:** `swift startup-macos.swift`

### Implementation Details

| Contract Requirement | macOS Implementation |
|---------------------|----------------------|
| Identity loading | JSON file via Foundation |
| Governance verification | JSON file validation via Foundation |
| Capability loading | JSON file validation via Foundation |
| Environment validation | ProcessInfo check |
| Receipt generation | JSON file creation via Codable |
| Governed mode entry | State update |

### File Locations

| File | Purpose | Path |
|------|---------|------|
| node-identity.json | Node identity | `~/Library/Librarian/node-identity.json` |
| governance-sync.json | Governance state | `~/Library/Librarian/governance-sync.json` |
| capabilities.json | Node capabilities | `~/Library/Librarian/capabilities.json` |
| startup-receipt-*.json | Startup receipt | `~/Library/Librarian/evidence/startup/startup-receipt-*.json` |

---

## Validation

This adapter satisfies:

- ✅ SH-3: macOS startup behavior maps to contract
- ✅ STARTUP-PROTOCOL.md: All 6 phases implemented
- ✅ STARTUP-OUTPUT-CONTRACT.md: Receipt format compliant
- ✅ SESSION-IDENTITY-CONTRACT.md: Identity format compliant

---

## Usage

```bash
swift startup-macos.swift --node-path ~/Library/Librarian --output-path ~/Library/Librarian/evidence/startup
```

---

## References

- Startup Protocol: `contracts/startup/STARTUP-PROTOCOL.md`
- Startup Output Contract: `contracts/startup/STARTUP-OUTPUT-CONTRACT.md`
- Session Identity Contract: `contracts/startup/SESSION-IDENTITY-CONTRACT.md`
