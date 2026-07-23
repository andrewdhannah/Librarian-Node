# EPIC-MCP-UPWARD-FACE-1

**Status:** Planning — not yet authorized
**Repository:** Librarian-Node

---

## Objective

Implement the Node's upward face: the MCP server, capability router, SDK add-on interface, and identity/auth layer that allows users, agents, and external tools to interact with the governance substrate through governed capabilities.

## Prerequisites

Storage maturity (numbered migrations, entity registry, permissions table) should be complete or in parallel, because multi-tenant upward-face operation depends on identity and permissions storage.

## Scope

| Component | Description | Depends On |
|-----------|-------------|------------|
| MCP Server | MCP protocol endpoint for agent/tool connections | Contracts stable |
| Capability Router | Route requests to registered capabilities | CapabilityRegistry contract |
| Capability Registry | Enumerate capabilities via MCP | CapabilityRegistry contract |
| SDK Add-on Interface | Allow add-ons to declare governed capabilities | CapabilityRegistry contract |
| Identity Manager | Authenticate principals connecting via MCP | NodeIdentity contract |
| Auth Manager | Authorize actions by identity + capability | Storage: permissions table |

## Non-Scope

- New contract types for capabilities (existing `Capability` and `CapabilityRegistry` are sufficient)
- New evidence or receipt types (existing types cover all governance events)
- New lifecycle or residency states
- Modifying existing governance engines
- Platform adapter changes

## Architecture

```
Agent / Tool / User
    │
    │  MCP protocol
    ▼
MCP Server
    │
    ▼
Capability Router ─── CapabilityRegistry (contracts)
    │
    ├─── Identity Manager ─── NodeIdentity (contracts)
    │
    ├─── Auth Manager ─────── permissions (storage)
    │
    ▼
Governance Core (existing)
    │
    ▼
Evidence + Receipt + Custody
```

## Acceptance Gates

| Gate | Description |
|------|-------------|
| UF-1 | MCP server accepts connections and negotiates protocol |
| UF-2 | Capability Registry enumerates available capabilities using existing contract types |
| UF-3 | Identity verification produces evidence using existing types |
| UF-4 | Authorized actions flow through governance core |
| UF-5 | Unauthorized actions are refused with structured response |
| UF-6 | All interactions produce receipts using existing receipt envelope |
| UF-7 | No new governance primitives introduced |

## Relationship to Storage Maturity

The upward face and storage maturity tracks can run in parallel, but with a dependency:

- Identity and auth caching can work without persistence (in-memory for single-tenant)
- Multi-tenant operation with persistent identity → permissions and entity tables needed
- Decision persistence for audit → decision_records table needed

Sprint 1 of this epic could operate without persistence (single-tenant, in-memory). Sprint 2 would add persistence through the matured storage layer.
