# M1-D0 — Qualification Semantics Contract Lock

**Receipt ID:** M1-D0-LOCK-20260807
**Date:** 2026-08-07
**Repository:** Librarian-Node
**Epic:** EPIC-RUST-MIGRATION-1
**Predecessor:** M1-D planning `5bb46b0`

---

## 0. Purpose

Locks the qualification semantics contract surface. This artifact answers the 12 review criteria from M1-D planning and establishes four separate contracts before any implementation begins.

**M1-D0 non-goal:** Implementation code. This is a contract-lock artifact only.

---

## 1. Review Criteria Answers

### 1.1 Who may cause a qualification transition?

**Answer:** Authority is explicit, not implicit.

| Transition | Authority Required |
|------------|--------------------|
| `unreviewed → reviewed` | Owner or delegated evaluator |
| `reviewed → qualified` | Owner or delegated approver + evaluator |
| `qualified → deprecated` | Owner or pre-authorized policy |
| `qualified → revoked` | Owner or pre-authorized security policy |
| `deprecated → revoked` | Owner or pre-authorized policy |

**Authority boundary:** `QUALIFICATION-AUTHORITY-CONTRACT-001.md`

### 1.2 What makes a qualification record valid?

**Answer:** Profile + record + evidence reference.

A qualification record is valid when:
- It references a valid qualification profile
- It contains all required fields
- It references valid evidence for all required dimensions

**Evidence boundary:** `QUALIFICATION-EVIDENCE-CONTRACT-001.md`

### 1.3 What evidence is mandatory?

**Answer:** Contract-defined, not implementation-defined.

Five mandatory dimensions:

| Dimension | Required For |
|-----------|--------------|
| `identity` | All qualifications |
| `capability` | All qualifications |
| `security_level` | S2+ capabilities |
| `qualification` | All qualifications |
| `constraints` | Policy-constrained capabilities |

**Evidence boundary:** `QUALIFICATION-EVIDENCE-CONTRACT-001.md`

### 1.4 Does qualification grant permission?

**Answer:** No.

```
QUALIFIED ≠ AUTHORIZED
```

Qualification does not grant permission. Permission is a separate authorization decision. Authorization decisions must not mutate qualification records.

**Authority boundary:** `QUALIFICATION-AUTHORITY-CONTRACT-001.md`

### 1.5 Does qualification make a capability executable?

**Answer:** Not by itself.

Execution requires four independent conditions:
1. Qualification (`passed`)
2. Availability (`registered`)
3. Permission (authorization contract)
4. Operational mode (F-3 contract)

Qualification alone is insufficient.

**Availability boundary:** `QUALIFICATION-AVAILABILITY-BOUNDARY-CONTRACT-001.md`

### 1.6 Can policy override qualification?

**Answer:** Explicitly defined, not implicit.

Policy can:
- Pre-authorize transitions (authority contract)
- Define staleness rules (evidence contract)
- Define consistency constraints (availability boundary)

Policy cannot:
- Mutate qualification records directly
- Bypass transition authority
- Override evidence requirements

**Policy boundary:** Policy context is a separate concern; policy changes affect qualification through defined transition mechanisms only.

### 1.7 How is security classification carried?

**Answer:** Preserve provenance; no collapse.

Security classification is **frozen at transition time**. The `security_classification` field in lifecycle events records the classification when the transition occurred, not the current classification. This preserves provenance and prevents retroactive reclassification.

**Transition boundary:** `QUALIFICATION-TRANSITION-CONTRACT-001.md`

### 1.8 Are lifecycle events append-only?

**Answer:** Yes.

Every transition produces an append-only lifecycle event in `qualification_lifecycle_events`. Events are never modified or deleted. The event trail is the audit history.

**Transition boundary:** `QUALIFICATION-TRANSITION-CONTRACT-001.md`

### 1.9 Can a failed transition partially persist?

**Answer:** No ambiguous half-transitions.

Transitions are atomic:
- Succeed completely: state changes, event recorded, evidence linked
- Fail completely: state unchanged, error recorded, no partial effect

Partial persistence is forbidden.

**Transition boundary:** `QUALIFICATION-TRANSITION-CONTRACT-001.md`

### 1.10 What is the relationship to availability?

**Answer:** Explicitly separated.

Qualification and availability are independent axes:

| Property | Qualification | Availability |
|----------|---------------|--------------|
| Controls | Verification state | Access state |
| Transitions | Qualification transitions | Availability transitions |
| Authority | Qualification authority | Availability authority |
| Effect | Determines if capability is verified | Determines if capability is accessible |

