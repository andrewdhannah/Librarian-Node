# Multi-Node MCP Document Custody Notes

**Status:** Planning input for canonical Mac Librarian design
**Date:** 2026-07-01
**Sprint:** WIN-MULTIPLATFORM-LIBRARIAN-PLANNING-1
**Author:** Windows PC lane

---

## 1. Purpose

Define the Windows-side requirements for MCP-based document custody in a multi-node Librarian system. These notes describe how agents, workers, and tools interact with canonical project documents through MCP without risking concurrent-write corruption, split-brain ledger state, or unauthorized canonical mutations.

This document is an **input** to the canonical MCP tool contract design. It does not define the final contracts — it records Windows-side requirements that the canonical contracts must satisfy.

### Key Rule

> Transport route is not authority route. MCP carries requests. The Librarian authority decides what is accepted.

MCP is a transport layer. Agents may submit proposals, evidence, and receipts through MCP tools. The Librarian authority validates, accepts, rejects, or queues those submissions. MCP does not grant write access to canonical state.

---

## 2. Core Constraints

### 2.1 Agents Must Not Directly Edit Canonical Docs Concurrently

Two agents working on the same project must not produce conflicting canonical document mutations. The architecture prevents this by:

- Requiring all canonical mutations to go through the active authority
- Providing lock/lease mechanisms for exclusive write access
- Using a proposal-and-apply model instead of direct file writes
- Detecting stale HEAD before applying any change

### 2.2 MCP Must Expose Proposal/Intake Tools, Not Generic File Write Tools

MCP tools exposed to agents should reflect the proposal-and-apply model:

- Agents call `project_proposal_submit` to propose a change
- Agents call `project_evidence_submit` to return evidence
- Agents call `project_receipt_submit` to return action receipts
- Agents do **not** call `file_write` or `file_overwrite` on canonical paths

Generic file-write MCP tools (if any) must be scoped to non-canonical working directories only.

### 2.3 Canonical Files Require Lock/Lease or Proposal Application

A canonical file may be modified through exactly one of two paths:

1. **Lock/lease path:** An agent or node acquires an exclusive lock on a specific document, modifies it, and releases the lock. The authority tracks lock state.
2. **Proposal path:** An agent submits a proposal packet describing the intended change. The authority validates it and, if accepted, applies the change atomically.

Both paths are mediated by the authority. Neither path permits direct concurrent writes.

### 2.4 Worker Nodes Submit Plans, Patches, Receipts, and Evidence

Workers interact with the authority through a defined submission model:

| Submission Type | Contents | Authority Action |
|----------------|----------|-----------------|
| Proposal packet | Intended change description, diff/patch, rationale | Validate → Accept/Reject/Request revision |
| Evidence packet | Test results, model output, verification data | Validate → Store to evidence directory → Link to work order |
| Receipt packet | Action receipt, execution log, exit codes | Validate → Append to receipt chain |
| Patch/draft | File diff or replacement content | Authority applies if proposal accepted |

### 2.5 Active Librarian Authority Validates, Merges, Rejects, or Asks Owner

The authority is the sole decision-maker for canonical state changes:

- **Validate:** Check that the proposal is well-formed, permitted, and does not conflict with current state
- **Merge:** Apply the change if it integrates cleanly with current canonical state
- **Reject:** Deny the change with a structured reason (conflict, not permitted, stale base, etc.)
- **Ask Owner:** Escalate to the human Owner when the change requires approval (e.g., crosses a risk threshold)

### 2.6 Ledger Writes Require Exclusive Authority

The sprint ledger is the single source of truth for project progress. Only the active authority may write to it. This prevents split-brain scenarios where two nodes each believe they hold authority and produce conflicting ledger entries.

### 2.7 Stale HEAD Detection Required

Before applying any proposal or lock release that modifies canonical state, the authority must verify that the proposer's view of the current state is not stale:

