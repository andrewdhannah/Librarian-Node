# Cross-Platform Algorithm Portability

**Status:** Planning input for canonical Mac Librarian design
**Date:** 2026-07-01
**Sprint:** WIN-MULTIPLATFORM-LIBRARIAN-PLANNING-1
**Author:** Windows PC lane

---

## 1. Purpose

Identify what algorithms, models, and tools in the Librarian system should be portable across macOS, Windows, and Linux — and what must remain platform-specific.

This document is an **input** to the Mac canonical design. It records the Windows PC lane's experience with what has proven portable (or not) during the Windows Runtime Node and harness work.

---

## 2. Core Design Principle

> Design algorithms as schema-driven and testable outside the native shell. Then write OS adapters around them.

Implementation order:
1. Define the algorithm in language-neutral terms (pseudocode, logic, state machine)
2. Define input/output schemas in JSON Schema
3. Write contract tests that any implementation must pass
4. Implement in the primary platform language (Swift, Python, Rust, etc.)
5. Write the OS adapter layer for platform-specific I/O

---

## 3. Portable Algorithms

These algorithms are inherently platform-independent. They process structured data, produce structured output, and require no operating-system-specific capabilities.

### 3.1 Sprint Ledger Validation

**Already proven:** Windows `scripts/harness/validate-sprint-ledger.ps1` — 15 structural checks, JSON input/output.

**Portability:** High. The ledger is a JSON file. Validation reads JSON, checks structure and constraints, and produces a pass/fail report.

**Implementation principle:** Schema-driven. Define the ledger schema in JSON Schema. Any language can validate against it.

### 3.2 Receipt Validation

**Already proven:** Windows `scripts/verify-receipt.ps1` — 48 structural checks across v1 and v2 receipt schemas (schema integrity, artifact hash format, cleanup semantics, secret safety).

**Portability:** High. Receipts are JSON files. Validation reads JSON, applies deterministic checks, produces pass/fail report.

**Implementation principle:** Schema-driven. Receipt formats are defined in JSON Schema. Cross-platform validation library can be shared.

### 3.3 Work Packet Validation

**Already proven:** Python `work-packet-compiler/app/validator.py` — Pydantic-enforced Work Packet contract, deterministic rejection logic for privilege escalation, forbidden actions, dishonest steps.

**Portability:** High. Work packets are defined as Pydantic models (Python) but the logic is language-neutral. The validation rules (check permissions, check blacklist, check step honesty) are straightforward deterministic functions.

**Implementation principle:** The schema is JSON Schema. Validation rules can be expressed as a rules engine or ported to any language.

### 3.4 Proposal Intake Validation

**Proposed — not yet implemented.** The proposal intake model is a natural extension of work packet validation.

**Portability:** High. Proposals are structured JSON packets. Validation rules (schema conformance, stale HEAD detection, conflict detection, permission checks) are language-neutral.

**Implementation principle:** Schema-driven intake. Define the proposal packet schema in JSON Schema. Validation rules are deterministic functions over the proposal + current state.

### 3.5 Document Lock/Lease Validation

**Proposed — not yet implemented.** Lock management is a state machine over lock tokens and document paths.

**Portability:** High. Lock state is a small JSON data structure. State transitions (acquire, release, expire, revoke) are deterministic given the current state and the requested action.

**Implementation principle:** State machine with JSON state. The authority tracks lock state in memory or a small JSON file. Lock validation is a pure function.

### 3.6 Node Role Validation

**Proposed — not yet implemented.** Role validation checks that a node's declared capabilities match its requested role, and that the requesting node is authorized.

**Portability:** High. Node roles and capabilities are JSON structures. Validation rules are deterministic.

**Implementation principle:** Schema-driven role definitions. JSON Schema for role declarations + capability manifests.

### 3.7 Custody Manifest Validation

**Already proven conceptually:** Windows harness combines pre-mutation, post-flight, receipt, and validation into a custody workflow.

**Portability:** Medium-High. The custody model is a workflow with defined states and transitions. The state machine can be defined in language-neutral terms. The actual execution depends on OS-specific process management.

