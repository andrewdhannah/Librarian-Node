# Windows Multiplatform Librarian Planning

**Status:** Planning input for canonical Mac Librarian design
**Date:** 2026-07-01
**Sprint:** WIN-MULTIPLATFORM-LIBRARIAN-PLANNING-1
**Author:** Windows PC lane

---

## 1. Purpose

Define the Windows-side view of the eventual multiplatform Librarian architecture. This document records constraints, role definitions, and node model requirements from the Windows PC lane. It is an **input** to the later Mac canonical multiplatform design — not the source of truth for the full architecture.

### Scope

- Role-based node model for a multiplatform Librarian
- Reversible Mac/PC authority assignment
- Relationship between authority, client, worker, runtime, and verifier nodes
- Windows-local constraints that the canonical model must accommodate

### Non-Goals

- Not the canonical multiplatform architecture document
- Does not specify Mac implementation details
- Does not define file formats or schemas (those belong in canonical docs)
- Does not propose specific networking protocols
- Does not specify authentication or transport security

---

## 2. Core Principle: Librarian as Authority Service, Not OS-Bound App

The Librarian is not a macOS application that happens to manage files. It is an **authority service** that may run on any capable host. The operating system running the authority instance is a deployment choice, not an architectural property.

### Key Rule

> Mac may be the first active Librarian authority, but the architecture must treat that as a **role assignment**, not an OS property.

This means:
- Authority logic is portable by design
- No authority capability depends exclusively on macOS APIs
- Role transfer between Mac and PC is architecturally possible, even if not implemented in the first version
- Planning, packet, receipt, and ledger models are language-neutral and OS-neutral

---

## 3. Node Roles

A running Librarian instance occupies exactly one role at a time. Roles may be reassigned over the lifetime of a project.

### 3.1 Authority Node

The **authority node** is the single source of truth for a given project. It:

- Validates and accepts/rejects proposals
- Maintains the canonical sprint ledger
- Applies or rejects document changes
- Authorizes worker registrations
- Resolves conflicts
- Produces the official project receipt chain
- Presents decisions to the human Owner

**Constraints:**
- Exactly one active authority per project at any time
- Authority may be transferred (Mac → PC, PC → Mac, or to any capable node)
- Authority requires: durable storage, audit-capable filesystem, receipt chain integrity, and Owner access

### 3.2 Client Node

A **client node** is a human-facing interface to one or more Librarian authority instances. It:

- Presents project state to the user
- Submits proposals and evidence on behalf of the user or agent
- Displays receipt chains, validation status, and conflict state
- Does not mutate canonical state directly

**Constraints:**
- Multiple clients may attach to one authority
- A client may also be a worker or runtime node (role stacking is per-configuration)
- Client role does not imply write access to canonical state

### 3.3 Worker Node

A **worker node** performs autonomous or semi-autonomous work on behalf of the authority. It:

- Receives work assignments or self-proposes work via proposals
- Executes constrained actions (model inference, code generation, file analysis)
- Returns evidence and receipts to the authority
- Does not modify canonical project state without authority approval

**Constraints:**
- Multiple workers may attach to one authority
- Workers must register with the authority before submitting work
- Worker capabilities are declared at registration (e.g., "has GPU", "can run model X", "has network access")
- Workers are constrained by their declared capability set

### 3.4 Runtime Node

A **runtime node** hosts local model backends and provides advisory compute. It:

- Runs one or more model profiles (phi-4, qwen-coder, etc.)
- Exposes health-checked inference endpoints
- Does not initiate work — it serves inference requests from authorized callers
- Produces per-request receipts for audit

**Constraints:**
- A runtime node may be collocated with a worker, client, or authority
- The same physical machine may host multiple runtime profiles
- Runtime nodes declare available model profiles at registration

### 3.5 Router / Bridge Node

A **router or bridge node** translates between transport protocols and the Librarian authority. It:

- Exposes MCP endpoints for agent tool access
- Routes requests to the appropriate authority or worker
- Does not possess authority itself
- Implements transport-level security and rate limiting

**Constraints:**
- A router node is always subordinate to an authority
- Multiple router nodes may front one authority
- Transport route is not authority route (see custody notes)

### 3.6 Verifier Node

A **verifier node** independently validates receipts, proposals, or state assertions. It:

- Checks receipt integrity (hashes, chains, signatures)
- Validates work packet conformance to schemas
- Does not modify project state
- Produces verification receipts

**Constraints:**
- A verifier may run on any node
- Verification results are advisory to the authority
- The authority may weight or ignore verification results

### 3.7 Receipt Producer

Any node that performs an auditable action is a **receipt producer** for that action. Receipt production is a capability, not a standalone role. Every node role above should produce receipts for actions within its scope.

---

## 4. Role Assignment and Revocation

| Property | Behavior |
|----------|----------|
| Assignment | A node is assigned a role via configuration or authority registration protocol |
| Duration | Roles persist until explicitly reassigned or revoked |
| Revocation | The current authority may revoke any subordinate node's role |
| Authority transfer | Transfer requires: (a) current authority is reachable, (b) successor node is capable, (c) Owner approves, (d) ledger records the transfer |
| Split-brain prevention | Only one active authority per project. If the current authority goes offline, workers and clients enter a "no authority" state. A new authority may be promoted only with Owner approval and conflict resolution. |

