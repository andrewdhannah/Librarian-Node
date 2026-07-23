# NODE-PHASE-2-ARCHITECTURE-PLANNING-1

**Status:** Planning Document  
**Scope:** Phase 2 roadmap for the Runtime Node, built on the 15-sprint execution trust substrate  
**Constraint:** No autonomous execution, scheduling, dispatch, or authority expansion  

---

## 1. Current Capability Map

### 1.1 What the Node Can Do

#### Identity, Registration, Capability Proof
- Generate and persist a node identity (UUID, hostname, platform, runtime version)
- Load identity from disk across restarts; regenerate on corruption
- Submit registration requests with optional capabilities hash
- Confirm, suspend, and retire registration
- Register capability claims (type, runtime, model)
- Link evidence packets to claims
- Verify claims against evidence packets (hash-based integrity)
- Query verification state grouped by capability type
- Snapshot capabilities from database evidence

#### Session Attribution, Bootstrap Adaptation
- Create sessions (with optional capability snapshot from bridge)
- Activate, close, and expire sessions
- Guard operations with `require_active_session` gate
- Track operation counts and evidence IDs per session
- Auto-expire stale sessions after configurable timeout
- Assess hardware (GPU, RAM, CPU, disk) and runtime status
- Generate bootstrap recommendations (runtime install, GPU driver, model sizing)
- Create bootstrap plans from approved recommendations
- Require owner approval for high-impact bootstrap actions
- Execute plans and produce bootstrap receipts

#### Evidence Custody, Integrity Verification
- Append receipts to a hash-linked custody chain (SHA-256)
- Verify full chain integrity (payload tamper detection, broken links, hash mismatch)
- Query envelopes by type, time range, provenance
- Build provenance graph with relationship links
- Apply retention policies (max envelopes, retention days)
- Seed identity as first chain envelope

#### Core Participation Contracts (Optional Sync)
- Generate node projections (identity, registration, capabilities, session/custody counts)
- Prepare sync requests with projection and sync history
- Process sync receipts and discovery responses
- Create discovery announcements and process responses
- Operate fully offline when Core endpoint is not configured
- Track sync attempt history

#### Operational Visibility
- Report per-component health (identity, registration, capabilities, sessions, bootstrap, custody, core)
- Provide node overview (status, uptime, state, counts)
- Generate diagnostic reports combining health, overview, sessions, custody
- Compute health summaries (healthy/degraded/unhealthy counts)
- Summarize sessions and custody in diagnostic form

#### Owner Workflows
- Review node state, capabilities, sessions, custody, bootstrap history
- List pending approvals (unapproved bootstrap plans, registration requests, unverified claims)
- Submit decisions (approve/reject/defer) for bootstrap plans and registration
- Generate decision receipts with before/after state
- Log decisions to action history
- Chain decision receipts into custody

#### Fleet Management
- Maintain inventory of known nodes from projections
- Aggregate fleet health (healthy/degraded/unhealthy/online/offline counts)
- Breakdown health by component
- Compare capabilities across fleet nodes
- Run discovery scans and process scan results (new/existing nodes)
- Mark nodes offline; mark offline nodes not seen since cutoff
- Persist inventory across restarts

#### Capability Allocation
- Evaluate requirements against fleet node capabilities
- Score node suitability (match ratio + evidence bonus, range 0–1)
- Generate allocation recommendations with reasoning
- Accept/reject recommendations with receipts
- Filter recommendations by status
- Find suitable nodes for requirements

#### Owner Allocation Workflows
- View pending allocation queue (only proposed recommendations)
- Review all recommendations with optional status filter
- View recommendation detail
- Submit approve/reject decisions
- Log action receipts
- Query decision history

#### Workload Session Linking
- Create workload sessions from approved allocation decisions (includes allocation link)
- Activate workload sessions (transitions underlying session to active)
- Complete and fail workload sessions (closes underlying session)
- Query links by workload ID
- List workload sessions by node, state, or all
- Custody the workload allocation link

#### Workload Lifecycle Tracking
- Inventory workloads by state (active, completed, failed, pending, cancelled)
- Build timeline of workload lifecycle events (created, activated, completed/failed, receipt)
- Query workload history with filters (node, state, type, time range, limit)
- Generate full workload review with timeline and decision chain
- Get active count, recent completed, and failed workload lists