**Implementation principle:** Define the custody state machine and validation rules as a portable library. Keep OS-specific process/port inspection in adapter layer.

### 3.8 Pre/Post-Flight State Model

**Already proven:** Windows `scripts/harness/pre-mutation-check.ps1` (11 checks) and `scripts/harness/postflight-check.ps1` (14 checks).

**Portability:** Medium. The state model (what to check before and after a mutation) is portable. The actual checks (service state, port state, process state) are OS-specific.

**Implementation principle:** Define the check manifest (list of checks to run) in JSON. Each platform implements the OS-specific probe for each check. The runner logic (loop over checks, collect results, produce report) is portable.

### 3.9 Action Receipt Model

**Already proven:** Windows `scripts/harness/new-action-receipt.ps1` — generates granular action receipts with 9 recognized action types.

**Portability:** High. Action receipts are structured JSON/Markdown documents. The generation logic is template-driven and language-neutral.

**Implementation principle:** Define the action receipt schema in JSON Schema. Receipt generation is template rendering + data collection.

### 3.10 MCP Tool Contract Model

**Proposed — not yet implemented.** The MCP tool contracts define what tools agents can call, what inputs they accept, and what outputs they produce.

**Portability:** High. MCP tools are defined in JSON Schema. The contract definitions are language-neutral. The server implementation may be platform-specific but the contract specification is not.

**Implementation principle:** Define all MCP tool contracts in JSON Schema first. Generate server stubs from schemas where possible.

### 3.11 Prompt Guardrail Selection Model

**Already proven conceptually:** EQ Gateway defines risk-based routing decisions (blocked, approval_required, local_first, cloud_allowed) based on EQ State metadata.

**Portability:** High. The guardrail selection matrix is a decision table over structured input (EQ State). Both the input schema and the decision logic are language-neutral.

**Implementation principle:** Decision tables or rule engines. Express guardrail rules in JSON or a simple DSL.

---

## 4. Platform-Specific Adapters

These are inherently tied to a specific operating system and must be implemented per platform.

### 4.1 macOS Swift Shell

| Capability | Why Platform-Specific |
|------------|----------------------|
| App lifecycle (NSApplication) | macOS-only framework |
| Window management (NSWindow) | macOS-only framework |
| Menu bar, keyboard shortcuts | macOS-only conventions |
| File picker (NSOpenPanel) | macOS-only API |
| Drag-and-drop | macOS-only API |
| Local model host bridge | Platform-specific process management |
| Native notifications | macOS UserNotifications |

### 4.2 Windows PowerShell Harness

| Capability | Why Platform-Specific |
|------------|----------------------|
| Pre-mutation checks (11 checks) | OS-specific service/port/process inspection |
| Post-flight checks (14 checks) | OS-specific state verification |
| Baseline drift detection | OS-specific environment queries |
| Service control | Windows Service Control Manager API |
| NSSM service lifecycle | Windows-only tool |
| Process/port inspection | Windows API (Get-Process, netstat) |
| File system path rules | Windows path conventions (drives, backslashes) |
| Permissions/UAC | Windows security model |

### 4.3 Shared Cross-Platform Capabilities (Adapter Needed)

These concepts exist on every OS but require platform-specific implementations:

| Capability | macOS | Windows | Linux |
|------------|-------|---------|-------|
| Service control | launchctl | sc.exe / NSSM | systemctl |
| Process inspection | ps / Activity Monitor | Get-Process | ps / procfs |
| Port inspection | lsof | netstat / Get-NetTCPConnection | ss / netstat |
| File system paths | POSIX | Windows (drive letters) | POSIX |
| Permissions | sudo / sandbox | UAC / elevation | sudo / capabilities |
| Notifications | UserNotifications | Windows Toast | D-Bus notifications |

---

## 5. Portability Matrix

