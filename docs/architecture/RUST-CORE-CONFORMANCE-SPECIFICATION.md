# Rust Core Conformance Specification

**Version:** 1.0.0
**Status:** Baseline (frozen)
**Date:** 2026-08-06
**Author:** Librarian Project

---

## 1. Purpose

This specification defines how the Rust runtime is validated against the governed Librarian system. It exists so that every Rust module has exactly one question to answer:

> **"Does this conform to the specification?"**

...rather than:

> "Is this a good Rust design?"

Rust is an **implementation language**, not a place where architectural decisions drift from the governed design.

---

## 2. Position

The migration is a **conformance program**, not a port.

```
Swift Implementation (Reference Behavior)
        │
        │  observable behavior
        ▼
Platform Equivalence (Conformance Harness)
        │
        │  equivalence validation
        ▼
Rust Runtime (Canonical Execution Engine)
```

- **Swift** (TheLibrarian) = canonical implementation and reference behavior
- **Contracts** = canonical intent and invariants
- **Qualification receipts** = canonical evidence of conformance
- **Rust** = new implementation that must demonstrate behavioral equivalence

---

## 3. Baseline Freeze

Three concurrent references are frozen. If the system evolves after the port begins, these define exactly which version of behavior the Rust runtime targets. This prevents "moving target" migrations.

| Reference | Role | Repository | Frozen Commit |
|-----------|------|------------|---------------|
| Git baseline | Canonical implementation | `andrewdhannah/TheLibrarian` (sprint-3, private) | `15c5ef2c1d4d4ba72af2a765f0bc4ce8bd4b16cc` |
| Rust runtime | Implementation under conformance | `andrewdhannah/Librarian-Node` | `da3dfd4` |
| Equivalence harness | Conformance and migration harness | `andrewdhannah/Librarian-Platform-Equivalence` | `e42a6c6` |

**Baseline policy:** The Swift baseline is frozen. Rust targets `15c5ef2` behavior. Any change to the Swift baseline after this freeze requires a new baseline freeze and an equivalence gap analysis.

---

## 4. Equivalence Invariant

> **For a given input, the Rust runtime must produce equivalent observable behavior to the qualified Swift implementation.**

Observable behavior includes:

| Dimension | Conformance Check |
|-----------|-------------------|
| Startup sequence | Same phases, same order, same pass/fail outcomes |
| Registry state | Same registered entities, same state transitions |
| SQLite schema | Identical schema, migrations, data shapes |
| Receipt generation | Receipts conform to same schema and content rules |
| Evidence chains | Same chain structure, same immutability guarantees |
| Qualification outcomes | Same qualification decisions for same inputs |
| API responses | Same response structure and semantics |
| Error semantics | Same intentional error behavior |

**Equivalence is semantic, not bit-identical.** Deterministic fields must match exactly; timestamps and platform-specific identifiers may vary (see `docs/architecture/THREE-WAY-EQUIVALENCE-PROTOCOL.md`).

---

## 5. Conformance Levels

The Rust runtime is assessed against the sealed conformance levels defined in the canonical repository:

| Level | Name | Requirement |
|-------|------|-------------|
| C0 | Unassessed | Default; no conformance assessment performed |
| C1 | Contract Compatible | Contracts follow specification structure |
| C2 | Evidence Compatible | Evidence follows receipt format |
| C3 | Qualification Compatible | Qualifications follow protocol |
| C4 | Federation Compatible | Can participate in multi-organization governance |

Canonical sources:
- `TheLibrarian/conformance/CONFORMANCE-LEVELS-v1.json` (SEALED)
- `TheLibrarian/conformance/GOVERNANCE-CONFORMANCE-SPECIFICATION-v1.md`
- `TheLibrarian/conformance/conformance-suite-v1.json`

Test IDs from the sealed suite (e.g., `CONTRACT-SCHEMA-001`, `RECEIPT-SCHEMA-001`, `QUALIFICATION-SCHEMA-001`) are authoritative. Rust tests must map to these IDs; new Rust-specific test IDs use the `RUST-` prefix.

---

## 6. Subsystem Conformance Template

Every Rust subsystem must document, before implementation:

| Field | Definition |
|-------|------------|
| Contract source | Canonical spec or contract file(s) the subsystem implements |
| Expected inputs | Exact input shape the subsystem accepts |
| Expected outputs | Exact output shape the subsystem produces |
| Receipt format | Receipt schema the subsystem must emit |
| Qualification tests | Test IDs from the conformance suite that must pass |
| Golden fixtures | Canonical fixtures the subsystem must reproduce |
| Performance expectations | Measured bounds the subsystem must satisfy |

**Template file:** `conformance/subsystems/<name>.md` (in this repository).

---

## 7. Subsystem Registry (Dependency Order)