---

## 5. Single Active Authority per Project

The architecture enforces exactly one active authority node per project to prevent split-brain ledger writes.

**Rule:** The sprint ledger, receipt chain, and canonical document state are all owned by exactly one authority instance. A node that is not the current authority must not write to canonical state.

**Transfer protocol outline:**
1. Current authority acknowledges transfer intent
2. Successor node demonstrates capability
3. Owner approves
4. Ledger records the authority transfer event
5. Successor takes over; predecessor revokes its own authority
6. All nodes are notified (eventual consistency)

---

## 6. Multiple Clients and Workers per Authority

The authority supports multiple attached nodes simultaneously:

```
┌─────────────────────────────────────────────┐
│           Librarian Authority                │
│  (Mac or PC — role-based, not OS-bound)      │
└──────┬──────────┬──────────┬────────────────┘
       │          │          │
       ▼          ▼          ▼
   ┌────────┐ ┌────────┐ ┌──────────┐
   │ Client │ │ Worker │ │  Router  │
   │ (Mac)  │ │  (PC)  │ │  (MCP)   │
   └────────┘ └────────┘ └────┬─────┘
                              │
                         ┌────┴─────┐
                         │  Runtime  │
                         │   (PC)   │
                         └──────────┘
```

The same physical machine may run multiple roles. For example, a Windows PC today runs:
- Runtime node (model inference on RX 570)
- Worker node (harness scripts, packet validation)
- Client node (future Windows Librarian app)

Tomorrow that same PC could run:
- Authority node (if Mac is offline or role is transferred)
- Router/bridge node (MCP gateway)

---

## 7. Reversible Mac/PC Roles

| Scenario | Mac Role | PC Role |
|----------|----------|---------|
| Today (Phase 1) | Authority + Client | Worker + Runtime |
| Near future | Authority + Client | Worker + Runtime + Verifier |
| Mid future | Authority + Client + Router | Worker + Runtime + Authority (backup) |
| Future | Client only | Authority + Router + Worker + Runtime |
| Future | Worker + Runtime | Authority + Client |

The architecture does not hardcode Mac as permanent source of truth. Mac may be the first authority because it has the Swift IDE and the canonical repo. But the Windows PC must be able to assume authority later without an architecture rewrite.

---

## 8. Windows-Specific Constraints for the Canonical Model

These are Windows-local conditions that the multiplatform architecture must accommodate:

| # | Constraint | Implication for Canonical Model |
|---|------------|---------------------------------|
| W-01 | PowerShell 5.1 only (no pwsh) | All cross-platform scripts must work with PS 5.1 **or** a portable runtime |
| W-02 | No Swift toolchain | Authority logic on Windows cannot depend on Swift; must be portable language |
| W-03 | No .NET SDK installed | .NET-based governance tooling blocked without C: space reclamation |
| W-04 | C: drive critically low (14.9 GB free) | Large SDK installs (Visual Studio, .NET, Android NDK) are blocked |
| W-05 | Non-admin shell | Service-state operations require elevation on Windows |
| W-06 | NSSM-based service stack | Service lifecycle abstraction must account for NSSM + PowerShell + Python chain |
| W-07 | Windows 10 22H2 (past EOS) | Operational risk for long-running authority service |
| W-08 | RX 570 4 GB GPU (Vulkan) | Model host capability is limited; relevant for worker/runtime registration |

---

## 9. Relationship to Mac Canonical Design

This document is a Windows-side input. The Mac canonical multiplatform design should:

1. Define the formal node role schema and transfer protocol
2. Define the canonical role assignment/revocation mechanism
3. Define the MCP tool contracts for multi-node operations
4. Define the authority-transfer receipt schema
5. Produce schemas in language-neutral JSON/JSON Schema
6. Keep the portable core separate from macOS-specific UI code

---

## 10. Owner Decisions Required

| Decision | Question |
|----------|----------|
| D-MP-01 | Approve role-based node model as the multiplatform architecture starting point? |
| D-MP-02 | Approve "single active authority per project" rule? |
| D-MP-03 | Approve reversible Mac/PC roles as an architectural requirement? |
| D-MP-04 | Approve node registration via MCP as the intended mechanism? |

---

## Appendix A: Node Role Quick Reference

| Role | Writes Canonical State? | Requires Owner? | Requires Durable Storage? | Portability |
|------|------------------------|-----------------|--------------------------|-------------|
| Authority | Yes | Yes (for decisions) | Yes | Portable service |
| Client | No | No | No | UI framework per platform |
| Worker | No (proposes only) | No (constrained) | No (ephemeral ok) | Script/agent language |
| Runtime | No (serves inference) | No | No | Server binary per platform |
| Router/Bridge | No | No | No | Portable server |
| Verifier | No | No | No | Portable algorithm |
