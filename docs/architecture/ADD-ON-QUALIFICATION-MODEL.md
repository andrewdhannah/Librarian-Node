# Add-on Qualification Model

**Status:** Planning
**Prerequisites:** SDK contract model, `QualificationHarness` (existing)

---

## Principle

The qualification system should not know what it is qualifying. It qualifies
governed capability providers. The provider can be a model profile, a runtime
node, an add-on, or a future external capability — the harness is generic.

Existing harness:

```
Capability Provider
    ↓
QualificationHarness  (existing — reused)
    ↓
Evidence + Receipt
    ↓
Ready
```

## Add-on Qualification Lifecycle

```
Registered
    ↓
Manifest Validation
    ↓
Capability Validation
    ↓
Governance Boundary Validation     ← NEW gate
    ↓
Storage Migration Validation
    ↓
Health Validation
    ↓
Provenance Validation
    ↓
Qualified
    ↓
Ready (available via CapabilityRegistry)
```

Each gate produces an `EvidenceRecord` and may produce a `Receipt`.
Failed gates block the transition to `Ready`.

---

## Gate 1: Governance Boundary Qualification

**Purpose:** Enforce that add-ons extend capability, not authority.

**Input:**

- Add-on manifest (capability IDs, permission requests)
- Capability declarations
- Receipt type references
- Evidence category references

**Checks:**

| Check | Allowed | Rejected |
|-------|---------|----------|
| CapabilityCategory values | Existing enum values only | New variant or string |
| EvidenceCategory values | Existing enum values only | New variant or string |
| ReceiptType values | Existing enum values only | New variant or string |
| Permission requests | Match existing permission model | New authority paths |
| Custody operations | Existing `CustodyAction` values | New custody semantics |
| Governance primitives | None introduced | Any new type or variant |

**Implementation:** Reuses `EquivalenceHarness.check_contract_equivalence()` —
the add-on's declarations are compared against the allowed governance surface.
Same check infrastructure, different input.

```
Add-on declarations
    ↓
EquivalenceHarness.check_contract_equivalence()
    ↓
PASS → proceed to Gate 2
FAIL → add-on rejected — governance leak detected
```

### Negative Tests

Before broad SDK adoption, explicit tests should prove the boundary is enforced:

```
Attempt: Add-on creates a new CapabilityCategory variant
Result:  Rejected — governance leak

Attempt: Add-on emits unauthorized ReceiptType
Result:  Rejected

Attempt: Add-on bypasses governance client
Result:  Rejected

Attempt: Add-on creates permissions directly
Result:  Rejected
```

---

## Gate 2: Capability Compatibility Qualification

**Purpose:** Validate that declared capability versions are compatible with
the substrate's capability registry.

**Input:**

- Declared capability IDs and versions
- Substrate capability registry

**Checks:**

| Scenario | Resolution |
|----------|-----------|
| Capability does not exist in registry | New registration allowed (first add-on) |
| Capability exists at same version | Must pass compatibility check |
| Capability exists at newer version | Version resolver determines path |
| Capability exists at older version | Migration check required |

Reuses the existing version resolution pattern from `MigrationRunner`:

```
Capability Request
    ↓
Version Resolver
    ↓
Migration / Compatibility Check
    ↓
Evidence
    ↓
Receipt
```

Version evolution becomes auditable rather than an implicit assumption.

---

## Gate 3: Storage Migration Validation

**Purpose:** Verify that add-on storage migrations run correctly and do not
affect other components.

**Input:**

- Add-on migration list (from `AddonMigration` trait)
- Private add-on database

**Checks:**

| Check | Method |
|-------|--------|
| Migrations run in order | MigrationRunner (reused) |
| Migrations are idempotent | Run twice, verify same result |
| Rollback works | Apply migration, roll back, verify state |
| No cross-database effects | Verify only add-on's private DB changed |

Migrates add-on storage to the latest schema version, then reports health.

---

## Gate 4: Health Validation

**Purpose:** Confirm the add-on can become `Ready` and respond to health checks.

**Input:**

- Add-on provider instance

**Checks:**

| Check | Method |
|-------|--------|
| Provider responds | Health check request |
| Storage reachable | `storage.health()` |
| Capabilities enumerable | `CapabilityRegistry` reports declared capabilities |
| No startup errors | Lifecycle log |

---

## Gate 5: Provenance Chain Validation

**Purpose:** Verify that add-ons which ingest data preserve source provenance
through all transformations.

**Input:**

- Sample ingestion pipeline (for conversation ingestion: Claude JSON → artifacts)

**Checks:**

| Link | Must Have |
|------|-----------|
| Source artifact | hash, importer identity, timestamp |
| Derived artifact | parent reference to source |
| Entity extraction | source link to derived artifact |
| Governance action | receipt chain from evidence |

A missing link fails qualification:

```
claude_export.json
      ↓
conversation_001          ← has source hash + importer
      ↓
entity: CapabilityRouter  ← has source link to conversation_001
      ↓
design_finding_001        ← has source link to entity
      ↓
review_decision_001       ← has receipt chain
```

Example provenance record:

```json
{
  "artifact_id": "conv-abc-123",
  "type": "conversation_note",
  "source_artifact": "claude_export_xyz",
  "source_hash": "sha256:abc...",
  "created_by": "conversation-ingestion",
  "derived_artifacts": ["entity-capability-router"],
  "governance_owner": "librarian"
}
```

---

## Gate 6: Qualification Receipt

After all gates pass, a qualification receipt is emitted:

```
Subject:    claude-conversation-ingestion
Status:     QUALIFIED

Checks:
  governance_boundary   PASS
  capability_version    PASS
  storage_migration     PASS
  provenance_chain      PASS
  health                PASS

Evidence IDs:  [evt-qual-gov-001, evt-qual-cap-001, ...]
Receipt ID:    REC-Q-CONV-INGEST-001
```

This gives the same evidence trail as model qualification and node qualification.
The receipt type is existing `ReceiptType::Equivalence` — no new receipt type needed.

---

## Implementation Strategy

The existing `QualificationHarness` in `librarian-core/src/governance/qualification/`
remains generic. Add-on qualification adds an adapter:

```
QualificationHarness (existing — unchanged)
    │
    ├── ModelQualificationAdapter (existing — model profiles)
    │
    ├── AddonQualificationAdapter ← NEW
    │     ├── governance boundary checks
    │     ├── capability version resolution
    │     ├── storage migration validation
    │     ├── health checks
    │     └── provenance chain validation
    │
    └── RuntimeQualificationAdapter (future — node qualification)
```

The harness remains subject-agnostic. Adapters provide subject-specific checks.
This preserves the pattern established with the substrate: contracts first,
adapters second.

## Acceptance Gates for SDK Epic

| Gate | Description |
|------|-------------|
| SDK-1 | SDK crate exists, compiles, depends only on librarian-contracts |
| SDK-2 | Add-on manifest declares identity, version, capabilities, storage |
| SDK-3 | Add-on lifecycle tracked |
| SDK-4 | Governance boundary qualification prevents authority leaks |
| SDK-5 | Capability version resolution works |
| SDK-6 | Storage migration validation works |
| SDK-7 | Provenance chain validation works |
| SDK-8 | Qualification receipt emitted with existing ReceiptType |
| SDK-9 | No new governance primitives introduced |
