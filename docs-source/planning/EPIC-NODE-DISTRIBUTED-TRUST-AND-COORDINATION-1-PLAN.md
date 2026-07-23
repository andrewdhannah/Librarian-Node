# EPIC-NODE-DISTRIBUTED-TRUST-AND-COORDINATION-1 — Full Plan

**Status:** Plan sealed — first sprint ready to authorize  
**Preceded By:** Three completed Phase epics (26 sprints)  
**Objective:** Move from internal node maturity to governed distributed participation.

---

## Architecture Context

The node after Phase 3 can identify itself, prove capabilities, maintain custody, execute workloads, generate intelligence, and expose operational state. What it cannot yet do is **prove it is allowed to interact with other nodes or authorities.**

This epic creates the bridge between a capable local runtime and a trusted distributed system.

```
Phase 1 — Identity          "I exist."
Phase 2 — Intelligence      "I understand."
Phase 3 — Operational       "I am observable."
Phase 4 — Federation        "I am trusted to participate."
```

---

## Locked Planning Decisions

### Decision 1: Candidate Flow vs. Existing Registration

**Candidate flow is an additional layer, not a replacement.**

The node owns its own lifecycle (unregistered → registered). The authority admission lifecycle (discovered → candidate → evidence → review → admitted) sits on top. The node cannot self-authorize trust — it can request admission, but the owner/authority grants it.

### Decision 2: Series 2 Timing

**Implementation waits until registry governance is complete and a second node environment exists.** Contracts can be designed beforehand but not implemented until two physical nodes exist for testing.

### Decision 3: Series 3 Timing

**Deferred until MCP/Librarian access exists.** The purpose is contract alignment, not UI recreation. The dashboard replacement happens after Node contracts stabilize and Librarian UI primitives are accessible via MCP.

### Decision 4: MCP Architecture

**MCP bridge layer, not MCP inside the runtime.** A separate process consumes Node contracts and exposes them via MCP protocol. The Node itself has no MCP dependency.

### Decision 5: Sprint 1.5 Apply Boundary Scope

**Registry only.** The proposed → approved → applied state machine is formalized for registry operations. Documented as a reusable pattern but not retrofitted to other subsystems yet.

### Decision 6: Workload Distribution Model

**Recommendation-only initially.** The Node may recommend placement. It does not assign itself work. Existing allocation authority (owner decision → receipt → session creation) remains the execution gate.

---

## Series 1 — Registry Governance Hardening (5 sprints)

### Sprint 1.1: NODE-REGISTRY-CANDIDATE-FLOW-1

**Goal:** Create the admission state machine around existing registration.

**State machine:**
```
DISCOVERED
    ↓
CANDIDATE
    ↓
EVIDENCE_COLLECTION
    ↓
UNDER_REVIEW
    ↓
APPROVED / REJECTED
    ↓
ADMITTED
```

**Contract types (librarian-contracts/src/registry/):**
- `NodeCandidate` (candidate_id, node_id_ref, discovery_method, status, first_seen_at, last_updated_at)
- `CandidateEvidence` (evidence_id, candidate_id, evidence_type: "identity"|"capability"|"custody"|"health"|"owner_note", payload, collected_at)
- `CandidateReviewReceipt` (receipt_id, candidate_id, decision: "approve"|"reject"|"request_info", reviewer, reason, decided_at)

**Service (librarian-node/src/node/):**
- `RegistryCandidateService` — discover, collect evidence, review, expire

**Endpoints:**
- `POST /registry/discover` — creates candidate from node identity
- `POST /registry/candidate/{id}/collect-evidence` — gathers identity/capability/custody/health
- `POST /registry/candidate/{id}/submit-review` — transitions to UNDER_REVIEW
- `POST /registry/candidate/{id}/review` — owner decides (approve → ADMITTED, reject → REJECTED)
- `GET /registry/candidates` — list with status filter
- `GET /registry/candidate/{id}` — single detail
- `GET /registry/candidate/{id}/evidence` — collected evidence
- `POST /registry/expire` — expire stale candidates

**Wiring:** Approval triggers registration confirmation (unregistered → registered) in existing registration service.

**Boundary:** This sprint does not register nodes. It only creates the admission pathway.

---

### Sprint 1.2: NODE-REGISTRY-ENFORCEMENT-1

**Goal:** Make registry state authoritative.

**Rules enforced:**
- A node cannot claim trusted fleet membership unless registry conditions are satisfied
- A node cannot advertise verified capability unless registered with valid evidence
- A node cannot accept privileged workloads unless registered with valid custody
- Expired candidate evidence invalidates admission

**Service methods:**
- `check_registration_required()` — blocks session creation if node not registered
- `check_capability_evidence_expiry()` — marks capabilities degraded if evidence stale
- `check_candidate_expiry()` — auto-expire candidates past threshold
- `log_enforcement_event(rule, violation)` — records enforcement actions

**Policy integration:** Enforcement rules become part of the existing policy framework.

