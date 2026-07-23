# WO-003: PHASE0-EVIDENCE-COLLECTION-1

**Status:** Authorized
**Epic:** Platform Foundation
**Repository:** Librarian-Node

---

## Purpose

Establish environmental readiness and contract compatibility for the Librarian-Node workspace.

The question being answered:

> Can this environment execute the same contract assumptions as the reference implementation?

Not:

> Can we build the Windows version?

---

## Non-Goals

- No governance algorithm implementation
- No runtime expansion
- No new authority paths
- No librarian-core implementation
- No librarian-node implementation
- No contract modification

---

## Evidence Collection

### 1. Environment Evidence

Collect host, build, and runtime facts into `evidence/phase0/environment/`.

#### Host Facts

| Fact | Source | Evidence File |
|------|--------|---------------|
| OS name and version | `uname -a` / `ver` | `environment/host-os.txt` |
| CPU model and features | `/proc/cpuinfo` / `sysctl` | `environment/host-cpu.txt` |
| Available memory | `free -h` / `vm_stat` | `environment/host-memory.txt` |
| Filesystem layout | `ls` of key paths | `environment/host-filesystem.txt` |
| Disk space | `df -h` | `environment/host-disk.txt` |
| Network configuration | `ifconfig` / `ip addr` | `environment/host-network.txt` |

#### Build Facts

| Fact | Source | Evidence File |
|------|--------|---------------|
| Rust toolchain version | `rustc --version` | `environment/build-rustc.txt` |
| Cargo version | `cargo --version` | `environment/build-cargo.txt` |
| Target triples | `rustc --print cfg` | `environment/build-targets.txt` |
| Dependency tree | `cargo tree` | `environment/build-dependencies.txt` |
| Lock file hash | SHA-256 of Cargo.lock | `environment/build-lock-hash.txt` |

#### Runtime Facts

| Fact | Source | Evidence File |
|------|--------|---------------|
| Process supervision | `ps aux` / `tasklist` | `environment/runtime-processes.txt` |
| Environment variables | `env` / `Get-ChildItem Env:` | `environment/runtime-env.txt` |
| Permissions | `ls -la` on key paths | `environment/runtime-permissions.txt` |

### 2. Contract Validation Evidence

Validate that the contract crate loads and operates correctly.

| Check | Method | Evidence File |
|-------|--------|---------------|
| Crate compiles | `cargo check -p librarian-contracts` | `contracts/crate-check.txt` |
| All tests pass | `cargo test -p librarian-contracts` | `contracts/test-results.txt` |
| All 8 modules present | Source inspection | `contracts/module-inventory.txt` |
| Contract versions correct | Version constants match expected | `contracts/version-report.txt` |

### 3. Serialization Evidence

Prove deterministic serialization.

| Check | Method | Evidence File |
|-------|--------|---------------|
| Canonical JSON stable | Serialize same struct twice, compare output | `serialization/canonical-stability.txt` |
| SHA-256 deterministic | Hash same content twice, compare output | `serialization/hash-stability.txt` |
| Round-trip fidelity | Serialize → deserialize → re-serialize, compare | `serialization/round-trip.txt` |
| Cross-platform compatible | JSON output matches expected format | `serialization/wire-format.txt` |

### 4. Compatibility Evidence

Verify version resolution and contract compatibility.

| Check | Method | Evidence File |
|-------|--------|---------------|
| Contract versions resolve | All `*_CONTRACT_VERSION` constants match expected | `compatibility/version-resolution.txt` |
| Serialization envelope valid | Envelope format matches schema | `compatibility/envelope-validation.txt` |
| Forward compatibility works | Unknown fields preserved | `compatibility/forward-compat.txt` |

---

## Output

All evidence is written to `evidence/phase0/`:

```
evidence/phase0/
├── environment/
│   ├── host-os.txt
│   ├── host-cpu.txt
│   ├── host-memory.txt
│   ├── host-filesystem.txt
│   ├── host-disk.txt
│   ├── host-network.txt
│   ├── build-rustc.txt
│   ├── build-cargo.txt
│   ├── build-targets.txt
│   ├── build-dependencies.txt
│   ├── build-lock-hash.txt
│   ├── runtime-processes.txt
│   ├── runtime-env.txt
│   └── runtime-permissions.txt
├── contracts/
│   ├── crate-check.txt
│   ├── test-results.txt
│   ├── module-inventory.txt
│   └── version-report.txt
├── serialization/
│   ├── canonical-stability.txt
│   ├── hash-stability.txt
│   ├── round-trip.txt
│   └── wire-format.txt
├── compatibility/
│   ├── version-resolution.txt
│   ├── envelope-validation.txt
│   └── forward-compat.txt
└── PHASE0-CERTIFICATION-RECEIPT.md
```

---

## Acceptance Gates

| Gate | Requirement |
|------|-------------|
| PC-1 | Environment evidence collected and verifiable |
| PC-2 | Contract crate validated — loads, tests pass, modules present |
| PC-3 | Serialization proven deterministic across runs |
| PC-4 | Hashes proven stable across serialization runs |
| PC-5 | Repository integrity confirmed — workspace complete, exclusions intact |
| PC-6 | Phase 0 evidence packet produced with all findings |

---

## Dependencies

- Authorization receipt AR-WO-003-20260723
- Existing `librarian-contracts` crate (complete and compiled)
- Rust toolchain installed

## Effort

1 sprint (~1 week)

## Sequence

```
WO-003: PHASE0-EVIDENCE-COLLECTION-1   ← CURRENT
        ↓
WO-004: RUST-CORE-GOVERNANCE-PORT-1    ← Blocked until WO-003 complete
        ↓
WO-005: WINDOWS-NODE-RUNTIME-INTEGRATION-1
        ↓
WO-006: UBUNTU-LINUX-TIER1-PORT-1
```