#### Evidence Intelligence
- Analyze workload outcomes by type (success rates, durations, evidence counts)
- Analyze capability effectiveness (success correlation per capability)
- Analyze allocation accuracy (recommendation → outcome correlation)
- Generate categorized findings: workload outcome, capability, allocation, node health, trend
- Severity classification: info, notable, warning, critical
- Produce consolidated intelligence report with all analysis sections

### 1.2 What the Node Cannot Do

| Cannot | Why |
|--------|-----|
| Dispatch workloads autonomously | No scheduling engine; allocation requires owner decision |
| Override owner decisions | All mutations through bootstrap/allocation require approve/reject path |
| Modify canonical history or custody chain | Custody chain is append-only; integrity check detects tampering |
| Resolve authority conflicts | No multi-owner conflict resolution |
| Self-optimize or self-modify based on intelligence findings | Findings are informational only; no automatic action |
| Create sessions without owner-approved allocation | Workload sessions require an allocation decision receipt |
| Act as a scheduler or orchestrator | No cron, queue-based dispatch, or retry logic |
| Execute without owner presence | Bootstrap requires approval; allocation requires decision |
| Publish findings as commands | Intelligence findings are read-only reports returned to caller |
| Cross into canonical Core territory | Core participation is optional, projection-based, sync-receipt-gated |

---

## 2. Identify the Next Architectural Layer

Three branches are available. Each is evaluated below.

### Branch A — Evidence Intelligence Expansion

**Premise:** The node can answer "what happened?" (workload outcomes, capability effectiveness, allocation accuracy). Next: "what patterns require attention?"

**What this branch adds:**
- Findings classification into actionable categories (performance degradation, capability mismatch, repeated failures)
- Anomaly detection — deviation of current metrics from historical baselines
- Pattern recognition across workload types, time windows, node groups
- Trend analysis with severity escalation (repeated same-type findings escalate)
- Retention of cross-session intelligence (findings persist and compound)

**Constraints applied:**
- Detection is not action. All findings are presented to owner for review
- No automatic resolution, retry, or remediation
- No scheduler or cron — analysis is on-demand or owner-requested
- No mutation of allocation logic based on findings

**Cost/Benefit:**
- Low implementation cost (builds on existing `EvidenceIntelligenceService` structure)
- High owner value (surface actionable patterns without waiting for owner to manually cross-reference)
- Risk: findings may leak into automated recommendation scoring if not bounded

### Branch B — Owner Decision Intelligence

**Premise:** The node has decisions, findings, receipts, and history. Next: synthesize into owner-facing guidance.

**What this branch adds:**
- Owner insight surface: trend reports, success rates over time, node comparison
- Decision support: recommendations with historical evidence backing
- Capability effectiveness visualizations (which capabilities produce successful workloads)
- Allocation accuracy trends (is the scoring model improving?)
- Owner-facing dashboard summaries

**Constraints applied:**
- Insights inform owners; they do not make decisions
- No automatic adjustment of scoring weights based on historical accuracy
- No "auto-approve" for recommendations based on past patterns
- Owner always reviews and decides

**Cost/Benefit:**
- Medium implementation cost (requires aggregation queries and presentation layer)
- High owner value (owners need synthesized information to make informed decisions)
- Risk: synthesized recommendations may be perceived as automated decisions unless explicitly bounded

### Branch C — Runtime Resilience

**Premise:** The node knows what it is and what happened. Next: "can it remain trustworthy under failure?"

**What this branch adds:**
- Offline operation with integrity verification: can the node continue operating when Core is unavailable?
- Interrupted session recovery: if the node crashes mid-session, can it reconstruct state?
- Incomplete receipt handling: what happens when a receipt is partially persisted?
- Custody integrity after failure: can the chain recover if the last N envelopes were not flushed?
- Reconciliation contracts: reconnect → compare state → validate → explain differences → owner decides remediation

**Constraints applied:**
- Recovery is state reconstruction, not automatic dispatch
- All reconciliation outcomes require owner decision
- No automatic replay of failed workloads
- Owner decides whether to trust recovered state

**Cost/Benefit:**
- High implementation cost (requires failure injection, state machine recovery, reconciliation protocol)
- Critical for production readiness but less owner-visible
- Risk: recovery logic that is too aggressive could be mistaken for autonomous behavior

### Evaluation and Recommendation

**Priority order:** Branch A → Branch B → Branch C