---

### Sprint 1.3: NODE-REGISTRY-MCP-TOOLS-1

**Goal:** Define the MCP tools the Node exposes for registry operations. Tool definitions only — no MCP server implementation.

**Architecture:**
```
Librarian MCP Client
    ↓
MCP Registry Bridge (separate process)
    ↓
Node Registry API
    ↓
Runtime DB
```

**Tool definitions:**
| Tool | Purpose | Access |
|------|---------|--------|
| `registry.inspect_node` | Query node identity and state | Read |
| `registry.query_candidates` | List pending candidates | Read |
| `registry.retrieve_evidence` | Get candidate evidence packages | Read |
| `registry.submit_review` | Submit owner decision on candidate | Write (gated) |
| `registry.request_action` | Request governed action | Write (gated) |

**Invariant:** No tool bypasses the existing authority workflow (owner decision → receipt → state change).

---

### Sprint 1.4: NODE-REGISTRY-OWNER-ACTIONS-1

**Goal:** Complete the human authority loop. Every registry state change requires an owner decision with receipt.

**Contract types:**
- `RegistryOwnerAction` (action_id, action_type: "approve_candidate"|"reject_candidate"|"suspend_node"|"reinstate_node"|"expire_evidence"|"override_enforcement", target_id, owner, reason)
- `RegistryOwnerActionReceipt` (receipt_id, action_id, previous_state, new_state, custody_envelope_id)

**Flow:**
```
Candidate
    ↓
Evidence Package
    ↓
Owner Decision
    ↓
Decision Receipt
    ↓
Registry Mutation
```

**Wiring:** Single entry point for all registry owner decisions. Candidate approval (1.1), enforcement overrides (1.2), future suspension/reinstatement all route through this service.

---

### Sprint 1.5: NODE-REGISTRY-APPLY-BOUNDARY-1

**Goal:** Formalize state transition boundaries for registry operations.

**State model:**
```
PROPOSED
    ↓
APPROVED
    ↓
APPLIED
    ↓
VERIFIED
```

**Contract types:**
- `RegistryStateChange` (change_id, target_type, target_id, proposed_state, approved_state, applied_state, status: "proposed"|"approved"|"applied"|"rejected"|"failed")

**Key invariant:** Proposed state must never auto-advance to applied. Every transition requires explicit approval.

**Scope:** Registry operations only. Documented as a reusable pattern for workload allocation, policy changes, and capability changes — but not retrofitted to those subsystems yet.

---

## Series 2 — Multi-Node Coordination (4 sprints)

**Timing:** Implementation waits until registry governance is complete AND a second node environment exists.

### Sprint 2.1: NODE-FLEET-DISCOVERY-1

Allow trusted nodes to discover peers. Discovery ≠ admission — no trust granted by discovery alone.

### Sprint 2.2: NODE-FLEET-TRUST-SYNC-1

Synchronize identity state, capability evidence, custody status, and trust expiration across fleet members.

### Sprint 2.3: NODE-WORKLOAD-DISTRIBUTION-1

Recommendation-only. Node may recommend placement; it does not assign itself work. Owner decides through existing allocation authority.

### Sprint 2.4: NODE-CROSS-NODE-EVIDENCE-1

Extend custody chain across multiple nodes. Proof chain preserves lineage across machine boundaries without cross-node mutation.

---

## Series 3 — Librarian Integration Layer (3 sprints)

**Timing:** Deferred until MCP/Librarian access exists. Dashboard replacement happens after Node contracts stabilize.

### Sprint 3.1: NODE-MCP-LIBRARIAN-BRIDGE-1

Node MCP resources and tool definitions. Authentication boundary. Evidence retrieval contracts.

### Sprint 3.2: NODE-EVIDENCE-CONSUMPTION-1

Librarian consumes Node evidence (not Node becoming Librarian replacement).

```
Node Evidence → MCP Bridge → Librarian Core → Owner View
```

### Sprint 3.3: NODE-OPERATIONAL-SURFACE-INTEGRATION-1

Replace temporary dashboard with Librarian UI primitives. Dashboard becomes Librarian View + Node Operational Data, not a separate application.

---

## Epic-Level Invariants

| Invariant | Enforcement |
|-----------|-------------|
| Node cannot claim trusted roles without verified identity + evidence | Registry enforcement blocks sessions and capability claims |
| All registry transitions produce receipts entering custody chain | OwnerAction service generates receipts for every mutation |
| Owner decision required for all registry state changes | Apply boundary enforces proposed → approved → applied |
| MCP exposes controlled operations, not authority bypass | Tool definitions gated; no tool bypasses existing authority workflow |
| Cross-node evidence is append-only custody | Evidence chain preserves lineage; no cross-node mutation |
| Distribution recommends, does not dispatch | Workload distribution generates recommendations; owner decides via existing allocation |
| A node may prove capability, request trust, and provide evidence. It may not grant itself authority. | Candidate flow adds outer layer; registration remains internally owned |