- Compare the proposer's `base_commit` against the current canonical HEAD
- If the proposer is behind: reject with `STALE_BASE` status and include the current HEAD
- The proposer must rebase/refresh and resubmit

### 2.8 Conflict Responses Required

When a proposal conflicts with the current canonical state:

| Conflict Type | Response |
|---------------|----------|
| Lock held by another node | Reject with `LOCK_HELD`, include holder identity |
| Stale base commit | Reject with `STALE_BASE`, include current HEAD |
| Semantic merge conflict | Reject with `MERGE_CONFLICT`, include conflict description |
| Authority changed | Reject with `AUTHORITY_CHANGED`, include new authority identity |
| Proposal duplicates existing | Reject with `DUPLICATE`, reference existing proposal ID |
| Proposal violates schema | Reject with `SCHEMA_VIOLATION`, include validation errors |

---

## 3. Proposed MCP Tools

The following MCP tools are proposed for the multi-node custody model. These are **not** the final canonical contracts — they are the Windows-side requirements that the canonical contracts must satisfy.

### 3.1 State Inspection

| Tool | Description | Returns |
|------|-------------|---------|
| `project_state_snapshot_get` | Returns the current canonical project state: HEAD commit, active authority, locked files, pending proposals, ledger summary | JSON state object |
| `project_ledger_status` | Returns the current sprint ledger summary | Latest sealed sprint, next authorized sprint, recent entries |

### 3.2 Document Lock Management

| Tool | Description | Returns |
|------|-------------|---------|
| `project_document_lock_acquire` | Request an exclusive lock on a specific canonical document path | Lock token or rejection reason |
| `project_document_lock_release` | Release a previously acquired lock | Success or error |
| `project_document_lock_status` | Query lock state for one or more document paths | Lock holder, acquired time, expiry if any |

**Lock semantics:**
- Locks are per-document-path, not per-agent
- A lock prevents other nodes from acquiring a lock or submitting proposals for that path
- Locks should have configurable timeouts to prevent abandoned locks
- The authority may revoke stale locks

### 3.3 Proposal and Evidence Submission

| Tool | Description | Returns |
|------|-------------|---------|
| `project_proposal_submit` | Submit a proposal packet describing an intended change | Proposal ID, initial status (pending/accepted/rejected) |
| `project_proposal_status` | Query the status of a previously submitted proposal | Current status, authority notes, conflict details if any |
| `project_proposal_apply` | Request that an accepted proposal be applied to canonical state | Apply result or error |
| `project_evidence_submit` | Submit evidence packet (test results, model output, etc.) | Evidence receipt ID |
| `project_receipt_submit` | Submit an action receipt for inclusion in the receipt chain | Receipt chain update or error |

### 3.4 Node Registration

| Tool | Description | Returns |
|------|-------------|---------|
| `project_node_register` | Register this node as a worker, client, runtime, or verifier | Registration ID, assigned role, capabilities acknowledged |
| `project_node_capabilities_get` | Query what capabilities this node is authorized to use | List of authorized tools, model profiles, action types |

---

## 4. Document Intake Model

### 4.1 Proposal Packet Structure (Required Fields)

```
proposal_id:          unique identifier
proposer_node_id:     registering node identity
proposer_role:        worker | client
target_document:      canonical document path
base_commit:          proposer's view of current HEAD
change_type:          create | update | delete
change_content:       diff, patch, or replacement text
rationale:            why this change is needed
evidence_refs:        optional list of evidence receipt IDs
risk_classification:  low | medium | high
submitted_at:         timestamp
```

### 4.2 Evidence Packet Structure (Required Fields)

```
evidence_id:          unique identifier
work_order_id:        originating work order or proposal
producer_node_id:     node that produced the evidence
evidence_type:        test_result | model_output | verification | measurement
content:              evidence payload (JSON, text, or reference)
tool_version:         version info of producing tool
submitted_at:         timestamp
```

