# EPIC-NODE-INSTALLABILITY-AND-PORTABILITY-1

**Status:** PLANNING  
**Layer:** Runtime Node  
**Depends On:**
- ADR-PLATFORM-001 CORE / NODE AUTHORITY ARCHITECTURE
- ADR-PLATFORM-002 PLATFORM LIFECYCLE
- ARCHITECTURAL-BOUNDARY-MAP

---

## Objective

Transform the Runtime Node from a development-capable runtime into an installable, portable, recoverable platform component.

This epic implements the Node component-local lifecycle:

```
ABSENT
  ↓
INSTALL
  ↓
INITIALIZE
  ↓
QUALIFY
  ↓
IDENTITY
  ↓
READY
```

This epic does not implement:

- Core admission
- Node discovery
- Fleet trust
- Owner approval
- Multi-node coordination

Those belong to Core relationship lifecycle.

---

## Invariants

### NODE-I1 — Standalone Capability

A Node must be able to reach READY without Core connectivity.

### NODE-I2 — Installation Before Activation

Installation prepares the system. Activation into platform operations occurs later.

### NODE-I3 — Evidence Preservation

All lifecycle events produce append-only evidence.

### NODE-I4 — Identity Persistence

Node identity survives restart and upgrade.

### NODE-I5 — Hardware Requalification

Hardware changes require qualification evidence refresh.

---

## Sprint Sequence

---

### NODE-INSTALL-FOUNDATION-1

**Objective:** Create the Node installation contract.

**Scope:** Define:
- installation layout
- runtime directories
- configuration locations
- database locations
- logging locations
- version metadata

**Deliverables:**
- installer specification
- filesystem contract
- installation validator

**Gates:**

| Gate | Description |
|------|-------------|
| INSTALL-1 | Installation creates required structure |
| INSTALL-2 | Missing dependencies are detected |
| INSTALL-3 | Version information is recorded |
| INSTALL-4 | Installation is repeatable |

---

### NODE-FIRST-RUN-INITIALIZATION-1

**Objective:** Implement first-run initialization.

**Scope:** Create:
- local database
- configuration
- runtime identity preparation
- lifecycle evidence store

**State Transition:** INSTALL → INITIALIZE

**Gates:**

| Gate | Description |
|------|-------------|
| INIT-1 | Initialization is idempotent |
| INIT-2 | Database migrations execute correctly |
| INIT-3 | Configuration validation succeeds |
| INIT-4 | Initialization failure rolls back cleanly |

---

### NODE-HARDWARE-QUALIFICATION-LIFECYCLE-1

**Objective:** Move qualification from development tooling into lifecycle.

**Scope:** Integrate:
- hardware discovery
- GPU detection
- runtime capability checks
- model qualification

**State Transition:** INITIALIZE → QUALIFY

**Gates:**

| Gate | Description |
|------|-------------|
| QUALIFY-1 | Hardware profile recorded |
| QUALIFY-2 | Qualification evidence generated |
| QUALIFY-3 | Unsupported hardware fails safely |
| QUALIFY-4 | Qualification is repeatable |

---

### NODE-IDENTITY-LIFECYCLE-1

**Objective:** Formalize Node identity creation.

**Scope:** Implement:
- identity generation
- identity persistence
- identity recovery
- identity evidence

**State Transition:** QUALIFY → IDENTITY

**Gates:**

| Gate | Description |
|------|-------------|
| IDENTITY-1 | Unique identity generated |
| IDENTITY-2 | Identity survives restart |
| IDENTITY-3 | Identity changes require explicit reset |

---

### NODE-READY-STATE-CONTRACT-1

**Objective:** Define operational readiness.

**Scope:** READY means:
- installed
- initialized
- qualified
- identified

READY does not mean admitted.

**Gates:**

| Gate | Description |
|------|-------------|
| READY-1 | Node reports READY state |
| READY-2 | Health contract available |
| READY-3 | Lifecycle state persisted |

---

### NODE-UPGRADE-AND-RECOVERY-1

**Objective:** Implement lifecycle recovery.

**Scope:** Support:
- upgrades
- rollback
- interrupted installation recovery
- database migration recovery

**Gates:**

| Gate | Description |
|------|-------------|
| RECOVERY-1 | Failed upgrade restores previous state |
| RECOVERY-2 | Evidence remains append-only |
| RECOVERY-3 | Identity remains stable |

---

### NODE-PORTABILITY-MIGRATION-1

**Objective:** Implement Node migration protocol participation.

**Scope:**

**Export:**
- database
- configuration
- evidence
- identity metadata

**Import:**
- durable state
- validation
- requalification

**Gates:**

| Gate | Description |
|------|-------------|
| MIGRATION-1 | Export is verifiable |
| MIGRATION-2 | Import validates state |
| MIGRATION-3 | Old installation can retire safely |

---

## Epic Exit Criteria

**Node can:**
- ✓ install on clean machine
- ✓ initialize independently
- ✓ qualify hardware
- ✓ create identity
- ✓ reach READY
- ✓ upgrade safely
- ✓ recover from failure
- ✓ migrate durable state

**Node cannot:**
- self-admit
- bypass Core authority
- become fleet trusted without admission
