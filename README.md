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
├── platform/                # Platform-specific adapters and evidence
│   └── windows/             #   NSSM, PowerShell, Windows service integration
├── scripts/                 # Evidence collection, qualification, operations
├── fixtures/                # Test fixtures and evidence
├── config/                  # Runtime configuration
├── docs/                    # Architecture, operations, sprints
└── evidence/                # Collected evidence artifacts
```

## Platform Support

| Platform | Core | Node | Status |
|----------|------|------|--------|
| Windows | Planned | Active | Rust router + PowerShell + NSSM |
| macOS | Swift (separate repo) | Planned | launchd adapter |
| Linux | Planned | Planned | systemd adapter (future) |

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
| Platform: Windows | ✅ Router, qualification, service integration (pre-existing) |
| Platform: Linux | ❌ Not yet |
| Phase 0 Evidence | ⏳ Planned (WO-003) |

## Governance Model

All changes follow the Librarian governance process:

**Proposal → Impact Analysis → Invariant Review → Owner Authorization → Implementation → Certification**

Evidence is append-only. State may change; evidence does not.

---

## License

MIT — See [LICENSE](LICENSE).

## Security

See [docs/security/SECURITY-BASELINE.md](docs/security/SECURITY-BASELINE.md).
