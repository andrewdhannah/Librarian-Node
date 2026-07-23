# EPIC-MCP-UPWARD-FACE-1

**Status:** Planning — not yet authorized
**Repository:** Librarian-Node

---

## Objective

Implement the Node's upward face — the MCP control plane that exposes governed capabilities to users, agents, and external tools. Every MCP operation flows through governance (custody, evidence, receipt) regardless of the underlying platform adapter.

## Architecture

```
Agent / User
    │
    │ MCP
    ▼
Capability Request
    │
    ├── Identity verification
    ├── Authorization check
    ├── Custody claim
    ├── Capability execution (via platform adapter)
    ├── Evidence collection
    ├── Receipt emission
    └── Response
```

MCP is the control plane interface. It does not know which platform adapter
executes the capability — that is determined by the runtime adapter layer.

```
                MCP (control plane)
                     │
               Capability API
                     │
             ┌───────┴───────┐
             │ Governance Core│
             └───────┬───────┘
                     │
           Capability Providers
                     │
        ┌────────────┼────────────┐
        |            |            |
     macOS        Windows       Linux
    launchd       NSSM         systemd
```

## Capability Model

A capability is a portable, governed operation. It does not know what platform
it runs on. The platform adapter provides the OS-specific implementation.

```
Capability {
    id: "system.session.cleanup"
    requires_authorization: true
    implementation: {
        macOS:  launchd adapter
        Windows: NSSM adapter
        Linux:  systemd adapter
    }
}
```

The MCP client never knows which implementation ran:

```json
// Request
{ "capability": "system.session.cleanup", "target": "project-x" }

// Response receipt
{
  "receipt_type": "Equivalence",
  "schema_version": "1.0.0",
  "action": "capability_execution",
  "evidence_ids": ["evt-cleanup-start", "evt-cleanup-complete"]
}
```

This is the same pattern RuntimeAdapter proved: the governance layer does not
need to know the platform.

## Relationship to Script Convergence

The CONV track (MACOS-001 → CONV-001 → CONV-002 → CONV-003) drives existing
platform-specific scripts toward this capability model. Each script is audited
for what capability it represents, then promoted into a governed capability
with a Rust implementation behind the appropriate platform adapter.

Old model:

```
Platform
 └── Python/shell script
      └── perform operation
```

New model:

```
MCP
 └── Capability Router
      └── Governed capability
           ├── custody
           ├── residency tracking
           ├── evidence generation
           ├── receipt emission
           └── platform adapter (macOS/Windows/Linux)
```

## Prerequisites

- Storage maturity (STORAGE-001): ✅ Complete
- Entity registry (STORAGE-002): Required for capability ownership
- Permissions table (STORAGE-004): Required for multi-tenant auth
- Identity persistence: Required for durable audit

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

## Acceptance Gates

| Gate | Description |
|------|-------------|
| UF-1 | MCP server accepts connections and negotiates protocol |
| UF-2 | Capability Registry enumerates available capabilities using existing contract types |
| UF-3 | Identity verification produces evidence using existing types |
| UF-4 | Authorized actions flow through governance core — custody + evidence + receipt |
| UF-5 | Unauthorized actions are refused with structured response |
| UF-6 | All interactions produce receipts using existing receipt envelope |
| UF-7 | No new governance primitives introduced |

## Relationship to Storage Maturity

The upward face and storage maturity tracks can run in parallel, but with a dependency:

- Identity and auth caching can work without persistence (in-memory for single-tenant)
- Multi-tenant operation with persistent identity → permissions and entity tables needed
- Decision persistence for audit → decision_records table needed

Sprint 1 of this epic could operate without persistence (single-tenant, in-memory). Sprint 2 would add persistence through the matured storage layer.

## Key Invariant

MCP does not know the OS.
Capabilities do not know the OS.
Platform adapters are the only OS-aware layer.
Governance is the only invariant layer.