**Rationale:**
1. **Branch A (Evidence Intelligence Expansion)** is the natural next step. The intelligence infrastructure exists but is shallow — it produces static reports. Expanding to classification, anomaly detection, and cross-session analysis adds owner value at low risk. This directly answers "what patterns should I look at?" without crossing into autonomy.

2. **Branch B (Owner Decision Intelligence)** follows naturally once findings have substance. An owner dashboard is useless without meaningful findings to display. Branch A feeds Branch B.

3. **Branch C (Runtime Resilience)** is the highest risk and highest cost. It is essential for production but should be scheduled after the intelligence layer so that owners have visibility into what happened during failures. Branch C also benefits from Branch A/B findings to help owners evaluate reconciliation outcomes.

---

## 3. Recommended Sprint Sequence

### Sprint Sequence Overview

```
NODE-EVIDENCE-INTELLIGENCE-CLASSIFICATION-1
        │
        ▼
NODE-EVIDENCE-INTELLIGENCE-ANOMALY-1
        │
        ▼
NODE-OWNER-INSIGHT-SURFACE-1
        │
        ▼
NODE-PATTERN-ESCALATION-1
        │
        ▼
NODE-RECONCILIATION-FOUNDATION-1
        │
        ▼
NODE-RECOVERY-CUSTODY-1
```

### Sprint 1: NODE-EVIDENCE-INTELLIGENCE-CLASSIFICATION-1

**Purpose:** Classify raw intelligence findings into actionable owner-facing categories.

**Scope:**
- Introduce finding categories: `performance_degradation`, `capability_mismatch`, `repeated_failure`, `allocation_drift`, `node_instability`
- Each category has structured metadata: threshold triggers, severity defaults, required evidence count
- Add cross-workload-type correlation (same node, different workload types → comparison)
- Add time-windowed analysis (last hour, last 24h, last 7d)
- Findings are read-only and owner-reviewable

**Acceptance gates:**
- Classification categories exist as contract types
- Each finding maps to exactly one classification category
- Cross-type correlation produces valid aggregate findings
- Time-windowed analysis returns correct subsets
- All findings are read-only (no mutation path from finding to action)

**Explicit exclusions:**
- No automatic action on findings
- No scheduling (analysis is on-demand)
- No modification of allocation scoring
- No owner notification infrastructure (just data contracts)

**Dependencies:** Completed sprints NODE-WORKLOAD-EVIDENCE-INTELLIGENCE-1 (provides raw analysis)

---

### Sprint 2: NODE-EVIDENCE-INTELLIGENCE-ANOMALY-1

**Purpose:** Detect deviation from historical patterns and surface as findings.

**Scope:**
- Baseline tracking: store per-workload-type and per-node historical averages (success rate, duration, evidence count)
- Deviation detection: current window differs from baseline beyond configurable threshold
- Anomaly findings: generated when deviation exceeds threshold
- Anomaly categories: `success_rate_drop`, `duration_spike`, `evidence_drop`, `node_silent`
- Baseline is stored locally (no Core persistence required)
- Owner can view anomalies alongside classification findings

**Acceptance gates:**
- Baseline computation produces correct aggregates
- Deviation detection triggers findings only above threshold
- Anomaly findings have severity based on deviation magnitude
- No automatic response: anomalies are informational only
- Baselines reset on owner request

**Explicit exclusions:**
- No automatic rollback or remediation
- No predictive or pre-emptive analysis
- No cross-node anomaly correlation (single-node per finding)
- Baseline is not synced to Core (stays local)

**Dependencies:** NODE-EVIDENCE-INTELLIGENCE-CLASSIFICATION-1 (anomalies map to classification categories)

---

### Sprint 3: NODE-OWNER-INSIGHT-SURFACE-1

**Purpose:** Create owner-facing synthesis from findings, receipts, history, and decisions.

**Scope:**
- Insight report combining findings, trends, and historical context
- Trend visualization data: success rates over time, allocation accuracy trends, node health trajectory
- Recommendation quality summary: historical accuracy of allocation scores by node and capability type
- Owner-facing summary grouped by severity and category
- All insights are read-only presentations of existing data

**Acceptance gates:**
- Insight report compiles all existing intelligence data
- Trend data is computed correctly from historical records
- Recommendation quality summary matches allocation accuracy analysis
- No new data collection — insight surface only
- No decision-enabling actions in the insight surface

