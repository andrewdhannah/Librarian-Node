# Librarian Platform Contract Specification v1.0

**Canonical specification for the Librarian platform contract layer.**
**EPIC-MCP-CONTRACT-CANONICALIZATION-1, CB-004 prerequisite.**

This document defines the platform specification that all Librarian runtimes
(Swift, Rust, and future implementations) must satisfy. The specification is
owned by the contracts layer, not by any runtime implementation.

---

## 0. Governing Principle

**No runtime implementation may define platform contracts. It may only implement
qualified platform contracts.**

This mirrors the existing platform philosophy:
- Models do not define governance.
- Agents do not define authority.
- Evidence does not define policy.
- Runtimes do not define contracts.

---

## 1. Canonical Contract Inventory

Every type in the platform contract surface. These types are defined in the
`librarian-contracts` Rust crate and serialized to JSON for transport. All
runtimes must produce and consume identical JSON representations.

### 1.1 Capability Registry

| Contract | Module | Request | Response |
|----------|--------|---------|----------|
| Search | `capability_registry::search` | `SearchRequest` | `SearchResponse` |
| List | `capability_registry::search` | `ListRequest` | `ListResponse` |
| Resolve | `capability_registry::resolve` | `ResolveRequest` | `ResolveResponse` |
| Load | `capability_registry::load` | `LoadRequest` | `LoadResponse` |
| Import | `capability_registry::import_skill` | `ImportRequest` | `ImportResponse` |
| Evidence Query | `capability_registry::evidence` | `EvidenceQueryRequest` | `EvidenceQueryResponse` |
| Task History | `capability_registry::evidence` | `TaskHistoryRequest` | `TaskHistoryResponse` |
| Agent Usage | `capability_registry::evidence` | `AgentUsageRequest` | `AgentUsageResponse` |
| Revoke Impact | `capability_registry::evidence` | `RevokeImpactRequest` | `RevokeImpactResponse` |
| Summary | `capability_registry::evidence` | — | `AgentSummaryResponse` |

### 1.2 Identity

| Contract | Module | Types |
|----------|--------|-------|
| Node Identity | `identity` | `NodeIdentity`, `PlatformVersion`, `GovernanceIdentity` |
| Capability Identity | `capability_registry::load` | `CapabilityIdentity` |

### 1.3 Evidence

| Contract | Module | Types |
|----------|--------|-------|
| Capability Event | `capability_registry::evidence` | `CapabilityEvidenceEvent` |
| Resolution Receipt | `capability_registry::resolve` | `ResolveReceipt` |
| Load Receipt | `capability_registry::load` | `LoadReceipt` |
| Agent Summary | `capability_registry::evidence` | `AgentUsageSummary` |
| Revoke Impact | `capability_registry::evidence` | `RevokeImpact` |

### 1.4 Supporting Types

| Type | Module | Role |
|------|--------|------|
| `CapabilityStatus` | `capability_registry::types` | Lifecycle status enum |
| `CapabilityType` | `capability_registry::types` | Type classification enum |
| `SecurityClassification` | `capability_registry::types` | Security level enum |
| `SourceType` | `capability_registry::types` | Provenance enum |
| `CapabilitySummary` | `capability_registry::types` | Metadata-only summary |
| `ImportActionResult` | `capability_registry::types` | Import result |

---

## 2. Versioning and Compatibility Rules

### 2.1 Contract Version

The current contract version is **1.0.0**. It is tracked in
`librarian-contracts/src/capabilities.rs` as `CAPABILITY_CONTRACT_VERSION`.

### 2.2 Field Lifecycle

| Annotation | Meaning |
|------------|---------|
| **STABLE** | Field will not change within this major version. |
| **PROVISIONAL** | Field exists but may change. Must be implemented to enable evolution. |
| **DEPRECATED** | Field will be removed in the next major version. Runtimes must implement but should warn consumers. |

### 2.3 Backward Compatibility Rules

A change is backward-compatible if:

1. **Adding an optional field** — new field has a default value, existing payloads deserialize correctly.
2. **Adding a variant to an enum** — consumers handle unknown variants gracefully.
3. **Relaxing a constraint** — making a required field optional.

A change is breaking if:

1. **Removing a field** — existing payloads fail to deserialize.
2. **Making an optional field required** — existing payloads fail to deserialize.
3. **Changing a field type** — existing payloads fail to deserialize.
4. **Renaming a field** — existing payloads fail to deserialize.
5. **Removing an enum variant** — existing payloads fail to deserialize.

### 2.4 Version Transition

```
Major version: breaking changes
Minor version: backward-compatible additions
Patch version: fixes that don't change the wire format
```

---

## 3. Qualification Levels

Every contract must pass five levels of qualification before a runtime can
claim conformance.

### Q-1: Structural (Schema Conformance)

**Proves:** Wire format conforms to the platform JSON Schema.

**Method:** Validate every payload against `docs/schemas/*.schema.json`.

**Pass criteria:** All required fields present. All field types match. No
extraneous fields. Enum values are valid.

**Artifact:** `schemas/mcp-*.schema.json`

