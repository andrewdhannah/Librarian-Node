# Add-on SDK Model

**Status:** Planning
**Prerequisites:** ENTITY-001 ✅, DECISIONS-001 ✅, PERMISSIONS-001 ✅, UF-001 ⏳

---

## Core Principle

Add-ons extend capability surface. They do not extend governance authority.
An add-on is simply software that registers one or more capabilities.
The governance layer does not need an "add-on" concept — it only needs
entities, capabilities, decisions, and permissions.

## What the SDK Exposes

The SDK exposes four things to an add-on: a **capability declaration interface**,
an **execution context**, a **governance client**, and a **storage client**.

### Capability Declaration Interface

An add-on declares what it can do using the existing `Capability` contract type.
From the core's perspective, an add-on capability is indistinguishable from
a built-in capability — the router routes to the registered provider.

```rust
Capability {
    capability_id: "conversation.import",
    name: "Import Conversations",
    category: CapabilityCategory::InformationProcessing,
    requires_authorization: true,
}
```

### Add-on Manifest

Each add-on also provides an identity document for discovery:

```rust
AddonManifest {
    addon_id: "claude-conversation-ingestion",
    version: "1.0.0",
    sdk_version: 1,
    provider: "local",
    storage: Some(StorageDecl { r#type: "sqlite" }),
    capabilities: vec!["conversation.import", "conversation.search"],
    permissions: vec!["filesystem.read"],
}
```

This is metadata — not governance. It tells the router what exists and what
the add-on needs to operate.

### Add-on Lifecycle

The SDK manages add-on lifecycle states. The CapabilityRegistry knows a
capability exists, but it also needs to know whether the provider can
currently execute.

```rust
enum AddonLifecycleState {
    Installed,
    Initializing,
    Ready,
    Degraded,
    Disabled,
    Removed,
}
```

### Capability Health

Discovery should include health status so the router does not invoke
broken providers:

```rust
CapabilityHealth {
    capability_id: "conversation.search",
    status: HealthStatus::Degraded,
    last_check: "...",
    diagnostics: "Embeddings index rebuilding — estimated 2 min",
}
```

### Execution Context

When invoked, the add-on receives only what its handler needs:

- Requesting entity identity (who invoked it)
- Decision that authorized it (why it was approved)
- Permission that allowed it (that they may invoke it)
- Capability parameters

The add-on never sees raw platform state, governance internals,
database handles, or custody state.

### Governance Client (Middleware)

The SDK applies governance before the handler executes:

```
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
```

The handler never performs governance checks itself.

### Storage Client

Each add-on owns its own private database. The SDK provisions isolated
storage so the add-on does not need to know where data lives:

```
addons/
    conversation-ingestion/
        data.db
    report-generator/
        data.db
    obsidian-indexer/
        graph.db
```

The SDK provides:

- `storage.open()` — get handle to add-on's private database
- `storage.health()` — database reachable and consistent
- `storage.backup()` — snapshot add-on data
- `storage.vacuum()` — reclaim space
- `storage.schema_version()` — current migration level

The core never opens add-on databases directly.

### Migration Contract

Each add-on owns its schema lifecycle through the SDK:

```rust
trait AddonMigration {
    fn current_version(&self) -> u32;
    fn migrate(&self, from: u32, to: u32) -> Result<()>;
}
```

Add-ons register migrations with the SDK on initialization. The SDK
ensures migrations run in order before the add-on becomes Ready.

### Provenance Contract

Add-ons that ingest external data should record provenance so the
governance layer can trace where information originated:

```rust
IngestionProvenance {
    source: "claude_export.json",
    imported_at: "2026-07-23T00:00:00Z",
    source_hash: "sha256:abc123...",
    provider: "Claude",
    external_id: "conv-xyz",
    derived_records: 12,
    entities_detected: 24,
}
```

This enables the governance layer to answer: "Where did this design
idea originate?" — traceable back through receipts, evidence, and
provenance records.

## How Add-ons Interact

### Registration

```
Add-on Developer
    ↓
Declares Capability + Manifest
    ↓
Capability registered in CapabilityRegistry
    ↓
Entity registered in entity registry (ENTITY-001)
    ↓
Decision records add-on authorization (DECISIONS-001)
    ↓
Permission maps entity → capability (PERMISSIONS-001)
    ↓
Add-on initializes (migrations run)
    ↓
Health check → Ready
    ↓
Discoverable via CapabilityRegistry + Health
```

