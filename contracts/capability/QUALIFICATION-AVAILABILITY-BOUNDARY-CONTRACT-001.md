# QUALIFICATION-AVAILABILITY-BOUNDARY-CONTRACT-001.md — Qualification/Availability Boundary

**Version:** 1.0.0
**Status:** DRAFT (pending owner review)
**Sealed suite ID:** `QUALIFICATION-AVAILABILITY-BOUNDARY-SCHEMA-001`
**Last Updated:** 2026-08-07

**Semantic parent:** `QUALIFICATION-TRANSITION-CONTRACT-001` (DRAFT — transition authority is prerequisite)
**Gate:** RUST-MIGRATION-M1 gate M1-4 (qualification semantics)

---

## 0. Purpose

Defines the **boundary between qualification and availability**. This contract answers: **How do qualification and availability interact, and what are the consistency constraints?**

Qualification and availability are independent governance axes. They share the same capability record but operate independently. This contract ensures they remain decoupled.

---

## 1. Core Invariants

```
QUALIFIED   ≠ AVAILABLE     — qualification does not control availability
QUALIFIED   ≠ EXECUTING     — qualification does not enable execution
AVAILABLE   ≠ QUALIFIED     — availability does not imply qualification
```

These invariants hold independently. No combination implies any other.

---

## 2. Axis Definitions

### 2.1 Qualification Axis

| Value | Description |
|-------|-------------|
| `not_tested` | No qualification evidence exists |
| `qualifying` | Qualification in progress |
| `passed` | Qualification evidence complete and valid |
| `failed` | Qualification evidence incomplete or invalid |
| `stale` | Qualification evidence expired |
| `suspended` | Qualification suspended (temporary) |

**Governed by:** Qualification transitions (QUALIFICATION-TRANSITION-CONTRACT-001)

### 2.2 Availability Axis

| Value | Description |
|-------|-------------|
| `discovered` | Capability discovered, not yet registered |
| `registered` | Capability registered, available for use |
| `disabled` | Capability temporarily unavailable |
| `removed` | Capability permanently unavailable |

**Governed by:** Availability transitions (separate contract)

### 2.3 Lifecycle State (Status)

| Value | Description |
|-------|-------------|
| `unreviewed` | Not yet reviewed |
| `reviewed` | Reviewed, not yet qualified |
| `qualified` | Qualified |
| `deprecated` | Deprecated |
| `revoked` | Revoked |

**Governed by:** Qualification transitions (QUALIFICATION-TRANSITION-CONTRACT-001)

---

## 3. Consistency Constraints

### 3.1 Permitted Combinations

| Qualification | Availability | Lifecycle | Consistent? |
|---------------|--------------|-----------|-------------|
| `not_tested` | `discovered` | `unreviewed` | Yes |
| `not_tested` | `registered` | `unreviewed` | Yes |
| `passed` | `registered` | `qualified` | Yes |
| `passed` | `disabled` | `qualified` | Yes (qualified but unavailable) |
| `failed` | `registered` | `reviewed` | Yes (failed qualification) |
| `stale` | `registered` | `qualified` | Yes (qualification expired) |
| `suspended` | `disabled` | `qualified` | Yes (suspended) |

### 3.2 Forbidden Combinations

| Qualification | Availability | Lifecycle | Reason |
|---------------|--------------|-----------|--------|
| `not_tested` | `registered` | `qualified` | Cannot be qualified without testing |
| `failed` | `registered` | `qualified` | Cannot be qualified with failed evidence |
| `revoked` | `registered` | `qualified` | Revoked capabilities cannot be qualified |

### 3.3 Consistency Enforcement

Consistency is enforced by:

1. Transition validation (transitions check axis compatibility)
2. State invariants (state machine validates combinations)
3. Audit trail (inconsistencies are detectable)

---

## 4. Axis Independence

### 4.1 Qualification Transitions Do Not Affect Availability

When a qualification transition occurs:

| Transition | Effect on Availability |
|------------|----------------------|
| `not_tested → qualifying` | None |
| `qualifying → passed` | None |
| `qualifying → failed` | None |
| `passed → stale` | None |
| `passed → suspended` | None |
| `suspended → passed` | None |

Availability is **never** modified by qualification transitions.

### 4.2 Availability Transitions Do Not Affect Qualification

When an availability transition occurs:

| Transition | Effect on Qualification |
|------------|------------------------|
| `discovered → registered` | None |
| `registered → disabled` | None |
| `disabled → registered` | None |
| `registered → removed` | None |

Qualification is **never** modified by availability transitions.

### 4.3 Axis Coupling Prohibition

The following couplings are **forbidden**:

| Coupling | Reason |
|----------|--------|
| Qualification transition changes availability | Violates axis independence |
| Availability transition changes qualification | Violates axis independence |
| Qualification state determines availability | Violates axis independence |
| Availability state determines qualification | Violates axis independence |

---

## 5. Relationship to Execution

### 5.1 Qualification Does Not Enable Execution

A qualified capability is not automatically executable. Execution requires:

| Requirement | Source |
|-------------|--------|
| Qualification | Qualification axis = `passed` |
| Availability | Availability axis = `registered` |
| Permission | Authorization contract (separate) |
| Operational mode | F-3 contract (separate) |

All four requirements must be satisfied. Qualification alone is insufficient.

### 5.2 Availability Does Not Imply Qualification

A registered capability is not automatically qualified. Registration only means:

- The capability is known to the system
- The capability is available for use (if qualified)
- The capability has not been removed

Registration does not imply qualification, permission, or operational mode.

### 5.3 Execution Does Not Modify Qualification

When a capability is executed:

| Effect | On Qualification | On Availability |
|--------|------------------|-----------------|
| Successful execution | None | None |
| Failed execution | None | None |
| Execution timeout | None | None |
| Execution error | None | None |

Execution is a **read-only** operation with respect to qualification and availability. Execution never mutates governance axes.

---

## 6. Relationship to Operational Mode

### 6.1 Qualification as Input to Operational Mode

Qualification state is an **input** to operational-mode derivation:

| Qualification State | Operational Mode Effect |
|--------------------|------------------------|
| `passed` | Normal operational mode derivation |
| `failed` | Degraded operational mode |
| `stale` | Degraded operational mode |
| `suspended` | Suspended operational mode |

### 6.2 Operational Mode Does Not Modify Qualification

Operational-mode decisions never modify qualification records:

| Operational Mode Decision | Effect on Qualification |
|--------------------------|------------------------|
| Mode change | None |
| Mode degradation | None |
| Mode restoration | None |

Operational mode is **downstream** of qualification. It reads qualification state; it never writes it.

---

## 7. Explicit Exclusions

| Item | Status | Note |
|------|--------|------|
| Availability transitions | Out of scope | Separate contract |
| Operational-mode derivation | Out of scope | F-3 (separate) |
| Permission granting | Out of scope | Authorization contract (separate) |
| Execution logic | Out of scope | Execution contract (separate) |
| Registry mutation | Out of scope | Observation ≠ mutation |

---

## 8. Authorization

This contract defines the qualification/availability boundary. It must be locked before any qualification implementation begins.

M1-D0 lock establishes this contract surface. M1-D1 (qualification types) may proceed after this lock.
