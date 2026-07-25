# Librarian Node

**Status:** Production — Audit Phase  
**Lifecycle:** ADR-PLATFORM-002 (Platform Lifecycle)  

---

## Purpose

This repository contains the **shared Rust substrate** for The Librarian platform. It is a cross-platform Rust workspace that provides the portable Node layer — contracts, governance algorithms, and runtime execution — for every platform the Librarian runs on.

| Plane | Crate | Responsibility |
|-------|-------|---------------|
| **Contract** | `librarian-contracts` | Identity, lifecycle, evidence, receipts, custody, capabilities, serialization |
| **Capability** | `librarian-core` | Governance algorithms, qualification, evidence pipeline, registry |
| **Execution** | `librarian-node` | Services, residency supervisor, operator dashboard |
| **Observation** | `scripts/`, `fixtures/`, `reports/` | Evidence collection, qualification, diagnostics |

## Repository Architecture

```
Librarian-Node/
├── librarian-contracts/     # Portable contract definitions (Rust crate)
├── librarian-core/          # Portable governance algorithms (Rust crate)
├── librarian-node/          # Portable node runtime (Rust crate)
├── contracts/               # Shared startup contracts (platform-neutral specs)
│   └── startup/             #   STARTUP-PROTOCOL, STARTUP-OUTPUT-CONTRACT, SESSION-IDENTITY-CONTRACT
├── platform/                # Platform-specific adapters and evidence
│   ├── windows/             #   NSSM, PowerShell, Windows service + startup adapter
│   ├── linux/               #   Bash startup adapter (reference)
│   └── macos/               #   Swift startup adapter (reference)
├── schemas/                 # Platform-neutral JSON schemas
├── scripts/                 # Evidence collection, qualification, operations, validation
├── fixtures/                # Test fixtures and evidence
├── config/                  # Runtime configuration
├── docs/                    # Architecture, operations, sprints
└── evidence/                # Collected evidence artifacts
```

## Platform Support

| Platform | Core | Node | Status |
|----------|------|------|--------|
| Windows | Planned | Active | Rust router + PowerShell + NSSM + startup adapter |
| macOS | Swift (separate repo) | Reference | launchd adapter + Swift startup adapter |
| Linux | Planned | Reference | systemd adapter + Bash startup adapter |

## Related Repositories

| Repository | Role | Language |
|-----------|------|----------|
| [Librarian-Platform-Equivalence](https://github.com/andrewdhannah/Librarian-Platform-Equivalence) | Equivalence validation framework | Docs + JSON schemas |
| CarbideFrame `active/librarian/` | macOS Core (reference implementation) | Swift |
| Future: Librarian-macOS | macOS application (extracted from CarbideFrame) | Swift |
| Future: Librarian-Linux | Linux deployment target | Rust |

## Current State

| Area | Status |
|------|--------|
| `librarian-contracts` | ✅ Complete (8 domains, 28 tests, 41 types mapped to Swift) |
| `librarian-core` | ⏳ Scaffolded (contracts ready, algorithms pending) |
| `librarian-node` | ⏳ Scaffolded (contracts ready, runtime pending) |
| Platform: Windows | ✅ Router, qualification, service integration, startup adapter |
| Platform: Linux | ✅ Reference startup adapter |
| Platform: macOS | ✅ Reference startup adapter |
| Startup Contracts | ✅ STARTUP-PROTOCOL, STARTUP-OUTPUT-CONTRACT, SESSION-IDENTITY-CONTRACT |
| Reference Architecture | ✅ NODE-REFERENCE-ARCHITECTURE, adapter boundary, three-way equivalence |
| Three-Way Equivalence | ✅ Proven: same governance input → same governance outcome on all three platforms |

## Governance Model

## Reference Architecture

The repository defines a canonical node specification that all platform implementations must satisfy:

```
              Librarian-Node Canonical Guidance
                         |
        +----------------+----------------+
        |                |                |
        ▼                ▼                ▼
     macOS Node      Windows Node     Linux Node
```

**Key documents:**
- [`contracts/startup/STARTUP-PROTOCOL.md`](contracts/startup/STARTUP-PROTOCOL.md) — 6-phase startup sequence
- [`contracts/startup/STARTUP-OUTPUT-CONTRACT.md`](contracts/startup/STARTUP-OUTPUT-CONTRACT.md) — Startup receipt format
- [`contracts/startup/SESSION-IDENTITY-CONTRACT.md`](contracts/startup/SESSION-IDENTITY-CONTRACT.md) — Node identity requirements
- [`schemas/startup-receipt.schema.json`](schemas/startup-receipt.schema.json) — Platform-neutral startup receipt schema
- [`docs/architecture/NODE-REFERENCE-ARCHITECTURE.md`](docs/architecture/NODE-REFERENCE-ARCHITECTURE.md) — Architecture overview
- [`docs/architecture/PLATFORM-ADAPTER-BOUNDARY.md`](docs/architecture/PLATFORM-ADAPTER-BOUNDARY.md) — Contract/adapter boundary
- [`docs/architecture/THREE-WAY-EQUIVALENCE-PROTOCOL.md`](docs/architecture/THREE-WAY-EQUIVALENCE-PROTOCOL.md) — Cross-platform equivalence proof

**Equivalence proven:** Same governance input produces same governance outcome across Windows, Linux, and macOS. See [`evidence/phase0/reference-architecture/`](evidence/phase0/reference-architecture/).

All changes follow the Librarian governance process:

**Proposal → Impact Analysis → Invariant Review → Owner Authorization → Implementation → Certification**

Evidence is append-only. State may change; evidence does not.

---

## License

MIT — See [LICENSE](LICENSE).

## Security

See [docs/security/SECURITY-BASELINE.md](docs/security/SECURITY-BASELINE.md).
