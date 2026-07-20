# Invariant Review

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  
**Governance Step:** 3 of 6 (Proposal → Impact Analysis → Invariant Review → Owner Authorization → Implementation → Certification)

---

## 1. Purpose

This document defines the invariants that must be preserved by any change to the Windows Runtime Node. Before a change can be authorized, it must be verified against these invariants.

---

## 2. Platform Invariants

### P-I1: Evidence is Append-Only

**Statement:** State may change; evidence does not.

**Violation:** Any operation that modifies or deletes existing evidence.

**Verification:**
- Evidence tables use INSERT only
- Evidence files are write-once
- No update/delete operations on evidence
- Recovery evidence is appended, not inserted

### P-I2: Rollback is First-Class

**Statement:** Every lifecycle phase supports rollback to the previous state.

**Violation:** Any operation that cannot be reversed.

**Verification:**
- Each sprint defines rollback behavior
- Rollback is tested before forward progress
- Rollback evidence is recorded

### P-I3: Installability Precedes Activation

**Statement:** Installation prepares the system; activation is separate.

**Violation:** Installer that also activates Core functions.

**Verification:**
- Installer scope is limited to: binary placement, database creation, configuration
- Activation requires separate step
- READY state is achievable without activation

### P-I4: Migration is Cross-Machine

**Statement:** Migration is a protocol, not a state.

**Violation:** Migration treated as a lifecycle state.

**Verification:**
- Export/import protocol defined
- Hardware requalification required
- Old installation retired after migration

---

## 3. Node Invariants

### N-I1: Standalone Capability

**Statement:** A Node must reach READY without Core connectivity.

**Violation:** Node that cannot initialize without Core.

**Verification:**
- Node installs offline
- Node qualifies hardware offline
- Node generates identity offline
- Node reaches READY offline

### N-I2: Installation Before Activation

**Statement:** Installation is complete before platform activation.

**Violation:** Installation and activation combined into single step.

**Verification:**
- INSTALL state exists before ACTIVATION
- Rollback from activation removes only activation artifacts

### N-I3: Capability Does Not Imply Authority

**Statement:** A Node reporting "I can run MiniCPM5" does not mean "MiniCPM5 is approved."

**Violation:** Node self-authorizing capabilities.

**Verification:**
- Node does not claim capability authority
- Node does not approve capability classification
- Admission requires Core/Owner process

### N-I4: Identity Persistence

**Statement:** Node identity survives restart and upgrade.

**Violation:** Identity lost on restart.

**Verification:**
- Identity stored in database
- Identity restored after restart
- Identity preserved across upgrade
- Recovery path for identity loss

### N-I5: Hardware Requalification

**Statement:** Hardware changes require qualification evidence refresh.

**Violation:** Qualification data reused across hardware changes.

**Verification:**
- Hardware profile includes machine identity
- Hardware change triggers requalification
- Old qualification data preserved as evidence

---

## 4. Core Invariants

### C-I1: Core Owns Canonical Truth

**Statement:** Core is the single source of canonical truth.

**Violation:** Node producing canonical truth.

**Verification:**
- Core creates canonical records
- Node produces advisory evidence only
- Core validates and ingests evidence
- Node cannot modify governance rules

### C-I2: MCP Exposes Capability, Not Authority

**Statement:** MCP tools expose capabilities, not authority to make decisions.

**Violation:** MCP tool that can bypass authority.

**Verification:**
- MCP tools follow proposal/evidence/receipt pattern
- No generic file-write on canonical paths
- Authority mutations require Owner approval

### C-I3: All Mutations Produce Receipts

**Statement:** Every state change produces an action receipt.

**Violation:** Mutation without receipt.

**Verification:**
- Every write operation generates receipt
- Receipts are append-only
- Receipts reference parent operation

### C-I4: Owner Authority is External to Automation

**Statement:** Owner authority is not granted to automated components.

**Violation:** Automated system making Owner-level decisions.

**Verification:**
- Owner decisions require human interaction
- Automated systems cannot change governance rules
- Authority grants require Owner signature

### C-I5: Discovery Does Not Equal Trust

**Statement:** Discovering a Node does not mean trusting it.

**Violation:** Automatic trust on discovery.

**Verification:**
- Discovery and admission are separate states
- Admission requires trust evaluation
- Trust evaluation requires evidence review
- Owner approval required for admission

---

## 5. Review Process

### 5.1 Pre-Implementation

Before implementation begins:
1. Identify affected invariants
2. Analyze violation risk
3. Document mitigation measures
4. Obtain Owner authorization for any violation

### 5.2 Post-Implementation

After implementation completes:
1. Verify invariants are preserved
2. Document verification results
3. Record in sprint certification

### 5.3 Violation Handling

**Accidental violation:**
1. Stop implementation
2. Assess impact
3. Restore invariant
4. Record violation evidence
5. Adjust process to prevent recurrence

**Intentional violation:**
1. Requires Owner authorization
2. Document justification
3. Define scope and duration
4. Record authorization evidence
5. Schedule invariant restoration

---

## 6. References

- ADR-PLATFORM-001 — Core / Node Authority Architecture
- ADR-PLATFORM-002 — Platform Lifecycle
- IMPACT-ANALYSIS.md — Impact analysis process
- EPIC-NODE-INSTALLABILITY-AND-PORTABILITY-1-PLAN — Node implementation plan
