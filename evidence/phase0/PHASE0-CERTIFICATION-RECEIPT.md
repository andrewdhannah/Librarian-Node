# Phase 0 Certification Receipt

**Receipt ID:** PC-WO-003-20260723
**Sprint:** WO-003 — PHASE0-EVIDENCE-COLLECTION-1
**Date:** 2026-07-23
**Repository:** Librarian-Node
**Evidence Directory:** `evidence/phase0/`

---

## Certification Statement

The Librarian-Node environment has been observed and documented. The following evidence confirms that this environment can execute the same contract assumptions as the Swift/macOS reference implementation.

**No implementation changes were introduced.** This is observation-only evidence.

---

## Acceptance Gate Status

| Gate | Requirement | Result |
|------|-------------|--------|
| PC-1 | Environment evidence collected | ✅ PASS — 14 evidence files |
| PC-2 | Contract crate validated | ✅ PASS — 0 errors, 28/28 tests |
| PC-3 | Serialization proven deterministic | ✅ PASS — canonical JSON stable |
| PC-4 | Hashes proven stable | ✅ PASS — SHA-256 identical across runs |
| PC-5 | Repository integrity confirmed | ✅ PASS — workspace intact, exclusions in place |
| PC-6 | Evidence packet produced | ✅ PASS — all 25 evidence files + this receipt |

**All 6 gates: PASS**

---

## Evidence Inventory

### Environment (14 files)

| File | Content | Verdict |
|------|---------|---------|
| `environment/host-os.txt` | macOS Darwin 24.6.0, x86_64 | ✅ |
| `environment/host-cpu.txt` | Intel i7-4790 @ 3.60 GHz, 4C/8T | ✅ |
| `environment/host-memory.txt` | 16 GB (17179869184 bytes) | ✅ |
| `environment/host-disk.txt` | 238 GiB total, 39 GiB available | ✅ |
| `environment/host-network.txt` | en1 active at 192.168.10.22/24 | ✅ |
| `environment/build-rustc.txt` | rustc 1.95.0 (2026-04-14), x86_64-apple-darwin | ✅ |
| `environment/build-cargo.txt` | cargo 1.95.0 (2026-03-21) | ✅ |
| `environment/build-targets.txt` | target_arch=x86_64, target_os=macos | ✅ |
| `environment/build-dependencies.txt` | All 3 workspace crates resolved | ✅ |
| `environment/build-lock-hash.txt` | SHA-256: 71ce638937fd... | ✅ |
| `environment/runtime-processes.txt` | Standard macOS process set | ✅ |
| `environment/runtime-permissions.txt` | All paths 755, owned by andrew | ✅ |

### Contracts (4 files)

| File | Summary | Verdict |
|------|---------|---------|
| `contracts/crate-check.txt` | cargo check: 0 errors, 0 warnings | ✅ |
| `contracts/test-results.txt` | cargo test: 28 passed, 0 failed | ✅ |
| `contracts/module-inventory.txt` | 8 modules, 41 public types | ✅ |
| `contracts/version-report.txt` | All 8 version constants resolve correctly | ✅ |

### Serialization (4 + 1 proof files)

| File | Summary | Verdict |
|------|---------|---------|
| `serialization/canonical-stability.txt` | Canonical JSON deterministic across runs | ✅ |
| `serialization/hash-stability.txt` | SHA-256 hash: 59b2d7ee... (64 chars, stable) | ✅ |
| `serialization/round-trip.txt` | serialize → deserialize → serialize: identical | ✅ |
| `serialization/wire-format.txt` | snake_case + SCREAMING_SNAKE_CASE match Swift | ✅ |
| `serialization/serialization-proof.txt` | Full run output with all 4 checks | ✅ |

### Compatibility (3 files)

| File | Summary | Verdict |
|------|---------|---------|
| `compatibility/version-resolution.txt` | All versions resolve, no drift | ✅ |
| `compatibility/envelope-validation.txt` | SerializationEnvelope wraps correctly | ✅ |
| `compatibility/forward-compat.txt` | ForwardCompatible preserves unknown fields | ✅ |

---

## Contract Versions

| Module | Version | Status |
|--------|---------|--------|
| Identity | 1.0.0 | ✅ |
| Lifecycle | 1.1.0 | ✅ |
| Evidence | 1.0.0 | ✅ |
| Receipts | 1.0.0 | ✅ |
| Custody | 1.0.0 | ✅ |
| Capabilities | 1.0.0 | ✅ |
| Errors | 1.0.0 | ✅ |
| Serialization | 1.0.0 | ✅ |

---

## Environment Snapshot

| Dimension | Value |
|-----------|-------|
| Host | macOS 15.6 (Darwin 24.6.0) |
| CPU | Intel Core i7-4790 @ 3.60 GHz (4C/8T) |
| Memory | 16 GB |
| Rust | 1.95.0 (2026-04-14) |
| Target | x86_64-apple-darwin |
| Workspace members | 3 (librarian-contracts, librarian-core, librarian-node) |
| Cargo.lock hash | 71ce638937fd0feb5ff4d7e00d531d2ff10e7b7c485ad14477aa03c4cbf0cf1a |
| Contracts tests | 28/28 passing |

---

## Authorization Gate for WO-004

Phase 0 evidence is complete. WO-004 (RUST-CORE-GOVERNANCE-PORT-1) may be unblocked when the Owner authorizes it.

The environment is certified to execute the same contract assumptions as the Swift/macOS reference implementation.
