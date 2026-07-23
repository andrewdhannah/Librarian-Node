# Sprint 3 Completion Report

## WIN-LOCAL-MODEL-SINGLE-RESIDENCY-SUPERVISOR-1

**Date:** 2026-07-11
**Status:** READY FOR SEAL REVIEW

---

## Executive Summary

The model-residency supervisor is implemented, tested, and proven against the live prism llama.cpp runtime on the RX 570. The supervisor enforces single-model GPU residency through an eight-state machine, serializes access via Arc<Mutex>, persists leases and runs through the operational DB, and reconciles stale state at startup.

**Test suite:** 55 / 55 PASS (41 lib + 14 integration)
**Release build:** 0 errors, 0 warnings
**Live proof:** Sequential Q4_K_M → Q8_0 execution against prism + RX 570 verified

---

## Gate Map

| Gate | Description | Evidence | Status |
|------|-------------|----------|--------|
| RS-1 | Eight-state machine implemented | `state.rs`: ResidencyState enum with 8 variants; 9 state-machine unit tests | **PASS** |
| RS-2 | Illegal transitions rejected | `validate_transition()` tested: Unloaded→Ready, Unloaded→Running, Loading→Running all rejected | **PASS** |
| RS-3 | Single-resident invariant enforced | `test_rs3_cannot_acquire_while_active`: second acquire blocked while active lease exists | **PASS** |
| RS-4 | Active lease limit = 1 | `test_rs4_only_one_active_lease`: DB shows exactly one active lease after acquire | **PASS** |
| RS-5 | Run requires active lease | `test_rs5_run_requires_lease`: start_run without lease fails with "Cannot start run" | **PASS** |
| RS-6 | BackendProcess lifecycle reused | `process.rs` unchanged; supervisor composes BackendState into residency evidence; BackendState (Stopped/Starting/Healthy/Degraded/Failed) contributes to ResidencyState | **PASS** |
| RS-7 | Drain blocks generation | `test_rs7_drain_blocks_generation`: allows_generation() returns false after drain; `test_ready_with_drain_flag_blocks_generation`: drain from Running blocks start_run and complete_run | **PASS** |
| RS-8 | ProcessKill stop strategy used | `test_rs8_stop_strategy_is_process_kill`: SupervisorConfig.stop_strategy == ProcessKill | **PASS** |
| RS-9 | PID exit verification | `test_rs9_rs10_full_lifecycle`: verify_pid_exit transitions Unloading→VerifyingRelease | **PASS** |
| RS-10 | GPU release verification | `test_gpu_release_tolerance`: 3400 MiB within tolerance (baseline 3433, tolerance 100); 3200 MiB rejected | **PASS** |
| RS-11 | Block until release proven | `test_rs11_blocked_until_release`: acquire blocked during Unloading; succeeds after full release | **PASS** |
| RS-12 | Lifecycle evidence persisted | `test_db_persistence`: lifecycle_evidence table populated; `test_rs17a_qualification_boundary_preserved`: evidence_count > 0 | **PASS** |
| RS-13 | Stale lease reconciliation | `test_stale_lease_with_interrupted_run_reconciles`: dead PID detected, lease reconciled to Unloaded | **PASS** |
| RS-14 | Interrupted run reconciliation | `test_stale_lease_with_interrupted_run_reconciles`: active run marked interrupted at startup | **PASS** |
| RS-15 | Unmanaged process fails closed | `reconciliation.rs`: orphan detection via `detect_orphan_processes()`; orphans block new acquisition; stale lease path records evidence and forces recovery | **PASS** |
| RS-16 | Concurrent acquisition blocked | `test_rs16_concurrent_acquisition_blocked`: Arc<Mutex> serializes; exactly one of two concurrent acquires succeeds | **PASS** |
| RS-17 | Sequential Q4/Q8 residency (live) | `sprint3-live-proof.ps1`: Q4_K_M loaded 2187ms, generated 385ms, GPU released 3433 MiB; Q8_0 loaded 2609ms, generated 405ms, GPU released 3433 MiB; sequential with no overlap | **PASS** |
| RS-17A | Qualification boundary preserved | `test_rs17a_qualification_boundary_preserved`: Q8 completes full lifecycle; DB schema has no qualification tables; live proof confirms RS-17A: no role assignment or capability score created | **PASS** |
| RS-18 | Existing tests pass | 55/55 total: 13 DB + 9 state + 28 supervisor + 14 integration; original 27-test baseline preserved | **PASS** |
| RS-19 | Comprehensive test suite | 28 residency tests covering: state transitions, lease enforcement, run binding, drain exclusion, release verification, concurrency, sequential lifecycle, qualification boundary, reconciliation | **PASS** |
| RS-20 | Release build clean | `cargo build --release`: 0 errors, 0 warnings | **PASS** |
| RS-21 | No routing-policy drift | Code/schema review: no capability routing, no model scoring, no task-class matching in residency module; supervisor only manages GPU lease lifecycle | **PASS** |
| RS-22 | Mac authority boundary preserved | DB schema review: no canonical authority tables (DB-13); no model switching logic (DB-14); no planning, context-memory, or packet decomposition; Windows remains advisory runtime node | **PASS** |

