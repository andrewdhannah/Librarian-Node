# Librarian Windows Runtime Node

**Status:** Production — Audit Phase  
**Architecture:** ADR-PLATFORM-001 (Core / Node Authority Architecture)  
**Lifecycle:** ADR-PLATFORM-002 (Platform Lifecycle)  

---

## Purpose

This repository contains the Windows Runtime Node for The Librarian platform. The Node is an execution authority component — it runs models, generates evidence, and reports to the Librarian Core.

The repository is used to:

- audit runtime health
- diagnose MCP failures
- validate governance integration
- qualify performance
- review security
- populate governance data
- produce certification evidence

**No architectural changes are performed without Owner authorization.**

## Architecture

The Librarian platform follows a Core / Node authority architecture:

```
Platform
    ├── Installability · Portability · Configuration · Discovery
    │
    ├── Librarian Core
    │   Canonical Authority
    │
    ├── Librarian Node ← YOU ARE HERE
    │   Execution Authority
    │
    ├── MCP Bridge
    │   Protocol Boundary
    │
    └── Shared Contracts
        Truth · Custody · Receipts · Evidence
```

The Node is a governed execution environment. It:
- Executes models via llama.cpp
- Manages runtime lifecycle and hardware
- Generates and records evidence
- Maintains local operational state
- Reports to Core

The Node does NOT:
- Create canonical truth
- Approve capability classification
- Modify governance rules
- Seal evidence as canonical
- Override Core decisions
- Claim canonical authority

## Current State

| Area | Status |
|------|--------|
| Node Identity | ✅ Complete |
| Capability Registry | ✅ Complete |
| Custody | ✅ Complete |
| Runtime Execution | ✅ Complete |
| Operational Dashboard | ✅ Complete |
| Registry Governance | ✅ Complete |
| MCP Contracts | ✅ Complete |
| Installability & Portability | ❌ Planned |
| Core Integration | ⏳ Waiting on Core |

## Sprint Roadmap

```
COMPLETE
-------
Node Foundation
Node Operational Maturity
Node Registry Governance
Platform Architecture Lock

NEXT
----
Node Installability (EPIC-NODE-INSTALLABILITY-AND-PORTABILITY-1)

THEN
----
Node ↔ Core Integration

THEN
----
Multi-Node Coordination

THEN
----
Distributed Platform
```

## Evidence Policy

This repository follows the Librarian governance process:

**Proposal**
    ↓
**Impact Analysis**
    ↓
**Invariant Review**
    ↓
**Owner Authorization**
    ↓
**Implementation**
    ↓
**Certification**

Evidence is append-only. State may change; evidence does not.

## Governance Model

All changes follow the governance documented in `docs/planning/INVARIANT-REVIEW.md` and `docs/planning/IMPACT-ANALYSIS.md`.

---

## License

See [LICENSE](LICENSE).

## Security

See [docs/security/SECURITY-BASELINE.md](docs/security/SECURITY-BASELINE.md) for security policies.
