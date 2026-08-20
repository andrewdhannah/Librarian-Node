# Linux Platform Adapter

**Version:** 1.0.0
**Platform:** Linux
**Status:** Reference Implementation

---

## Purpose

This document describes the Linux-specific implementation of the Librarian Node startup protocol.

---

## Startup Implementation

### File: startup-linux.sh

**Language:** Bash 4.0+
**Execution:** `chmod +x startup-linux.sh`

### Implementation Details

| Contract Requirement | Linux Implementation |
|---------------------|----------------------|
| Identity loading | JSON file via jq |
| Governance verification | JSON file validation via jq |
| Capability loading | JSON file validation via jq |
| Environment validation | Bash version check |
| Receipt generation | JSON file creation via jq |
| Governed mode entry | State update |

### File Locations

| File | Purpose | Path |
|------|---------|------|
| node-identity.json | Node identity | `/etc/librarian/node-identity.json` |
| governance-sync.json | Governance state | `/etc/librarian/governance-sync.json` |
| capabilities.json | Node capabilities | `/etc/librarian/capabilities.json` |
| startup-receipt-*.json | Startup receipt | `/var/librarian/evidence/startup/startup-receipt-*.json` |

---

## Validation

This adapter satisfies:

- ✅ SH-5: Linux startup behavior maps to contract
- ✅ STARTUP-PROTOCOL.md: All 6 phases implemented
- ✅ STARTUP-OUTPUT-CONTRACT.md: Receipt format compliant
- ✅ SESSION-IDENTITY-CONTRACT.md: Identity format compliant

---

## Usage

```bash
./startup-linux.sh --node-path /etc/librarian --output-path /var/librarian/evidence/startup
```

---

## References

- Startup Protocol: `contracts/startup/STARTUP-PROTOCOL.md`
- Startup Output Contract: `contracts/startup/STARTUP-OUTPUT-CONTRACT.md`
- Session Identity Contract: `contracts/startup/SESSION-IDENTITY-CONTRACT.md`
