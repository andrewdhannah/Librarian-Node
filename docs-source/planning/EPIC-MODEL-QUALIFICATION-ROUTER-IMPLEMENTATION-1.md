# EPIC-MODEL-QUALIFICATION-ROUTER-IMPLEMENTATION-1

**Status:** COMPLETE
**Prerequisite Planning Authority:** MODEL-QUALIFICATION-ROUTER-BASELINE-INTEGRATION-PLAN-1 (SEALED)
**Epic Author:** Owner
**Created:** 2026-07-11
**Closed:** 2026-07-12

---

## Epic Objective

Implement the governed model qualification and routing system that:

- binds results to exact model identity
- ingests Windows execution evidence
- runs staged qualification
- produces capability manifests
- creates Owner-approved router projections
- routes bounded work packets
- dispatches through the sealed Windows residency supervisor

while preserving:

- Mac = canonical qualification and routing authority
- Windows = runtime and residency authority

---

## Prerequisites

| Prerequisite | Status |
|-------------|--------|
| MODEL-QUALIFICATION-ROUTER-BASELINE-INTEGRATION-PLAN-1 | **SEALED** — 8 deliverables, 15 gates |
| Sprint 1 (DB Registry) | **SEALED** — 14 gates |
| Sprint 2 (Hardware & Qualification) | **SEALED** — 16 gates |
| Sprint 3 (Residency Supervisor) | **SEALED** — 23 gates |

---

## Epic Phases

### Phase 1 — Canonical Foundation

Establishes: canonical model identity, qualification identity, Mac persistence, schema ownership, version and supersession rules.

No model quality decisions yet.

| Sprint | Deliverable | Status |
|--------|------------|--------|
| MQR-F1 | Mac Canonical DB + Model Identity | **SEALED** — 38 tests, 5 gates |
| MQR-F2 | Task Pack + Validator Pack (filesystem) | **SEALED** — 28 tests, 5 gates |
| MQR-F3 | Bridge Packet Types + Serialization | **SEALED** — 46 tests, 5 gates |

### Phase 2 — Evidence and Qualification

Adds: Windows evidence export, Mac evidence intake, qualification runner, Stage 0/Stage 1, primitive probes, role trials.

Result: measured qualification evidence. Still no operational routing from raw test scores.

| Sprint | Deliverable | Status |
|--------|------------|--------|
| MQR-W1 | Windows Evidence Export API | **SEALED** — 15 tests, 15 gates |
| MQR-W2 | Windows Residency Status API | **SEALED** — 15 tests, 15 gates |
| MQR-Q1 | Qualification Runner Core | **SEALED** — 30 tests, 13 gates |
| MQR-Q2 | Stage 1 Smoke Test Executor | **SEALED** — 20 tests, 5 gates |
| MQR-Q3 | Stage 2 Primitive Probe Executor | **SEALED** — 22 tests, 5 gates |

### Phase 3 — Capability and Router Authority

Creates: capability manifests, rejection/conditional/supersession lifecycle, Owner decision path, approved router projection.

This is where "model executed successfully" becomes, through governed evidence, "model qualified for role X under constraints Y."

| Sprint | Deliverable | Status |
|--------|------------|--------|
| MQR-C1 | Capability Manifest + Owner Decision | **SEALED** — 33 tests, 5 gates |
| MQR-R1 | Execution Profile + Router Projection | **SEALED** — 34 tests, 5 gates |
| MQR-R2 | Packet Router Integration | **SEALED** — 29 tests, 5 gates |

### Phase 4 — Work Packet Routing and End-to-End Integration

Connects: packet planner → required capability → approved projection → model selection → execution profile → Windows supervisor → runtime evidence → Mac validation → next packet/replan.

| Sprint | Deliverable | Status |
|--------|------------|--------|
| MQR-I1 | End-to-End Qualification Pipeline | **SEALED** — 30 tests, 6 gates |
| MQR-I2 | End-to-End Routing Pipeline | **SEALED** — 22 tests, 6 gates |
| MQR-I3 | Comparative Analysis + Roster | **SEALED** — 41 tests, 6 gates |

---

## Epic Governance Rules

1. **Epic authorization does not mean all sprints are approved.** Each sprint requires explicit authorization before implementation begins.
2. **Each sprint has its own scope, gates, implementation, validation, and Owner seal.**
3. **Failed gates stop the epic.** No sprint proceeds if prior sprint gates are not satisfied.
4. **Scope changes require Owner approval.** The epic does not grant unlimited scope.
5. **The agent does not continue indefinitely.** After each sprint, work stops and awaits Owner review.

### Authorization Flow

```
Epic authorized
      ↓
Sprint MQR-F1 authorized → implement → validate → Owner review → seal
      ↓
Sprint MQR-F2 authorized → implement → validate → Owner review → seal
      ↓
... (each sprint follows the same pattern)
      ↓
Epic close condition verified → epic closes
```

---

## Epic Acceptance Gates

### MQR-E1 — Authority Split Preserved

No implementation collapses Mac qualification authority and Windows execution authority.

