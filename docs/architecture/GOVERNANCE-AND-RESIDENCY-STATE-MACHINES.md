# Governance and Residency State Machines

**Document:** GOVERNANCE-AND-RESIDENCY-STATE-MACHINES.md
**Version:** 1.0
**Status:** Active

---

## Purpose

Define the relationship between the two independent state machines that govern Librarian platform components: `LifecycleState` (trust/authority) and `ResidencyState` (resource occupation).

These are separate because they answer different questions. Collapsing them into one state machine confuses permission to exist with actual resource consumption.

---

## The Two Planes

```
Governance Plane                          Execution Plane
(LifecycleState)                          (ResidencyState)
                                          
Install                                   Requested
  ↓                                         ↓
Initialize                                Loading
  ↓                                         ↓
Qualify                                   Loaded
  ↓                                         ↓
Identity                                  Active
  ↓                                         ↓
Ready                                     Releasing
  ↓                                         ↓
Discovered                                Released
  ↓                                       Failed
Candidate                                 Blocked
  ↓
Admitted
  ↓
Operational
  ↓
Suspended
Retired
```

### LifecycleState — Governance Plane

Defined in `librarian-contracts/src/lifecycle.rs`

| State | Meaning |
|-------|---------|
| Install | Software installed on target system |
| Initialize | First-run setup complete |
| Qualify | Hardware and environment qualification passed |
| Identity | Node identity generated and registered |
| Ready | Accepting connections, available for discovery |
| Discovered | Found by Librarian Core or another node |
| Candidate | Under evaluation for platform admission |
| Admitted | Platform member, may begin operations |
| Operational | Fully authorized for production workloads |
| Suspended | Paused for maintenance or investigation |
| Retired | Decommissioned, no longer operational |

**Question answered:** Is this component trusted to operate?

### ResidencyState — Execution Plane

Defined in `librarian-contracts/src/residency.rs`

| State | Meaning | Occupying Resources? |
|-------|---------|---------------------|
| Requested | Start requested but not yet initiated | No |
| Loading | Instance starting, allocating resources | Yes |
| Loaded | Instance loaded and ready | Yes |
| Active | Instance actively processing | Yes |
| Releasing | Instance shutting down, releasing resources | Yes |
| Released | Instance fully stopped | No |
| Failed | Instance failed to start or runtime error | No |
| Blocked | Start blocked by policy or resource constraint | No |

**Question answered:** Is there an active instance consuming resources?

---

## Why Two State Machines

A governed execution component has two independent properties that change on different timescales:

| Property | Changes When | Example |
|----------|-------------|---------|
| Lifecycle trust | Owner authorizes or revokes | Component promoted from Candidate → Admitted |
| Residency occupation | Instance starts or stops | Model loaded into GPU memory |

These can change independently:

| Scenario | LifecycleState | ResidencyState | Valid? |
|----------|---------------|----------------|--------|
| Model installed, not running | Admitted | Released | ✅ Normal idle state |
| Model running, trusted | Operational | Active | ✅ Normal active state |
| Model revoked, still running | Suspended | Active | ⚠️ Needs forced release |
| Model running, failed | Operational | Failed | ✅ Runtime error, still trusted |
| Model retired, cannot start | Retired | Released | ✅ Cannot transition to Loading |

---

## Cross-Plane Enforcement Rules

These are invariants enforced between the two state machines, not additional states:

| Rule | Enforcement |
|------|-------------|
| R-1 | `LifecycleState::Operational` permits `ResidencyState::Active` |
| R-2 | `LifecycleState::Retired` prohibits transition to `ResidencyState::Loading` |
| R-3 | `LifecycleState::Suspended` requires transition from `Active` to `Releasing` |
| R-4 | `LifecycleState::Candidate` prohibits `ResidencyState::Active` (not yet trusted for production) |
| R-5 | `ResidencyState::Failed` does not change `LifecycleState` (runtime error ≠ trust revocation) |

---

## Applicability

`ResidencyState` is not model-specific. It applies to any governed execution component:

- **Local AI models** — model loaded into GPU memory
- **Runtime services** — daemon occupying a port
- **Plugins** — extension consuming CPU time
- **Future capability providers** — any component with resource occupation

The states are generic because the concept is generic: something is requested, loads, becomes active, releases, and eventually stops. The specific resource (GPU memory, port, CPU time) is a detail left to the implementation.

---

## Contract Location

| Type | File | Version |
|------|------|---------|
| `ResidencyState` | `librarian-contracts/src/residency.rs` | 1.0.0 |
| `ResidencyRecord` | `librarian-contracts/src/residency.rs` | 1.0.0 |
| `LifecycleState` | `librarian-contracts/src/lifecycle.rs` | 1.1.0 |

---

## Relationship to Existing Architecture

The two-plane model aligns with the existing platform split:

```
CarbideFrame (Swift Core)
    │
    ├── Governance Plane (LifecycleState)
    │   Sets trust and authority
    │
Librarian-Node (Rust Substrate)
    │
    ├── Execution Plane (ResidencyState)
    │   Tracks resource occupation
    │
    ├── Platform Adapters (platform/)
    │   OS-specific residency supervision
    │
Librarian-Platform-Equivalence
    │
    └── Validates both planes produce correct evidence
```

The governance plane is set by the Core (authority decisions). The execution plane is managed by the Node (runtime supervision). The equivalence framework validates both produce the correct evidence.