### Invocation

```
Agent / User
    ↓
MCP Request: { capability: "conversation.search" }
    ↓
Capability Router
    ├── Check health (is provider ready?)
    ├── Check permission (is entity allowed?)
    ├── Check decision (was it approved?)
    ├── Route to add-on provider
    ↓
SDK Governance Client
    ├── Identity check
    ├── Permission check
    ├── Decision check
    ├── Custody claim
    ├── ResidencyState::Active
    ├── Execute add-on handler
    ├── Evidence generation
    ├── Receipt emission
    └── ResidencyState::Released
    ↓
Response
```

## Creating a New Add-on

A new add-on requires:

1. An add-on manifest (identity, version, permissions)
2. A capability declaration (what it can do)
3. A handler function (receives execution context, returns result)
4. An entity record in the entity registry (the add-on's owner)
5. A decision authorizing the add-on
6. A permission mapping entity → capability
7. Optional: migrations for private storage

Example:

```rust
// 1. Define manifest
let manifest = AddonManifest {
    addon_id: "claude-conversation-ingestion",
    version: "1.0.0",
    sdk_version: 1,
    capabilities: vec!["conversation.import", "conversation.search"],
    storage: Some(StorageDecl { r#type: "sqlite" }),
};

// 2. Register capability
registry.register(Capability {
    capability_id: "conversation.import",
    name: "Import Conversations",
    category: CapabilityCategory::InformationProcessing,
    requires_authorization: true,
    enabled: true,
    schema_version: CAPABILITY_CONTRACT_VERSION,
});

// 3. Provide handler
async fn handle_import(context: ExecutionContext) -> Result<Output> {
    let file = context.params.get("source_path");
    let db = context.storage.open()?;
    let provenance = ingest_file(file, db)?;
    context.governance.emit_evidence(&provenance)?;
    Ok(Output { records: provenance.derived_records })
}
```

## What the Add-on Does NOT Need

- No governance primitives (new contract types, evidence categories, receipt types)
- No platform adapter code (launchd, NSSM, systemd)
- No custody logic
- No permission evaluation
- No receipt generation
- No database provisioning

Those are provided by the governance substrate through the SDK.

## Reference Add-on: Conversation Ingestion

The Claude/ChatGPT archive converter is a natural first SDK add-on.
It exercises nearly every SDK boundary:

| Boundary | Exercised By |
|----------|-------------|
| Capability declaration | `conversation.import`, `conversation.search` |
| Add-on manifest | AddonManifest with storage decl |
| Add-on lifecycle | Install → Initialize (migrate DB) → Ready |
| Health reporting | Index status, DB integrity |
| Storage isolation | Private conversation.db |
| Migration contract | Schema versions for conversation store |
| Provenance | Source hash, import timestamp, entity count |
| Evidence generation | Import receipts |
| Multi-capability | Separate import, search, export capabilities |

The existing HTML tool (`claude_archive_drop_site_v2.html`) does the
conversion work. The add-on wraps it in governance: custody before
import, evidence after import, provenance tracking, search capability.

## Artifact Ownership

Add-ons that derive new artifacts from source data must preserve the
distinction between artifact classes:

| Class | Owner | Example |
|-------|-------|---------|
| Source artifact | Source/importer | Claude export JSON |
| Derived artifact | Transformation pipeline | Normalized conversation, Markdown note, extracted entity |
| Governance artifact | Librarian Core | Evidence record, receipt, provenance |

Each derived artifact should record:

```json
{
  "artifact_id": "conv-abc-123",
  "type": "conversation_note",
  "source_artifact": "claude_export_xyz",
  "created_by": "conversation-ingestion",
  "governance_owner": "librarian"
}
```

This prevents ambiguity when multiple add-ons transform the same information.

## Future: Capability Composition

The current model routes individual capability requests. A future extension
could allow the router to compose capabilities across add-ons:

```
conversation.import
        ↓
knowledge.extract_entities
        ↓
compare_against_work_ledger
        ↓
generate_review_queue
```

The router could orchestrate this without any add-on knowing about the others.
Each capability remains independently governed — the composition is a routing
concern, not a governance concern.

## Key Constraint

Governance does not know what an add-on is. It knows entities,
capabilities, decisions, and permissions. An add-on is simply a
provider that registered one or more capabilities. The same routing,
authorization, evidence, and receipt pipeline applies regardless
of whether a capability came from the core, a built-in module, an
SDK add-on, or a future remote provider.
