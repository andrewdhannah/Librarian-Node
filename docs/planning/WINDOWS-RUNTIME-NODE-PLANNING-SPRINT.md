# Windows Runtime Node — Planning Sprint

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  
**Architecture:** ADR-PLATFORM-001 (Core / Node)  
**Lifecycle:** ADR-PLATFORM-002 (Platform Lifecycle)  

---

## 1. Purpose

This sprint defines the planning and evidence collection for the Windows Runtime Node before any corrective work begins. The sprint is strictly observational — no changes are made to the runtime, configuration, databases, or dependencies.

---

## 2. Scope

### In Scope

- Runtime process inventory
- MCP connectivity and configuration
- Network listeners and open ports
- Rust toolchain and dependency versions
- GPU and hardware inventory
- Database presence, schema versions, and migration state
- Registry reconciliation status
- Startup and error logs
- Known failures and reproducible symptoms
- Configuration review
- Dependency review

### Out of Scope

- Source code modifications
- Configuration changes
- Database modifications
- Security policy changes
- Performance optimization
- Architecture changes
- Dependency upgrades

---

## 3. Phases

### Phase 0: Evidence Collection

Collect current state of:
- Filesystem layout
- Binary versions
- Database state
- Runtime state
- MCP state
- Network state
- Process state
- Hardware state

### Phase 1: Audit

Analyze collected evidence:
- Compare against expected state
- Identify discrepancies
- Document known issues
- Classify findings by severity

### Phase 2: Certification

Produce certification packages:
- Sprint certification for each completed sprint
- Final certification for the Node

---

## 4. Evidence Policy

All evidence is stored under `evidence/` with the following structure:

```
evidence/
├── phase0/       — Current state baseline
├── sprint1/      — First implementation sprint
├── sprint2/      — Second implementation sprint
├── sprint3/      — Third implementation sprint
├── sprint4/      — Fourth implementation sprint
├── sprint5/      — Fifth implementation sprint
├── sprint6/      — Sixth implementation sprint
└── sprint7/      — Seventh implementation sprint
```

**Evidence is append-only.** State may change; evidence does not.

---

## 5. Governance

All changes follow the Librarian governance process:

```
Proposal
    ↓
Impact Analysis
    ↓
Invariant Review
    ↓
Owner Authorization
    ↓
Implementation
    ↓
Certification
```

---

## 6. References

- ADR-PLATFORM-001 — Core / Node Authority Architecture
- ADR-PLATFORM-002 — Platform Lifecycle
- ARCHITECTURAL-BOUNDARY-MAP — Code organization
- EPIC-NODE-INSTALLABILITY-AND-PORTABILITY-1-PLAN — Node implementation plan
