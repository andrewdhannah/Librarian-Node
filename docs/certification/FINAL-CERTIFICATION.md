# Final Certification

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  

---

## 1. Purpose

Define the final certification process for the Windows Runtime Node. Final certification is issued when the Node has completed all planned sprints and meets all acceptance criteria.

---

## 2. Certification Criteria

### 2.1 Architectural Criteria

| Criterion | Description |
|-----------|-------------|
| Authority boundaries | Core ↔ Node separation enforced |
| Lifecycle compliance | Node follows ADR-PLATFORM-002 lifecycle |
| Packet contracts | All cross-boundary communication uses sealed types |
| MCP boundary | MCP is Agent ↔ Core transport only |

### 2.2 Operational Criteria

| Criterion | Description |
|-----------|-------------|
| Installation | Node installs on clean machine |
| Initialization | Databases created and migrated |
| Qualification | Hardware measured and profiled |
| Identity | Node identity generated and persistent |
| READY state | Node reaches READY without Core |
| Upgrade | Node upgrades safely |
| Recovery | Node recovers from failure |
| Migration | Node migrates durable state |

### 2.3 Security Criteria

| Criterion | Description |
|-----------|-------------|
| Secrets policy | No secrets in repository |
| Access control | Localhost-only by default |
| Identity integrity | Identity survives restart |
| Evidence integrity | Evidence is append-only |

### 2.4 Performance Criteria

| Criterion | Description |
|-----------|-------------|
| Model loading | Models load within acceptable time |
| Inference | Inference completes within timeout |
| VRAM | VRAM usage within 4GB limit |
| Health | Health endpoint responds |

---

## 3. Certification Process

```
All Sprints Complete
    │
    Verify All Acceptance Gates
    │
    Verify All Invariants
    │
    Review All Evidence
    │
    Performance Qualification
    │
    Security Review
    │
    Issue Final Certification
    │
    Proceed to Integration
```

---

## 4. Certification Artifacts

| Artifact | Location | Description |
|----------|----------|-------------|
| Sprint certifications | `docs/certification/` | Per-sprint certification |
| Performance baseline | `docs/performance/BASELINE.md` | Performance measurements |
| Security baseline | `docs/security/SECURITY-BASELINE.md` | Security posture |
| Invariant review | `docs/planning/INVARIANT-REVIEW.md` | Invariant verification |
| Evidence review | `evidence/` | All lifecycle evidence |

---

## 5. References

- SPRINT-CERTIFICATION.md — Sprint certification process
- RECOVERY-CERTIFICATION.md — Recovery certification
- INVARIANT-REVIEW.md — Invariant review
- ADR-PLATFORM-002 — Platform Lifecycle
