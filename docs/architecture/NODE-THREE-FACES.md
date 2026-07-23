# Node Architecture: Three Faces

**Status:** Active
**Repository:** Librarian-Node

---

## Overview

The Librarian Node has three distinct interfaces corresponding to three different interaction domains. Each face has been validated to different degrees by the WO-001 through WO-006 sequence.

```
                              Users / Agents
                                  │
                              ┌───┴───┐
                              │  MCP  │
                              └───┬───┘
                                  │
              ┌───────────────────┴───────────────────┐
              │           UPWARD FACE                 │
              │   (MCP Server · Capability Router ·   │
              │    SDK Add-ons · Identity/Auth)       │
              ├───────────────────────────────────────┤
              │           NODE RUNTIME                 │
              │   (Capability Registry · Governance   │
              │    Core · Adapter Dispatcher)          │
              ├───────────────────────────────────────┤
              │          INWARD FACE                   │
              │   (Governance Substrate · Contracts   │
              │    · Evidence · Receipts · Custody)   │
              ├───────────────────────────────────────┤
              │         DOWNWARD FACE                  │
              │   (RuntimeAdapters · Platform)        │
              └───────────────────┬───────────────────┘
                                  │
              ┌───────────────────┴───────────────────┐
              │  Windows Adapter   Linux Adapter      │
              │  (NSSM/service)    (systemd/journald)  │
              └───────────────────────────────────────┘
```

---

## Face 1: Upward — Users, Agents, and External Systems

**Interface:** MCP protocol + SDK add-on API
**Status:** Architecture defined, not yet exercised

### Components

| Component | Responsibility | Status |
|-----------|---------------|--------|
| MCP Server | Accept agent/tool connections via MCP protocol | Architecture defined |
| Capability Router | Route requests to registered capabilities | Architecture defined |
| Capability Registry | Enumerate and describe available capabilities | Contract exists (`Capability`, `CapabilityRegistry`) |
| SDK Add-on Interface | Allow third-party extensions to declare capabilities | Architecture defined |
| Identity Manager | Authenticate and identify connecting principals | Contract exists (`NodeIdentity`, `IdentityClaim`) |
| Auth Manager | Authorize actions based on identity and permissions | Requires `permissions` table |

### Request Flow

```
Agent → MCP Connect → Identity Verification
    ↓
Capability Discovery → "What can I do?"
    ↓
Action Request → "Execute capability X"
    ↓
Authorization Check → "Is this principal allowed?"
    ↓
Governance Core → Evidence + Receipt
    ↓
Response → "Action complete. Receipt: {id}"
```

### Architectural Invariant

MCP does not directly manipulate runtime state. It requests capabilities through the governance layer, which produces evidence and receipts. Same dependency direction as MQR and WO-005.

---

## Face 2: Inward — Governance Substrate

**Interface:** Rust API + contract types
**Status:** ✅ Validated by WO-004, MQR, WO-005, WO-006

### Components

| Component | Status | Tests |
|-----------|--------|-------|
| `librarian-contracts` (9 modules) | ✅ Complete | 37 |
| `GovernanceDb` (5 tables) | ✅ Complete | 8 |
| `CursorEngine` | ✅ Complete | Integration |
| `CustodyEngine` | ✅ Complete | Integration |
| `EvidenceGenerator` | ✅ Complete | Integration |
| `ReceiptGenerator` | ✅ Complete | Integration |
| `EquivalenceHarness` | ✅ Complete | Integration |
| `RuntimeSupervisor` | ✅ Complete | Integration |

### Storage Maturity Roadmap

The governance storage needs additional tables to support multi-tenant and multi-principal operation:

| Table | Purpose | Priority |
|-------|---------|----------|
| Numbered migrations | Schema evolution with audit trail | Required before any new table |
| `entity_registry` | Track governed entities and their owners | Required for multi-tenant |
| `decision_records` | Persist owner authorization decisions | Required for audit |
| `permissions` | Principal-to-capability authorization | Required for access control |
| `service_registry` | Track registered runtime services | Required for observability |

---

## Face 3: Downward — Platform Adapters

**Interface:** `RuntimeAdapter` trait + `ProcessEvent` model
**Status:** ✅ Validated by WO-005 (Windows), WO-006 (Linux)

### Adapter Implementations

| Platform | Adapter | Integration | Authored |
|----------|---------|-------------|----------|
| Windows | `WindowsAdapter` | NSSM, PowerShell, Windows services | WO-005 |
| Linux | `LinuxAdapter` | systemd, journald, `/proc` | WO-006 |
| macOS | (planned) | launchd, plist, macOS process APIs | Not yet |

### Adapter Contract

Each adapter maps platform-specific events to `ProcessEvent`:

```
Platform Event            →          ProcessEvent
──────────────────────────────────────────────────
systemctl start           →          StartRequested
launchctl bootstrap       →          StartRequested
NSSM start                →          StartRequested
```

The governance layer never sees the platform-specific origin.

---

## Repository Model

The three faces map to the existing repository structure:

```
librarian-contracts/
├── identity.rs        # NodeIdentity, IdentityClaim — used by upward face
├── capabilities.rs    # Capability, CapabilityRegistry — used by upward face
├── residency.rs       # ResidencyState — used by downward face
├── evidence.rs        # EvidenceRecord — used by all faces
├── receipts.rs        # Receipt — used by all faces
└── custody.rs         # CustodyEvent — used by all faces

librarian-core/src/governance/
├── db.rs              # GovernanceDb — inward storage
├── runtime/           # RuntimeAdapter + implementations — downward face
│   ├── adapter.rs
│   ├── linux/
│   └── ...
├── qualification/     # MQR consumer
│   └── ...
├── cursor.rs          # Lifecycle cursor engine
├── custody.rs         # Custody protocol
├── evidence.rs        # Evidence generation
├── receipts.rs        # Receipt generation
└── equivalence.rs     # Equivalence validation
```

The upward face (MCP server, capability router, SDK add-ons) would live in a new layer above `librarian-core` — either in `librarian-node/src/server/` or a new `librarian-server/` crate.

---

## Dependency Direction

All three faces follow the same direction validated through WO-001 through WO-006:

```
Upward Face (MCP/agents)     Downward Face (platforms)
        │                            │
        └──────────┬─────────────────┘
                   │
                   ▼
          Governance Substrate
                   │
                   ▼
             Contracts
```

Consumers depend on governance. Governance does not depend on consumers.