**Explicit exclusions:**
- No UI — insight contracts and data structures only
- No automated recommendations based on trends
- No modification of allocation scoring based on historical accuracy
- No scheduling — insights are generated on demand

**Dependencies:** NODE-EVIDENCE-INTELLIGENCE-ANOMALY-1 (feeds meaningful anomaly findings into insight surface)

---

### Sprint 4: NODE-PATTERN-ESCALATION-1

**Purpose:** Escalate findings that repeat across sessions or time windows.

**Scope:**
- Repeated finding detection: same finding category + same context (node, workload type) across N sessions triggers escalation
- Escalation levels: `noted` → `monitored` → `actionable` → `critical`
- Each escalation level has severity and owner-facing description
- Escalation is a property of the finding, not an independent action
- Owner can acknowledge escalation (marks as `acknowledged`, stops re-escalation for that pattern)

**Acceptance gates:**
- Repeat detection uses configurable N threshold
- Escalation levels are monotonic (can only increase)
- Acknowledgement stops re-escalation for same pattern
- Escalation does not trigger any automatic action
- Escalation can be reset by owner

**Explicit exclusions:**
- No automatic remediation at any escalation level
- No notification outside the data layer (no email, no alert)
- No cross-owner escalation
- Escalation does not influence allocation scoring

**Dependencies:** NODE-OWNER-INSIGHT-SURFACE-1 (insight contracts carry escalation metadata)

---

### Sprint 5: NODE-RECONCILIATION-FOUNDATION-1

**Purpose:** Define and implement the reconciliation protocol for node reconnection after disconnection or partial failure.

**Scope:**
- Reconciliation contracts: `ReconciliationRequest`, `ReconciliationReport`, `ReconciliationAction`
- State comparison: compare local state (sessions, custody envelopes, receipts) with expected canonical state
- Difference classification: `missing_envelope`, `divergent_hash`, `orphan_session`, `incomplete_receipt`
- Report generation: what changed, what diverged, what is unrecoverable
- Owner-facing reconciliation summary with recommended actions (informational only)

**Acceptance gates:**
- Reconciliation produces categorized differences
- Differences are compared against last-known-good state
- Report includes recommended action for each difference (owner-facing only)
- Report is read-only
- Reconciliation does not mutate any state

**Explicit exclusions:**
- No automatic repair or recovery
- No state mutation during reconciliation
- No replan or re-execute logic
- Cannot reconcile against state that was never persisted

**Dependencies:** NODE-EVIDENCE-INTELLIGENCE-CLASSIFICATION-1 (differences classified using existing categories)

---

### Sprint 6: NODE-RECOVERY-CUSTODY-1

**Purpose:** Enable the node to recover custody chain integrity after interruption without automatic dispatch.

**Scope:**
- Custody recovery state machine: `healthy` → `suspect` → `reconciling` → `owner_review` → `recovered` | `failed`
- Recovery detects: partial writes, corrupted envelopes, broken chain links
- Recovery produces: evidence of what was recovered, what was lost, what requires owner decision
- All recovered state is flagged as `recovered` (not equivalent to normal state)
- Owner decides: accept recovered state, roll back to last known good, or halt

**Acceptance gates:**
- Recovery state machine transitions are explicit and observable
- Recovered state is flagged and distinguishable from normal state
- Owner must explicitly accept any state change from recovery
- No state is mutated without owner decision
- Recovery produces a custody receipt for audit trail

**Explicit exclusions:**
- No automatic rollback
- No recovery-triggered workload resumption
- Recovery cannot create actions that the node could not perform normally
- No cross-node recovery coordination

**Dependencies:** NODE-RECONCILIATION-FOUNDATION-1 (reconciliation report drives recovery decisions)

---

### Dependency Graph

```
NODE-EVIDENCE-INTELLIGENCE-CLASSIFICATION-1
  │
  ├──→ NODE-EVIDENCE-INTELLIGENCE-ANOMALY-1
  │       │
  │       └──→ NODE-OWNER-INSIGHT-SURFACE-1
  │               │
  │               └──→ NODE-PATTERN-ESCALATION-1
  │
  └──→ NODE-RECONCILIATION-FOUNDATION-1
          │
          └──→ NODE-RECOVERY-CUSTODY-1
```

---

## 4. Risk Assessment

### Primary Risk: Observation Becoming Control

