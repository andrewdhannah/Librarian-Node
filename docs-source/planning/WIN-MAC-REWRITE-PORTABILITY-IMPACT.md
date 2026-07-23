# Mac Rewrite Portability Impact

**Status:** Planning input for canonical Mac Librarian design
**Date:** 2026-07-01
**Sprint:** WIN-MULTIPLATFORM-LIBRARIAN-PLANNING-1
**Author:** Windows PC lane

---

## 1. Purpose

Record how the multiplatform node model and portability requirements affect the Mac Librarian rewrite. This document identifies where the Mac implementation should use Swift for native shell purposes and where it should keep governance algorithms portable for future Windows/Linux use.

This document is an **input** to the Mac canonical design sprint. It is not the canonical architecture document.

---

## 2. Core Principle: Swift/Mac Layer Hosts; Portable Core Owns the Algorithms

The Mac Librarian rewrite should layer native macOS code on top of a portable algorithm core:

```
┌──────────────────────────────────────────┐
│  Swift / macOS Layer (native shell)       │
│  - App lifecycle                          │
│  - Windows, menus, dialogs                │
│  - File picker, drag-and-drop             │
│  - Local host bridge (macOS services)     │
│  - Platform integration (Spotlight, etc.) │
│  - Native notifications                   │
├──────────────────────────────────────────┤
│  Portable Core (language-neutral)         │
│  - Packet schema validation               │
│  - Receipt chain validation               │
│  - Sprint ledger logic                    │
│  - Document lock/lease model              │
│  - Custody rules                          │
│  - Node role validation                   │
│  - MCP tool contract models               │
│  - Proposal intake validation             │
│  - Pre/post-flight state model            │
│  - Action receipt model                   │
├──────────────────────────────────────────┤
│  HTML / JS UI (where portable)            │
│  - Receipt visualization                  │
│  - Ledger display                         │
│  - Proposal review UI                     │
│  - Status dashboards                      │
└──────────────────────────────────────────┘
```

### Rule

> Swift should host/adapt, not own the durable algorithms.

The Mac implementation may use Swift for the native shell now. But core governance algorithms should work in any language. If the algorithm can only run in Swift, it is not portable — and the architecture should have a documented reason for that exception.

---

## 3. What Swift/Mac Layer Should Own

The Swift/macOS layer is responsible for:

### 3.1 Native Shell
- App window management (NSWindow, NSWindowController)
- Menu bar and keyboard shortcuts
- Document-based app conventions
- Drag-and-drop file import
- File picker dialogs (NSOpenPanel, NSSavePanel)

### 3.2 Platform Integration
- macOS service integration (Services menu, Quick Actions)
- Spotlight indexing for project documents
- Unified Logging (os_log) for diagnostics
- AppleScript / Shortcuts automation (future)
- Touch Bar support (if applicable)

### 3.3 Local Host Bridge
- Starting and stopping local model backends
- Health-check polling
- Port allocation
- Process supervision
- Filesystem access mediation

### 3.4 User-Facing UI
- Project browser and document viewer
- Proposal review and approval interface
- Receipt chain visualization
- Ledger status display
- Settings and preferences
- Onboarding and setup flows

---

## 4. What Must Stay Portable

The following must not be embedded exclusively in macOS-specific Swift code:

### 4.1 Governance Algorithms

| Algorithm | Why Portable |
|-----------|-------------|
| Sprint ledger validation | Core audit mechanism; must run on any node |
| Receipt chain validation | Core audit mechanism; must run on any node |
| Work packet validation | Core action boundary; defined in WPC project |
| Proposal intake validation | Core intake mechanism; must run on authority (any OS) |
| Document lock/lease validation | Core concurrency control; must be platform-neutral |
| Node role validation | Core authority mechanism; must run on authority |
| Custody manifest validation | Core custody mechanism; must be auditable cross-platform |
| Pre/post-flight state model | Core harness pattern; proven on Windows PowerShell |

### 4.2 Schema and Data Models