### 4.3 Lock/Lease Metadata

```
lock_token:           unique lock identifier
document_path:        locked canonical path
holder_node_id:       node holding the lock
acquired_at:          timestamp
expires_at:           optional timeout
lock_type:            exclusive | advisory
status:               active | expired | revoked | released
```

### 4.4 Acceptance / Rejection / Conflict Statuses

| Status | Meaning |
|--------|---------|
| `pending` | Proposal received, awaiting authority review |
| `accepted` | Authority validated and accepted proposal |
| `rejected_locked` | Target document is locked by another node |
| `rejected_stale` | Base commit is behind current canonical HEAD |
| `rejected_conflict` | Semantic merge conflict detected |
| `rejected_schema` | Proposal fails schema validation |
| `rejected_authority` | Authority has changed since proposal was submitted |
| `rejected_permission` | Proposer lacks permission for this change |
| `escalated` | Authority has escalated to Owner for decision |
| `applied` | Proposal has been applied to canonical state |
| `superseded` | Another proposal has been applied instead |

---

## 5. Transport vs. Authority Boundary

```
                    ┌─────────────────────────────────────┐
                    │        Librarian Authority            │
                    │  Validates, applies, rejects,         │
                    │  maintains ledger, resolves conflicts │
                    └──────────┬──────────────────────────┘
                               │ internal protocol
                               │ (not MCP — authority-internal)
                               ▼
                    ┌─────────────────────────────────────┐
                    │        MCP Router / Bridge           │
                    │  Exposes tool contracts to agents    │
                    │  Routes requests to authority        │
                    └──────────┬──────────────────────────┘
                               │ MCP transport
                               │ (JSON-RPC over stdio/HTTP)
                               ▼
               ┌────────────────┴────────────────┐
               │                                 │
          ┌─────────┐                      ┌──────────┐
          │  Agent  │                      │  Worker   │
          │ (MCP)   │                      │ (Windows) │
          └─────────┘                      └──────────┘
```

**Key distinction:** The MCP tool layer is a transport facade. It does not hold authority. An agent calling `project_proposal_submit` is making a request to the authority through a transport channel. The authority processes that request according to its own validation and decision logic.

---

## 6. Windows-Specific Constraints

| # | Constraint | Implication for MCP Custody |
|---|------------|-----------------------------|
| MCP-W01 | PC runs PowerShell 5.1 (no pwsh) | MCP tool examples and reference implementations should include PS 5.1-compatible versions where relevant |
| MCP-W02 | PC is currently worker/runtime, not authority | Initial MCP tool testing should focus on proposal submission, evidence return, and receipt submission from worker nodes |
| MCP-W03 | PC has no Swift toolchain | MCP tool contracts must be defined in language-neutral terms (JSON Schema, Markdown); Windows-side reference implementations must be in Python or PowerShell |
| MCP-W04 | PC uses NSSM service stack | Service lifecycle MCP tools (if any) must account for NSSM wrapping |

---

## 7. Relationship to Mac Canonical Design

This document is a Windows-side input. The Mac canonical MCP document custody design should:

1. Define the formal MCP tool contracts in JSON Schema
2. Define the lock/lease schema and timeout model
3. Define the proposal and evidence packet schemas
4. Define the authority's validation and decision protocol
5. Define the conflict detection and resolution algorithm
6. Define the stale HEAD detection mechanism
7. Produce all schemas in language-neutral JSON/JSON Schema
8. Keep the custody algorithm portable (not macOS-only)

---

## 8. Owner Decisions Required

| Decision | Question |
|----------|----------|
| D-MCP-01 | Approve proposal-and-apply model as the primary document mutation path? |
| D-MCP-02 | Approve lock/lease as an additional mutation path for exclusive edits? |
| D-MCP-03 | Approve the list of proposed MCP tools as a starting point for canonical contracts? |
| D-MCP-04 | Approve the transport-is-not-authority boundary principle? |
