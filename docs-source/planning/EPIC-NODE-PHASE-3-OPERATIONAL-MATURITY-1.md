# EPIC-NODE-PHASE-3-OPERATIONAL-MATURITY-1

**Status:** Ready — first sprint defined  
**Preceded By:** EPIC-NODE-PHASE-2-INTELLIGENCE-AND-RESILIENCE-1 (complete)  
**Objective:** Transform the Runtime Node from a backend-capable system into an owner-operable system without introducing control-plane behavior.

**Core invariant:** Backend Capability → Visible Evidence → Owner Understanding → Owner Decision → Receipt

**Epic-level acceptance gates:**

### Visibility
- Every major backend subsystem has a human-readable representation
- Every insight has provenance
- Every decision surface links to evidence

### Authority
- Dashboard cannot execute privileged actions without existing owner workflows
- No UI shortcut bypasses receipts
- No intelligence output becomes a command

### Consistency
- UI terminology matches backend contracts
- Status indicators reflect actual state
- No hardcoded governance state remains

## Sprint Sequence

```
Sprint 1: DASHBOARD-OPERATIONAL-INTEGRATION-1
    ↓
Sprint 2: OWNER-INSIGHT-ENRICHMENT-1
    ↓
Sprint 3: POLICY-BOUNDARY-FOUNDATION-1
    ↓
Sprint 4: CAPABILITY-LIFECYCLE-1
    ↓
Sprint 5: MODEL-RUNTIME-INTEGRATION-1
    ↓
Sprint 6: FLEET-TRUST-MANAGEMENT-1
```

## Not In This Epic
- Autonomous optimization, scheduling, or remediation
- Control-plane behavior hidden behind UI
- Intelligence output becoming commands