| Model | Format | Why Portable |
|-------|--------|-------------|
| Work packet schema | JSON / JSON Schema | Language-neutral by design (WPC) |
| Receipt schema | JSON / JSON Schema | Language-neutral by design |
| Sprint ledger | JSON | Language-neutral by design |
| Lock/lease metadata | JSON | Language-neutral; needed for interoperability |
| Node role definitions | JSON | Language-neutral; needed for registration |
| Proposal packet schema | JSON / JSON Schema | Language-neutral; needed for MCP transport |
| Evidence packet schema | JSON / JSON Schema | Language-neutral; needed for MCP transport |

### 4.3 MCP Tool Contracts

| Contract | Why Portable |
|----------|-------------|
| Tool definitions (names, inputs, outputs) | JSON Schema — language-neutral |
| Validation logic for tool inputs | Algorithm — not macOS-specific |
| Response formatting | Algorithm — not macOS-specific |

### 4.4 Format Conventions

- Planning documents → Markdown
- Receipts → Markdown + JSON
- Ledger → JSON
- Schemas → JSON Schema
- Packets → JSON
- Evidence → JSON, text, or file references

---

## 5. What Must NOT Be in macOS-Only UI Code

The following patterns would make portability harder or impossible:

| Anti-Pattern | Why It Hurts | Better Approach |
|--------------|--------------|-----------------|
| Embedding authority logic in a SwiftUI View | Authority becomes tied to macOS app lifecycle | Authority logic is a service; SwiftUI is a client |
| Storing receipt validation in NSDocument subclasses | Receipt logic becomes unreachable from Windows | Receipt validation is a standalone algorithm |
| Using Core Data for the sprint ledger | Ledger schema becomes macOS-specific | JSON file + in-memory model is portable |
| Implementing MCP tool contracts only in Swift | Windows agents cannot use the same contracts | Define contracts in JSON Schema; implement per platform |
| Hardcoding Mac as the project root path | Prevents PC from assuming authority later | Use relative paths or configurable project roots |
| Using macOS-only file coordination APIs for lock management | Lock model cannot be shared with Windows nodes | Use authority-mediated lock/lease model (see MCP custody notes) |

---

## 6. Architecture Guidance Summary

### Swift/macOS Layer (native shell)
```
- app lifecycle
- window management
- file picker
- menus
- drag-and-drop
- local host bridge
- platform integration
- native notifications
- settings UI
```

### Portable Core (language-neutral)
```
- packet schemas
- receipt schemas
- ledger model
- lock/lease model
- custody rules
- node roles
- MCP tool contracts
- validation logic (where practical)
- HTML/JS UI (where portable)
```

---

## 7. Migration Guidance: Mac-First Now, Portable Later

### Phase 1 — Mac Native Shell (current/planned)
- Swift app with native Cocoa UI
- Core algorithms may be embedded in Swift
- Receipts, packets, and ledgers follow the proven Windows format conventions
- Document portability intent explicitly in code comments and architecture docs

### Phase 2 — Extract Portable Core (next)
- Identify and extract standalone algorithm modules from Swift code
- Rewrite validation logic as language-independent functions (or in a portable language)
- Define all schemas in JSON Schema with versioning
- Write contract tests that any implementation must pass
- Keep the Swift layer as a thin host around the portable core

### Phase 3 — Multiplatform (future)
- Windows implements the portable core in Python or Rust
- Linux implements the portable core as needed
- All platforms share the same schemas and contract tests
- The Swift layer becomes one host among many
- Authority can run on any platform

---

## 8. Relationship to Mac Canonical Design

This document is a Windows-side input. The Mac canonical design should:

1. Design the Swift/macOS layer as a host, not the authority itself
2. Define the portable core API boundary explicitly
3. Produce all schemas in JSON Schema first, Swift Codable second
4. Keep receipt, packet, ledger, and custody models in language-neutral formats
5. Write contract tests that any reimplementation must pass
6. Document where Swift-specific exceptions are justified (and plan to remove them)

---

## 9. Owner Decisions Required

| Decision | Question |
|----------|----------|
| D-MAC-01 | Approve "Swift hosts, portable core owns algorithms" as the Mac rewrite architecture principle? |
| D-MAC-02 | Approve extracting portable core in Phase 2 (after native shell is stable)? |
| D-MAC-03 | Approve JSON Schema-first design for all governance data models? |
| D-MAC-04 | Approve contract-test-driven portability (any implementation must pass the same tests)? |