| Algorithm/Model | Portability | Already Proven? | Schema-Driven? | OS Adapter Needed? |
|-----------------|-------------|-----------------|----------------|-------------------|
| Sprint ledger validation | High | ✅ Yes (Windows) | Yes | No |
| Receipt validation | High | ✅ Yes (Windows) | Yes | No |
| Work packet validation | High | ✅ Yes (Python/WPC) | Yes | No |
| Proposal intake validation | High | No | Yes (proposed) | No |
| Lock/lease validation | High | No | Yes (proposed) | No |
| Node role validation | High | No | Yes (proposed) | No |
| Custody manifest validation | Medium-High | ✅ Partial (Windows) | Yes (proposed) | Partially |
| Pre/post-flight state model | Medium | ✅ Yes (Windows) | Yes (proposed) | Yes |
| Action receipt model | High | ✅ Yes (Windows) | Yes | No |
| MCP tool contract model | High | No | Yes (proposed) | No |
| Prompt guardrail selection | High | ✅ Yes (EQ Gateway) | Yes | No |
| Service lifecycle | Low | ✅ Yes (Windows) | No | Yes |
| Process/port inspection | Low | ✅ Yes (Windows) | No | Yes |
| Native UI | Low | No | No | Yes |

---

## 6. Implementation Guidance

### 6.1 What to Design as Schema-Driven First

1. Sprint ledger schema (JSON Schema)
2. Receipt schema (JSON Schema) — v2 schema already exists
3. Work packet schema (JSON Schema) — proven in WPC project
4. Proposal packet schema (JSON Schema) — proposed
5. Evidence packet schema (JSON Schema) — proposed
6. Lock/lease state schema (JSON Schema) — proposed
7. Node role declaration schema (JSON Schema) — proposed
8. Custody manifest schema (JSON Schema) — proposed
9. Action receipt schema (JSON Schema) — proposed
10. MCP tool contract schemas (JSON Schema) — proposed
11. Pre-flight check manifest (JSON) — proposed
12. Post-flight check manifest (JSON) — proposed

### 6.2 What Can Be Reimplemented on Windows/Linux Later

- Sprint ledger validation → Python/Rust implementation from JSON Schema
- Receipt validation → Python/Rust implementation from JSON Schema
- Work packet validation → Python/Rust implementation from JSON Schema
- MCP tool server → Python (FastMCP) or Rust implementation
- Lock/lease state machine → Python/Rust from state machine definition
- Pre/post-flight check runner → Python/Rust with OS-specific probes

### 6.3 What Should Remain Swift-Specific

- macOS native app shell (NSApplication, windows, menus)
- macOS file picker and drag-and-drop
- macOS notifications
- macOS service integration (Spotlight, Shortcuts)

### 6.4 What Should Remain PowerShell-Specific (Windows)

- Existing harness scripts (pre-mutation, post-flight, baseline-diff, etc.)
- Windows service lifecycle scripts
- Windows-specific environment inventory

---

## 7. Good Implementation Principle

> Design algorithms as schema-driven and testable outside the native shell. Then write OS adapters around them.

This means:
1. A receipt validator should work as a command-line tool on any OS, given a receipt JSON file
2. A work packet validator should work as a library on any OS, given a packet JSON object
3. A lock manager should work as an in-process state machine on any OS, given lock state
4. The OS adapter is only needed for: file I/O, process management, port inspection, service control, native UI, and notifications

---

## 8. Relationship to Mac Canonical Design

This document is a Windows-side input. The Mac canonical design should:

1. Use the portability matrix above to decide which algorithms to implement in the portable core
2. Define each portable algorithm's schema in JSON Schema before implementing in Swift
3. Write contract tests for each portable algorithm before the Swift implementation
4. Keep the OS adapter layer thin and explicitly documented
5. Ensure that Windows and Linux can reimplement the portable algorithms without reverse-engineering Swift code

---

## 9. Owner Decisions Required

| Decision | Question |
|----------|----------|
| D-PORT-01 | Approve the portability matrix as a starting point for canonical algorithm separation? |
| D-PORT-02 | Approve contract-test-driven portability as the method for ensuring cross-platform compatibility? |
| D-PORT-03 | Approve JSON Schema-first design for all governance data models? |
| D-PORT-04 | Approve the "algorithm → schema → contract test → implementation → OS adapter" design order? |
