# Add-on SDK Model

**Status:** Planning
**Prerequisites:** ENTITY-001 ✅, DECISIONS-001 ✅, PERMISSIONS-001 ✅, UF-001 ⏳

---

## What the SDK Exposes

The SDK exposes three things to an add-on: a **capability declaration interface**, an **execution context**, and a **governance client**.

### Capability Declaration Interface

An add-on declares what it can do using the existing `Capability` contract type:

```rust
// Add-on declares a capability
Capability {
    capability_id: "addon.session.cleanup",
    name: "Session Cleanup",
    category: CapabilityCategory::ModelExecution,
    requires_authorization: true,
}
```

The declaration is registered with the `CapabilityRegistry`. No new contract types.

### Execution Context

When invoked, the add-on receives:

- The requesting entity identity (who invoked it)
- The decision that authorized it (why it was approved)
- The permission that allowed it (that they may invoke it)
- Capability parameters

The add-on never sees raw platform state or governance internals.

### Governance Client

The SDK provides a client that handles:

- Custody claim before execution
- ResidencyState tracking during execution
- Evidence generation (uses existing `EvidenceCategory`)
- Receipt emission (uses existing `ReceiptType::Equivalence`)

The add-on calls the governance client. It does not implement governance itself.

## How Add-ons Interact

### Registration

```
Add-on Developer
    ↓
Declares Capability (uses existing Capability type)
    ↓
Capability registered in CapabilityRegistry
    ↓
Entity registered in entity registry (ENTITY-001)
    ↓
Decision records add-on authorization (DECISIONS-001)
    ↓
Permission maps entity → capability (PERMISSIONS-001)
    ↓
Add-on is discoverable via CapabilityRegistry
```

### Invocation

```
Agent / User
    ↓
MCP Request: { capability: "addon.session.cleanup" }
    ↓
Capability Router
    ↓
SDK Governance Client
    ├── Entity check (who is asking?)
    ├── Permission check (are they allowed?)
    ├── Decision check (was it approved?)
    ├── Custody claim
    ├── ResidencyState::Active
    ├── Execute add-on handler
    ├── Evidence generation
    ├── Receipt emission
    └── ResidencyState::Released
    ↓
Response
```

The add-on handler does not perform governance checks. The SDK governance client handles all governance before the handler executes.

## Creating a New Add-on

A new add-on requires:

1. A capability ID (unique, stable — owned by substrate, not protocol)
2. A handler function (receives execution context, returns result)
3. Registration with the `CapabilityRegistry`
4. An entity record in the entity registry (the add-on's owner)
5. A decision authorizing the add-on
6. A permission mapping entity → capability

Example:

```rust
// 1. Define the capability
let capability = Capability {
    capability_id: "addon.report.generate",
    name: "Generate Report",
    category: CapabilityCategory::ModelExecution,
    requires_authorization: true,
    enabled: true,
    schema_version: CAPABILITY_CONTRACT_VERSION,
};

// 2. Register with the framework
registry.register(capability);

// 3. Provide the handler
async fn handle(context: ExecutionContext) -> Result<Output> {
    // context provides: entity_id, parameters, governance client
    let data = context.params.get("report_type");
    let result = generate_report(data);
    // Governance client handles evidence + receipt automatically
    context.governance.emit_evidence(&result)?;
    Ok(result)
}
```

### What the Add-on Does NOT Need

- No governance primitives (new contract types, evidence categories, receipt types)
- No platform adapter code (launchd, NSSM, systemd)
- No custody logic
- No permission evaluation
- No receipt generation

Those are provided by the governance substrate through the SDK client.

## Key Constraint

Add-ons extend capability surface. They do not extend governance authority. An add-on cannot create new permission levels, override decisions, or bypass evidence recording. If an add-on needs a new capability category, that requires a contracts-layer change — not an SDK change.