The intelligence layer generates findings that describe what the node observes. The risk is that these findings are later used to drive automated decisions without explicit owner approval.

| Risk Vector | Scenario | Mitigation |
|-------------|----------|------------|
| **Finding-driven allocation** | "Node A has 100% success rate for inference workloads" → allocation system automatically prefers Node A | Allocation scoring must remain read-only against findings; owners adjust scoring weights explicitly |
| **Anomaly auto-response** | "Node B success rate dropped 80%" → system auto-removes Node B from fleet | Anomaly detection output must not feed into fleet health without owner review |
| **Pattern escalation as command** | "Same failure pattern observed 5 times" → system auto-creates bootstrap plan to fix | Escalation must remain informational; plan creation requires explicit owner action |
| **Reconciliation as resume** | "Session was interrupted, state can be recovered" → system auto-resumes the session | Recovery produces evidence for owner; owner decides resume vs. discard |
| **Intelligence as scheduling** | "Node C is idle and capable" → system auto-dispatches workload to Node C | No dispatch exists in the system; all workload creation requires allocation decision |

### Guardrails

1. **No intelligence output feeds into any mutation path.** Every analysis function returns data; no analysis function calls a mutation method.
2. **All state transitions require explicit owner decision.** Bootstrap, allocation, session creation, recovery — all gated.
3. **Findings are append-only.** Once generated, findings cannot be deleted or modified, only acknowledged by owner.
4. **Custody chain is the single source of truth for state.** Intelligence findings are derived data, not authoritative state.
5. **Reconciliation reports are read-only.** The report describes differences; only the owner can decide what to do about them.

### Indicators of Unintended Autonomous Behavior

- A finding or analysis output directly triggers a state mutation (e.g., `analyze_workloads()` calls `create_session()`)
- The system creates a session without an owner-approved allocation decision
- An anomaly detection result causes a node to be removed from fleet without owner action
- A pattern escalation automatically creates a bootstrap plan
- Recovery mutates state without an explicit owner decision receipt
- The system uses historical accuracy to auto-adjust allocation scoring weights

---

## 5. Preserved Invariants

Phase 2 must not violate any of the following invariants established in Phase 1:

| Invariant | Description | Enforcement |
|-----------|-------------|-------------|
| **Owner authority remains final** | All state mutations (registration, bootstrap, allocation, session creation, recovery) require an explicit owner decision | Every mutation path is gated by `submit_decision()` or equivalent; no bypass methods exist |
| **Evidence backs every claim** | Capability claims require linked evidence packets with hash verification | `verify_claim()` validates evidence packet; unverified claims remain unverified until evidence is linked and validated |
| **Custody chain is append-only** | Receipt envelopes cannot be modified or removed once appended | `append_receipt()` only adds; no delete/update methods; integrity check detects tampering |
| **Core participation is optional, not required** | The node operates fully offline without Core endpoint | `CoreIntegrationService` functions work with `None` endpoint; sync is best-effort |
| **Recommendations are not dispatch commands** | Allocation recommendations, bootstrap recommendations, and intelligence findings are informational; execution requires separate owner decision | `generate_recommendation()` vs `accept_recommendation()` are separate calls; findings have no execution path |
| **Session attribution is immutable** | Every session has a node_id and session_id; sessions cannot be reassigned | Session state machine only moves forward (created → active → closed/expired) |
| **Workload sessions require allocation** | Workload sessions cannot be created without an allocation decision receipt | `create_workload_session()` requires `decision_receipt_id` |
| **Intelligence is derived, not authoritative** | Findings, analyses, and reports are computed from existing data and are not the source of truth | No intelligence function writes to authoritative state; reports are generated on demand |
| **Recovery is owner-decided** | Any state reconstruction after failure requires owner acceptance | Recovery state machine requires explicit owner decision at the `owner_review` → `recovered` transition |

---

## Document Metadata

- **Generated by:** NODE-PHASE-2-ARCHITECTURE-PLANNING-1
- **Date:** 2026-07-16
- **Based on review of:** 15 completed sprints (NODE-IDENTITY-AND-CAPABILITY-FOUNDATION-1 through NODE-WORKLOAD-EVIDENCE-INTELLIGENCE-1)
- **Files reviewed:** 30+ contract types in `librarian-contracts/src/`, 20+ service implementations in `librarian-node/src/node/`
- **Source tree root:** `G:\openwork\librarian-runtime-node\`