They share the same capability record but operate independently. Qualification transitions never affect availability; availability transitions never affect qualification.

**Availability boundary:** `QUALIFICATION-AVAILABILITY-BOUNDARY-CONTRACT-001.md`

### 1.11 What is the relationship to operational mode?

**Answer:** F-3 contract remains separate.

Qualification state is an **input** to operational-mode derivation:

| Qualification State | Operational Mode Effect |
|--------------------|------------------------|
| `passed` | Normal operational mode derivation |
| `failed` | Degraded operational mode |
| `stale` | Degraded operational mode |
| `suspended` | Suspended operational mode |

Operational-mode decisions never modify qualification records. Operational mode is **downstream** of qualification.

**Operational mode boundary:** F-3 contract (separate)

### 1.12 What is the authority source?

**Answer:** Must not accidentally become the Rust runtime.

The authority source for each transition must be identified:

| Transition | Authority Source |
|------------|------------------|
| Manual | Owner identity (authenticated) |
| Delegated | Delegation record + delegate identity |
| Automated | Pre-authorized policy + policy scope |

The Rust runtime **executes** transitions; it does **not authorize** them. Authorization comes from owner identity or pre-authorized policy. The runtime is an executor, not an authority.

**Authority boundary:** `QUALIFICATION-AUTHORITY-CONTRACT-001.md`

---

## 2. Contract Surface

### 2.1 Contracts Locked

| Contract | File | Purpose |
|----------|------|---------|
| Qualification Transition | `QUALIFICATION-TRANSITION-CONTRACT-001.md` | Transition authority and semantics |
| Qualification Evidence | `QUALIFICATION-EVIDENCE-CONTRACT-001.md` | Evidence requirements and validation |
| Qualification Authority | `QUALIFICATION-AUTHORITY-CONTRACT-001.md` | Authorization boundaries |
| Qualification/Availability Boundary | `QUALIFICATION-AVAILABILITY-BOUNDARY-CONTRACT-001.md` | Axis independence and consistency |

### 2.2 Contract Lineage

```
QUALIFICATION-TRANSITION-CONTRACT-001      (transition authority)
        │
        ├── QUALIFICATION-EVIDENCE-CONTRACT-001      (evidence requirements)
        │
        ├── QUALIFICATION-AUTHORITY-CONTRACT-001      (authorization boundaries)
        │
        └── QUALIFICATION-AVAILABILITY-BOUNDARY-CONTRACT-001      (axis independence)
```

### 2.3 Core Invariants (Preserved Across All Contracts)

```
QUALIFIED   ≠ AUTHORIZED    — qualification does not grant permission
QUALIFIED   ≠ AVAILABLE     — qualification does not control availability
QUALIFIED   ≠ EXECUTING     — qualification does not enable execution
```

---

## 3. First-Class Invariants

### 3.1 QUALIFIED ≠ AUTHORIZED

A capability can have a valid qualification record without thereby receiving permission to execute. Authorization decisions must not mutate the underlying qualification record merely because an execution request was permitted.

### 3.2 QUALIFIED ≠ AVAILABLE

A capability can be qualified without being available. Qualification transitions never affect availability. Availability transitions never affect qualification.

### 3.3 QUALIFIED ≠ EXECUTING

A qualified capability is not automatically executable. Execution requires four independent conditions: qualification, availability, permission, and operational mode.

---

## 4. What M1-D Will Demonstrate

| Property | Status | Evidence |
|----------|--------|----------|
| Transition authority | Defined | QUALIFICATION-TRANSITION-CONTRACT-001 |
| Evidence requirements | Defined | QUALIFICATION-EVIDENCE-CONTRACT-001 |
| Authorization boundaries | Defined | QUALIFICATION-AUTHORITY-CONTRACT-001 |
| Axis independence | Defined | QUALIFICATION-AVAILABILITY-BOUNDARY-CONTRACT-001 |
| Lifecycle audit trail | Defined | Append-only events |
| Failure semantics | Defined | Atomic transitions |

## 5. What M1-D Will NOT Demonstrate

| Item | Status | Note |
|------|--------|------|
| Permission granting | Out of scope | Authorization contract (separate) |
| Operational-mode derivation | Out of scope | F-3 (separate) |
| Registry mutation | Out of scope | Observation ≠ mutation |
| Schema evolution | Out of scope | Schema frozen |
| Execution logic | Out of scope | Execution contract (separate) |

---

## 6. Authorization

M1-D0 contract surface is locked. Four contracts established.

M1-D1 (qualification types) may proceed when the Owner authorizes it. No implementation before M1-D0 lock.