**Verification:** Every Mac-side DB operation produces no writes to Windows DB. Every Windows API call is a bounded request, not an authority transfer.

### MQR-E2 — Exact Qualification Identity Enforced

Capability results are bound to the exact artifact, runtime, hardware, task pack, validator pack, template, and generation configuration defined by the identity contract.

**Verification:** qualification_run records reference model_identity_id, task_pack_id, validator_pack_id, and system_profile_id. No qualification result exists without all four references.

### MQR-E3 — Execution Success Cannot Create Qualification

Windows runs, HTTP 200 responses, or successful token generation cannot directly create capability eligibility.

**Verification:** Windows runtime_runs has no capability columns. Windows API returns execution evidence, not qualification status. Stage 1 success does not enter capability_manifest.

### MQR-E4 — Qualification Evidence is Reproducible

Every role result is traceable to task packs, validator versions, runtime evidence, and exact model identity.

**Verification:** qualification_run → task_pack (versioned), validator_pack (versioned), evidence_packet (hash-verified), model_identity_record (SHA-256 bound).

### MQR-E5 — Rejected and Superseded Results Remain Queryable

Qualification history is preserved rather than overwritten.

**Verification:** capability_manifest supports superseded_by references. Old projections remain queryable. Qualification history is append-only.

### MQR-E6 — Router Consumes Approved Projection Only

The operational router never interprets raw benchmarks or qualification runs.

**Verification:** Router selects from router_projection WHERE superseded_by IS NULL. Router does not query qualification_run, capability_manifest, or execution_profile directly.

### MQR-E7 — Packet Routing Uses Capability Requirements

Work packets request required capabilities or job classes rather than arbitrary model names.

**Verification:** Work packet contains required_role. Router maps required_role → router_projection → model selection. No direct model name routing.

### MQR-E8 — Windows Remains Final Residency Authority

A Mac routing choice is only a residency request. Windows may reject or defer unsafe execution.

**Verification:** Mac sends qualification_request or residency_acquire. Windows validates hardware, checks residency state, and may reject. Mac handles rejection gracefully.

### MQR-E9 — DB-Backed Context Continuity Proven

Long-running work can continue across fresh model sessions using bounded DB-assembled context rather than live Markdown handoff state.

**Verification:** qualification_run persists execution context in DB. Context can be reconstructed from DB records without in-memory state.

### MQR-E10 — Q8 Canary Preserved End to End

MiniCPM5 Q8_0 may remain runtime-compatible while receiving no approved work-role assignment.

**Verification:** Q8_0 completes execution lifecycle (load → generate → release). Q8_0 has no entries in capability_manifest with status "approved" for any role. Q8_0 is not selected by router for any work packet.

### MQR-E11 — Full Lifecycle Proven

At least one bounded work packet completes the full chain:

plan → capability requirement → route → model residency → generation → evidence return → validation → context persistence → next-state derivation

**Verification:** Integration test or live proof demonstrates end-to-end packet lifecycle from routing through evidence return.

### MQR-E12 — No Authority Promotion Without Owner Path

Capability promotion and router eligibility follow the documented Owner/protocol approval rules.

**Verification:** router_projection can only be created from approved capability_manifest. capability_manifest can only reach "approved" status through owner_decision. No automatic promotion from execution evidence.

---

## Epic Close Condition

The epic closes when ALL of the following hold:

1. All 12 epic acceptance gates (MQR-E1 through MQR-E12) are satisfied
2. All 14 sprints are sealed with passing gates
3. The end-to-end chain is proven:

```
Authorized sprint
  → packet plan
  → bounded packet
  → capability requirement
  → approved model-role projection
  → runtime profile
  → Windows lease
  → single model residency
  → supervised run
  → execution evidence
  → Mac qualification/validation intake
  → durable DB state
  → next packet or Owner review
```

4. The negative control holds:

```
Q8 runtime success ≠ Q8 router eligibility
```

---

## Current Status

| Sprint | Gates | Tests | Status |
|--------|-------|-------|--------|
| MQR-F1 | 5/5 | 38 | **SEALED** |
| MQR-F2 | 5/5 | 28 | **SEALED** |
| MQR-F3 | 5/5 | 46 | **SEALED** |
| MQR-W1 | 15/15 | 15 | **SEALED** |
| MQR-W2 | 15/15 | 15 | **SEALED** |
| MQR-Q1 | 13/13 | 30 | **SEALED** |
| MQR-Q2 | 5/5 | 20 | **SEALED** |
| MQR-Q3 | 5/5 | 22 | **SEALED** |
| MQR-C1 | 5/5 | 33 | **SEALED** |
| MQR-R1 | 5/5 | 34 | **SEALED** |
| MQR-R2 | 5/5 | 29 | **SEALED** |
| MQR-I1 | 6/6 | 30 | **SEALED** |
| MQR-I2 | 6/6 | 22 | **SEALED** |
| MQR-I3 | 6/6 | 41 | **SEALED** |
| **Total** | **101/70** | **403** | **ALL SPRINTS SEALED** |
