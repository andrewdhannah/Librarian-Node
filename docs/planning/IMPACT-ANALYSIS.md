# Impact Analysis

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  
**Governance Step:** 2 of 6 (Proposal → Impact Analysis → Invariant Review → Owner Authorization → Implementation → Certification)

---

## 1. Purpose

This document defines the impact analysis process for changes to the Windows Runtime Node. Every proposed change must be analyzed for its impact on the platform before it can proceed to implementation.

---

## 2. Analysis Dimensions

### 2.1 Architectural Impact

Does the change affect:

- **Authority boundaries** — Core vs Node separation?
- **Lifecycle states** — ABSENT, INSTALL, INITIALIZE, QUALIFY, IDENTITY, READY?
- **Packet contracts** — QualificationRequest, EvidencePacket, ResidencyStatus?
- **MCP boundary** — Agent ↔ Core transport?
- **Installability** — Binary placement, filesystem layout, configuration?

### 2.2 Operational Impact

Does the change affect:

- **Evidence pipeline** — Generation, recording, export?
- **Receipt generation** — Action receipts, lifecycle evidence?
- **Health monitoring** — Health endpoints, crash recovery?
- **Logging** — Structured logging, log retention?
- **Service management** — Windows Service, start/stop, restart?

### 2.3 Security Impact

Does the change affect:

- **Node identity** — Identity generation, persistence, recovery?
- **Trust state** — Registered, verified, quarantined, revoked?
- **Secrets** — API keys, tokens, certificates, credentials?
- **Access control** — Who can read/write what?

### 2.4 Performance Impact

Does the change affect:

- **Model loading time** — Binary load, GPU allocation, health check?
- **Inference latency** — Token generation speed, context allocation?
- **Memory usage** — VRAM allocation, RAM usage, evidence storage?
- **Disk usage** — Database size, log growth, evidence archive?

### 2.5 Dependency Impact

Does the change affect:

- **Rust dependencies** — Cargo.toml changes, version bumps?
- **Runtime dependencies** — llama.cpp, Vulkan, GPU drivers?
- **External services** — MCP, Core, network endpoints?
- **Build dependencies** — Toolchain version, build flags?

---

## 3. Analysis Categories

### Category A: No Impact

The change has no measurable impact on architecture, operations, security, performance, or dependencies.

**Action:** Proceed to implementation.

### Category B: Limited Impact

The change has a measurable but contained impact.

**Action:** Document the impact, proceed to implementation with monitoring.

### Category C: Significant Impact

The change has a significant impact on one or more dimensions.

**Action:** Document the impact, escalate to Owner for authorization.

### Category D: Critical Impact

The change affects authority boundaries, security invariants, or platform-wide contracts.

**Action:** Requires full governance review and Owner authorization.

---

## 4. Analysis Template

```markdown
# Impact Analysis: [Change Name]

**Date:** YYYY-MM-DD
**Author:** [Name]
**Category:** [A/B/C/D]

## Summary

[Brief description of the proposed change]

## Architectural Impact

[Description of architectural impact, if any]

## Operational Impact

[Description of operational impact, if any]

## Security Impact

[Description of security impact, if any]

## Performance Impact

[Description of performance impact, if any]

## Dependency Impact

[Description of dependency impact, if any]

## Mitigations

[Any mitigation measures]

## Recommendation

[Proceed / Escalate / Block]
```

---

## 5. References

- ADR-PLATFORM-001 — Core / Node Authority Architecture
- ADR-PLATFORM-002 — Platform Lifecycle
- ARCHITECTURAL-BOUNDARY-MAP — Code organization
- INVARIANT-REVIEW.md — Invariant review process
