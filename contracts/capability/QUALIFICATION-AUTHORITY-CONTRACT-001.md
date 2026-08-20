# QUALIFICATION-AUTHORITY-CONTRACT-001.md — Qualification Authority Contract

**Version:** 1.0.0
**Status:** DRAFT (pending owner review)
**Sealed suite ID:** `QUALIFICATION-AUTHORITY-SCHEMA-001`
**Last Updated:** 2026-08-07

**Semantic parent:** `QUALIFICATION-TRANSITION-CONTRACT-001` (DRAFT — transition authority is prerequisite)
**Gate:** RUST-MIGRATION-M1 gate M1-4 (qualification semantics)

---

## 0. Purpose

Defines the **authorization boundaries** for qualification transitions. This contract answers: **Who is permitted to cause transitions, and how is that permission verified?**

Authority is the right to cause state transitions. Without authority, a transition is unauthorized regardless of evidence. The authority contract ensures that transitions are intentional, auditable, and bounded.

---

## 1. Core Invariant

```
Authority is explicit, not implicit.
Authority is verified, not assumed.
Authority is bounded, not global.
Authority does not accumulate.
```

---

## 2. Authority Roles

### 2.1 Role Definitions

| Role | Description | Authority Scope |
|------|-------------|-----------------|
| `owner` | Human authority with full governance rights | All transitions |
| `evaluator` | Human or system with evaluation authority | `unreviewed → reviewed`, `reviewed → qualified` (with owner pre-authorization) |
| `system` | Automated system with pre-authorized policy | `qualified → deprecated`, `qualified → revoked` (with owner notification) |
| `approver` | Human with approval authority | `reviewed → qualified` (with owner pre-authorization) |

### 2.2 Role Hierarchy

```
owner
  │
  ├── evaluator (owner-delegated)
  │
  └── approver (owner-delegated)
  
system (independent, pre-authorized)
```

### 2.3 Role Boundaries

| Role | Can Do | Cannot Do |
|------|--------|-----------|
| `owner` | All transitions, all authority | — |
| `evaluator` | Evaluate capabilities, recommend transitions | Authorize own transitions (requires `approver`) |
| `approver` | Approve transitions recommended by `evaluator` | Evaluate capabilities (requires `evaluator`) |
| `system` | Execute pre-authorized policy transitions | Initiate manual transitions |

---

## 3. Authority Verification

### 3.1 Manual Transitions

For manual transitions (`unreviewed → reviewed`, `reviewed → qualified`, `qualified → deprecated`, `qualified → revoked`):

| Requirement | Description |
|-------------|-------------|
| Identity | Transitioner identity must be authenticated |
| Authorization | Transitioner must have the required role |
| Evidence | Authority evidence must exist (signature, approval record) |
| Audit | Transition event must be recorded |

### 3.2 Automated Transitions

For automated transitions (`qualified → deprecated` by policy expiry, `qualified → revoked` by security policy):

| Requirement | Description |
|-------------|-------------|
| Policy | Pre-authorized policy must exist |
| Policy scope | Policy must cover the specific capability/transition |
| Evidence | Policy evidence must exist |
| Notification | Owner must be notified |
| Audit | Transition event must be recorded |

### 3.3 Authority Evidence

Every transition must include authority evidence:

| Transition Type | Evidence Required |
|-----------------|-------------------|
| Manual (owner) | Owner identity + timestamp |
| Manual (delegated) | Delegation record + delegate identity + timestamp |
| Automated (policy) | Policy ID + policy scope + evaluation result |
| Emergency | Emergency authority record + owner notification |

### 3.4 Authority Revocation

Authority can be revoked:

| Revocation | Effect |
|------------|--------|
| Role revocation | Role holder can no longer cause transitions |
| Policy revocation | Policy no longer authorizes transitions |
| Delegation revocation | Delegate can no longer act on behalf of owner |

Revocation is **immediate** and **auditable**. Transitions in progress at revocation time are blocked.

---

## 4. Authority Separation

### 4.1 Separation Requirements

The following roles must not be the same entity for a single transition:

| Pair | Reason |
|------|--------|
| Executor ≠ Authority | Self-authorization is forbidden |
| Evidence provider ≠ Auditor | Self-verification is forbidden |
| Evaluator ≠ Approver (for same transition) | Separation of duties |

### 4.2 Separation Enforcement

Separation is enforced by:

1. Role verification at transition time
2. Identity verification (not role assumption)
3. Audit trail (separation violations are detectable)

### 4.3 Separation Exceptions

Separation may be relaxed only for:

- `owner` role (owner has all authorities)
- Emergency transitions (with explicit audit trail)

---

## 5. Authority Source

### 5.1 Source Identification

The authority source for each transition must be identified:

| Transition | Authority Source |
|------------|------------------|
| `unreviewed → reviewed` | Owner or delegated evaluator |
| `reviewed → qualified` | Owner or delegated approver + evaluator |
| `qualified → deprecated` | Owner or pre-authorized policy |
| `qualified → revoked` | Owner or pre-authorized security policy |
| `deprecated → revoked` | Owner or pre-authorized policy |

### 5.2 Source Boundaries

The authority source must NOT be:

- The Rust runtime (runtime executes, does not authorize)
- The qualification record itself (record is outcome, not authority)
- The evidence record (evidence is proof, not authority)
- The transport adapter (adapter consumes, does not authorize)

### 5.3 Source Audit

Every transition must record the authority source:

- Who/what authorized the transition
- What authority was exercised
- What evidence supports the authorization

---

## 6. Relationship to Other Contracts

### 6.1 Authority ↔ Transition

Authority is a **prerequisite** for transitions. No authority → no transition, regardless of evidence.

### 6.2 Authority ↔ Evidence

Authority and evidence are **independent**:

- Authority: "Who is permitted to cause this transition?"
- Evidence: "What proof exists that qualification occurred?"

Both are required; neither substitutes for the other.

### 6.3 Authority ↔ Policy

Policy can pre-authorize transitions, but policy is not authority itself:

- Policy defines what transitions are permitted under what conditions
- Authority is the right to exercise policy-authorized transitions
- Policy revocation removes the pre-authorization
- Authority revocation removes the right to exercise

---

## 7. Explicit Exclusions

| Item | Status | Note |
|------|--------|------|
| Policy evaluation | Out of scope | Policy context (separate) |
| Evidence evaluation | Out of scope | Evidence contract (separate) |
| Permission granting | Out of scope | Authorization contract (separate) |
| Operational-mode derivation | Out of scope | F-3 (separate) |
| Registry mutation | Out of scope | Observation ≠ mutation |

---

## 8. Authorization

This contract defines the authority boundary. It must be locked before any qualification implementation begins.

M1-D0 lock establishes this contract surface. M1-D1 (qualification types) may proceed after this lock.