### Q-2: Representational (Serialization Equivalence)

**Proves:** Multiple implementations encode/decode contract types identically.

**Method:** Serialize a typed value in Rust → JSON. Deserialize in Swift →
typed value. Serialize back to JSON. Compare. Repeat in both directions.

**Pass criteria:** Round-trip produces byte-identical JSON (or semantically
identical where ordering is irrelevant, e.g., map keys).

**Artifact:** `librarian-contracts/src/capability_registry/*.rs`

### Q-3: Behavioral (Semantic Equivalence)

**Proves:** Same request produces the same domain result across runtimes.

**Method:** Submit identical `ResolveRequest` to both Swift and Rust
implementations. Compare `ResolveResponse` field by field.

**Pass criteria:** Identical field values. Identical default handling.
Identical optional behavior. Identical error responses for invalid inputs.

**Artifact:** Runtime implementations

### Q-4: Deterministic (Output Stability)

**Proves:** Repeated execution of the same request produces identical outputs
and receipts.

**Method:** Execute the same capability load 100 times with identical inputs.
Verify every receipt has the same content_hash, same identity fields, same
constraints.

**Pass criteria:** All 100 runs produce identical receipts. Content hash
matches SHA-256 of body. Receipt fields match request fields.

**Artifact:** `CapabilityContext.receipt`, evidence events

### Q-5: Evolution (Migration Safety)

**Proves:** Contract changes are classified correctly and the affected surface
is identified automatically.

**Method:** Compare two versions of a contract type. Classify each field
change as breaking or non-breaking. Identify which runtimes and transports
must be updated.

**Pass criteria:** Breaking changes are detected and blocked. Non-breaking
changes proceed. Affected components are reported.

**Artifact:** Contract diff tool

---

## 4. Pass/Fail Criteria

| Level | Pass | Fail |
|-------|------|------|
| Q-1 | All payloads validate against JSON Schema | Any payload fails validation |
| Q-2 | All round-trip tests produce identical JSON | Any round-trip produces different JSON |
| Q-3 | All field-by-field comparisons match | Any field differs between implementations |
| Q-4 | 100/100 identical runs | Any run produces a different output |
| Q-5 | Breaking changes blocked, non-breaking classified correctly | Misclassified change type |

---

## 5. Release Rules

1. **No contract change merges without Q-1 passing.**
2. **No runtime promotion without Q-2 and Q-3 passing.**
3. **No capability registry promotion without Q-4 passing.**
4. **No major version bump without Q-5 identifying all affected components.**
5. **The qualification harness must be run on every CI run for the contracts crate.**

---

## 6. Contract Authority

The canonical authority for each aspect of the platform specification:

| Aspect | Authority | Location |
|--------|-----------|----------|
| Wire format | JSON Schema | `docs/schemas/mcp-*.schema.json` |
| Canonical type system | Rust types | `librarian-contracts/src/` |
| Semantic specification | Contract docs | `docs/contracts/CAPABILITY-REGISTRY-MCP-CONTRACT.md` |
| Qualification tests | Test harness | `librarian-qualification/` (future) |

No runtime implementation may define or modify these artifacts. Runtimes
only implement what the specification defines.

---

## 7. Relationship to Transports

The platform specification is transport-independent. The following transports
are recognized:

| Transport | Status | Adapter Location |
|-----------|--------|-----------------|
| MCP (JSON-RPC) | Active | Swift: `Controllers/MCP/` |
| HTTP/REST | Future | Not yet implemented |
| CLI | Future | Not yet implemented |
| gRPC | Future | Not yet implemented |

A transport adapter must:
1. Wrap contract types in the transport envelope (e.g., JSON-RPC for MCP).
2. Satisfy Q-2 (serialization equivalence) for all contract types it carries.
3. Report its transport type in evidence receipts for attribution.

---

## 8. Qualification Enforcement

The qualification harness (`librarian-qualification`) enforces these rules
programmatically. It is the gatekeeper for:

- **CI pipelines** — Every PR against `librarian-contracts` runs Q-1 and Q-2.
- **Runtime releases** — Every runtime release runs Q-3 and Q-4.
- **Contract evolution** — Every contract change runs Q-5.

The harness produces a qualification report:

```
Contract: capability_registry::load::LoadRequest
  Q-1 Structural:      PASS
  Q-2 Representational: PASS
  Q-3 Behavioral:      PASS (Swift), NOT_APPLICABLE (Rust runtime pending)
  Q-4 Deterministic:   NOT_APPLICABLE (no runtime connected)
  Q-5 Evolution:       NOT_APPLICABLE (first version)

Status: QUALIFIED for Swift runtime
```

---

## 9. Future: Single Source of Truth

The current architecture uses three parallel artifacts (JSON Schema, Rust types,
Markdown docs). The long-term direction is a single canonical definition that
generates the others:

```
Canonical Contract Definition
        │
        ├── Generate Rust types
        ├── Generate JSON Schema
        ├── Generate Swift types
        ├── Generate TypeScript types
        └── Generate documentation
```

The qualification harness should be designed to accept generated artifacts
when this capability exists, without changing the test logic.
