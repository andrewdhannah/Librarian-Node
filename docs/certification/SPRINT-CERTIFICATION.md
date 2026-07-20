# Sprint Certification

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  

---

## 1. Purpose

Define the sprint certification process for the Windows Runtime Node. Each completed sprint must be certified before proceeding to the next sprint.

---

## 2. Certification Process

```
Sprint Complete
    │
    Verify Acceptance Gates
    │
    Review Evidence
    │
    Verify Invariants
    │
    Document Findings
    │
    Issue Certification
    │
    Proceed to Next Sprint
```

---

## 3. Certification Gates

### 3.1 Acceptance Gate Verification

| Gate | Verifier | Evidence Required |
|------|----------|-------------------|
| Code complete | Code review | Review comments |
| Tests pass | Test run | Test results |
| Evidence generated | Evidence review | Evidence files |
| Invariants preserved | Invariant review | Review document |

### 3.2 Evidence Review

| Evidence | Review Criteria |
|----------|-----------------|
| Lifecycle evidence | All transitions recorded |
| Test evidence | Tests pass with evidence |
| Rollback evidence | Rollback works with evidence |
| Error evidence | Errors handled with evidence |

### 3.3 Invariant Verification

| Invariant | Verification |
|-----------|--------------|
| Evidence is append-only | No modification of existing evidence |
| Rollback is first-class | Rollback tested and documented |
| Installability precedes activation | No premature coupling |

---

## 4. Certification Template

```markdown
# Sprint Certification: [Sprint Name]

**Date:** YYYY-MM-DD
**Certifier:** [Name]
**Sprint:** [Sprint ID]

## Acceptance Gates

| Gate | Status | Evidence |
|------|--------|----------|
| Gate 1 | ✅ / ❌ | [Link to evidence] |
| Gate 2 | ✅ / ❌ | [Link to evidence] |

## Evidence Review

| Evidence | Status | Notes |
|----------|--------|-------|
| Lifecycle evidence | ✅ / ❌ | |
| Test evidence | ✅ / ❌ | |
| Rollback evidence | ✅ / ❌ | |

## Invariant Verification

| Invariant | Status | Notes |
|-----------|--------|-------|
| Evidence is append-only | ✅ / ❌ | |
| Rollback is first-class | ✅ / ❌ | |
| Installability precedes activation | ✅ / ❌ | |

## Findings

[List any findings]

## Certification

[Certified / Not Certified]
```

---

## 5. References

- PHASE0-EXECUTION-PLAN.md — Phase 0 execution plan
- INVARIANT-REVIEW.md — Invariant review
- FINAL-CERTIFICATION.md — Final certification
- RECOVERY-CERTIFICATION.md — Recovery certification
