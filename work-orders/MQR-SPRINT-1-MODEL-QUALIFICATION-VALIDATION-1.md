# MQR-SPRINT-1: MODEL-QUALIFICATION-VALIDATION-1

**Status:** Authorized
**Repository:** Librarian-Node
**Parallel Track:** WO-005 — no dependency between the two

---

## Purpose

Demonstrate that model qualification is a consumer of the existing governance substrate — not a feature that requires new governance primitives.

## Scope

- Map existing model profiles from `config/model-profiles.json` to `Capability` contract types
- Map qualification outputs to existing `Evidence` types
- Generate `Receipt` types from qualification runs
- Track qualification runtime using `ResidencyState`
- Wire qualification results through `GovernanceDb` persistence
- Run qualification against a small model set (one profile minimum) to produce end-to-end evidence

## Protected Scope

- No new contract types
- No new authority paths
- No GPU/runtime management reimplementation
- No agent execution integration
- No full model qualification matrix
- No replacement of existing qualification scripts

## Acceptance Gates

| Gate | Criteria |
|------|----------|
| MQ-1 | Model profile maps to `Capability` type — no new capability category invented |
| MQ-2 | Qualification run produces `EvidenceRecord` using existing `EvidenceCategory` |
| MQ-3 | Qualification completion produces `Receipt` using existing `ReceiptType` |
| MQ-4 | Runtime tracking uses `ResidencyState` — no model-specific state machine |
| MQ-5 | Qualification evidence is persisted to `GovernanceDb` |
| MQ-6 | No model-specific governance primitives are introduced |