| # | Subsystem | Contract Source | Status |
|---|-----------|-----------------|--------|
| 1 | Core domain types (identity, receipts, evidence, custody, contracts) | `librarian-contracts` crate + canonical contracts | ⏳ Pending |
| 2 | SQLite persistence (GRDB concepts → sqlx/rusqlite), migrations | Canonical schema + migration history | ⏳ Pending |
| 3 | Registry | Canonical registry contracts | ⏳ Pending |
| 4 | Startup engine (fast path, validation, health checks) | `contracts/startup/STARTUP-PROTOCOL.md` (this repo) | ⏳ Pending |
| 5 | Qualification harness (test corpus, receipts, golden outputs) | `conformance/` suite + existing receipts | ⏳ Pending |
| 6 | Runtime services (scheduler, work compiler, node execution) | Canonical runtime contracts | ⏳ Pending |
| 7 | API layer (HTTP, IPC, MCP) | Canonical API contracts | ⏳ Pending |
| 8 | UI adapters | Canonical UI contracts | ⏳ Pending |

Each subsystem starts at C0 and must reach at least C2 (evidence compatible) before being considered implemented; C3 where qualification applies.

---

## 8. Milestone 0 — Deterministic Startup Engine

The first Rust milestone is a minimal executable capable of:

```
Rust Runtime
Load Project
    ↓
Open SQLite
    ↓
Load Registry
    ↓
Load Contracts
    ↓
Execute Startup Protocol
    ↓
Expose Runtime API
    ↓
Return Receipts
```

**Constraints (explicit non-goals for Milestone 0):**
- No UI
- No AI
- No MCP
- No networking
- No concurrency beyond what the startup sequence requires
- Just deterministic startup

**Acceptance gates (RUST-M0):**

| Gate | Verification |
|------|--------------|
| RUST-M0-1 | Loads a project directory and resolves canonical contract sources |
| RUST-M0-2 | Opens/creates SQLite database with schema conforming to canonical schema |
| RUST-M0-3 | Loads registry state from database |
| RUST-M0-4 | Loads contracts from canonical sources |
| RUST-M0-5 | Executes the 6-phase startup protocol per `STARTUP-PROTOCOL.md` |
| RUST-M0-6 | Produces a startup receipt conforming to `schemas/startup-receipt.schema.json` |
| RUST-M0-7 | Startup receipt passes equivalence check against Swift reference receipt |
| RUST-M0-8 | Same input produces same receipt across repeated runs (determinism) |

**Milestone 0 evidence:** receipts under `evidence/phase0/rust-core/m0/` compared against canonical receipts from the Swift baseline.

---

## 9. Divergence Protocol

When the Rust implementation differs from the Swift baseline, determine which case applies:

| Case | Determination | Action |
|------|---------------|--------|
| Rust bug | Rust behavior differs from contract and Swift | Fix Rust; add regression test |
| Swift bug | Swift behavior differs from contract and Rust | Record finding; do NOT port the bug |
| Contract clarification | Neither matches; contract ambiguous | File contract clarification; update contract; re-baseline |

Every divergence must be recorded with: input, expected (contract), Swift behavior, Rust behavior, and disposition.

---

## 10. Workflow

1. Freeze a specific Git commit as the baseline (see §3).
2. Identify subsystem boundaries (startup, registry, custody, receipts, scheduler, ...).
3. Reimplement **one** subsystem in Rust.
4. Run the **same** qualification tests and compare:
   - outputs
   - receipts
   - database state
   - API behavior
5. Repeat until the Rust runtime reaches feature parity.

The Swift code serves as an **oracle**. The qualification framework shifts the question from "Does the Rust code look correct?" to "Does it produce the same governed behavior?"

---

## 11. Evidence

- All Rust conformance evidence is append-only, under `evidence/phase0/rust-core/`.
- Evidence receipts conform to the canonical receipt formats.
- Qualification outcomes are recorded against the sealed conformance suite test IDs.

---

## 12. References

- `docs/architecture/NODE-REFERENCE-ARCHITECTURE.md` — node specification
- `docs/architecture/PLATFORM-ADAPTER-BOUNDARY.md` — contract/adapter boundary
- `docs/architecture/THREE-WAY-EQUIVALENCE-PROTOCOL.md` — cross-platform equivalence
- `contracts/startup/STARTUP-PROTOCOL.md` — startup protocol (Milestone 0 target)
- `schemas/startup-receipt.schema.json` — startup receipt schema
- `TheLibrarian@15c5ef2/conformance/CONFORMANCE-LEVELS-v1.json` — sealed conformance levels
- `TheLibrarian@15c5ef2/conformance/GOVERNANCE-CONFORMANCE-SPECIFICATION-v1.md` — conformance framework
- `TheLibrarian@15c5ef2/conformance/conformance-suite-v1.json` — conformance suite
