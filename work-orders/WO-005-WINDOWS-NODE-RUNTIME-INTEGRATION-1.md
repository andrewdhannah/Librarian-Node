# WO-005: WINDOWS-NODE-RUNTIME-INTEGRATION-1

**Status:** Authorized
**Repository:** Librarian-Node
**Dependencies:** WO-001 through WO-004 (complete)

---

## Purpose

Prove that an execution runtime can consume the governance substrate — `LifecycleState`, `ResidencyState`, `Custody`, `Evidence`, `Receipts` — without introducing new concepts.

## Scope

- Connect existing Windows runtime scripts to the governance database
- Route runtime lifecycle events through `GovernanceDb` persistence
- Map process start/stop/health to `ResidencyState` transitions
- Generate evidence on runtime operations
- Produce receipts for service lifecycle events
- Validate that the existing Rust router can report governance state

## Protected Scope

- No new contract types
- No new authority paths
- No capability expansion
- No model qualification logic
- No Windows-only governance concepts
- No redefinition of existing contract semantics

## Acceptance Gates

| Gate | Criteria |
|------|----------|
| RI-1 | Runtime start/stop events produce `ResidencyState` transitions persisted to `GovernanceDb` |
| RI-2 | Evidence records are created for runtime operations (start, stop, health check) |
| RI-3 | Receipts are created for service lifecycle events |
| RI-4 | Contract types are used — no runtime-specific state machine is introduced |
| RI-5 | No new contract types, no new authority paths, no capability expansion |