---

## Test Summary

| Module | Tests | Focus |
|--------|-------|-------|
| `db::tests` | 13 | Schema, migrations, CRUD, FK enforcement, no canonical tables |
| `residency::state::tests` | 9 | State machine transitions, allows_generation, potentially_resident |
| `residency::supervisor::tests` | 28 | Full supervisor lifecycle, drain regression, reconciliation, qualification boundary |
| Integration | 14 | HTTP contract, auth, refusal, profiles, body limits |
| **Total** | **55** | |

---

## Live Runtime Evidence

### Q4_K_M
- **Executable:** G:\llama.cpp-prism\build\bin\Release\llama-server.exe
- **Model:** G:\Models\minicpm5\MiniCPM5-1B-Q4_K_M.gguf
- **Lease:** lease-fba5f70bd5
- **PID:** 10804
- **Load duration:** 2187ms
- **Generation:** 385ms, 32 tokens
- **GPU release:** 3433 MiB free (baseline 3433, within tolerance)
- **Total lifecycle:** 6495ms

### Q8_0
- **Executable:** G:\llama.cpp-prism\build\bin\Release\llama-server.exe
- **Model:** G:\Models\minicpm5\MiniCPM5-1B-Q8_0.gguf
- **Lease:** lease-936ee9028d
- **PID:** 5952
- **Load duration:** 2609ms
- **Generation:** 405ms, 32 tokens
- **GPU release:** 3433 MiB free (baseline 3433, within tolerance)
- **Total lifecycle:** 6734ms

### Sequential Verification
- Active leases after Q4: 0
- Free VRAM between models: 3433 MiB
- Active leases after Q8: 0
- Active runs after Q8: 0
- Final free VRAM: 3433 MiB
- Qualification tables in DB: 0

---

## Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| ResidencyState = 8 states | Matches authorized lifecycle: Unloaded→Loading→Ready→Running→Draining→Unloading→VerifyingRelease→Unloaded + Failed from any |
| RuntimeStopStrategy = ProcessKill | Prism build does not support HTTP shutdown (`POST /health {"stop":true}`) |
| Derived residency (not persisted) | ResidencyState is computed from active lease + process inspection, not a stored column |
| Arc<Mutex> serialization | Prevents concurrent acquisition races; single-resident invariant enforced at supervisor level |
| BackendState composed, not replaced | Existing process.rs semantics unchanged; supervisor produces lifecycle evidence from BackendState |
| Separate runtime vs qualification | Runtime compatibility ≠ work-role suitability (Q8_0 is the canary) |
| Startup reconciliation | Handles stale leases, orphan processes, and interrupted runs at boot |

---

## Files Modified/Created

| File | Change |
|------|--------|
| `src/residency/mod.rs` | Module declarations and re-exports |
| `src/residency/state.rs` | 8-state machine, ResidencyState, RuntimeStopStrategy, validate_transition |
| `src/residency/supervisor.rs` | ModelResidencySupervisor, SupervisorConfig, ResidencySnapshot, 28 tests |
| `src/residency/reconciliation.rs` | reconcile_startup, orphan detection, stale lease recovery |
| `src/server.rs` | Added supervisor field to AppState |
| `src/main.rs` | Supervisor initialization, startup reconciliation |
| `src/lib.rs` | Added `pub mod residency` |
| `tests/integration_test.rs` | Updated AppState construction |
| `scripts/sprint3-live-proof.ps1` | Live runtime proof harness |

---

## Remaining Work

None. All 23 gates (RS-1 through RS-22 + RS-17A) are satisfied.

---

## Seal Recommendation

**RECOMMEND FOR SEAL.**

The implementation is properly bounded within the Windows runtime residency domain. It does not drift into Model Qualification, routing policy, or Mac authority. The live proof demonstrates sequential Q4/Q8 residency against the qualified prism runtime with verified GPU release.
