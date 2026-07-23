# MAC/WIN-ROUTER-WORKLOAD-OPTIMIZER-1 — Librarian Workload-Aware Context Route Optimizer

## Sprint Summary
- **Sprint Type**: Research / Simulator Extension
- **Repository**: `librarian-runtime-node`
- **Starting HEAD**: `2f05172`
- **Goal**: Extend the context reuse simulator into a Librarian-specific workload optimizer.

## Honesty Statement
This sprint is a **local simulator** for Librarian architecture research only.
- No production cache behavior is added.
- No GPU RDMA support is claimed.
- No real KV-cache acceleration is claimed.
- No model-server internals are modified.
- No production router behavior is changed.

## Prior Sprint Context
This sprint extends `MAC/WIN-CONTEXT-REUSE-SIMULATOR-0`, which:
- Showed context-path scheduling produces strong gains in parallel-session and large-context scenarios
- Revealed a 55% regression in Scenario C (stale cache) when strict governance blocking forced slower paths
- Classified as **HOLD** pending governance/performance tradeoff resolution

## Core Question
> Can The Librarian make better context-route decisions when the scheduler understands the kind of work being performed?

## What Was Created

### New Config: `config/router_workload_optimizer.default.json`
- 7 context routes (5 base + recent_turn_window + canonical_evidence_read)
- 9 Librarian workload profiles
- 6 graduated governance states
- 4 hardware/runtime profiles
- 6 strategies
- 8 scenarios

### New Simulator: `scripts/simulators/router_workload_optimizer.py`
- Extends the prior simulator's latency model
- Adds task-aware decision making via workload profiles
- Replaces hard stale-cache blocking with graduated governance penalties
- Hardware/runtime profile awareness
- Receipt-like explanation records for every decision

### Outputs
- `reports/router-workload-optimizer-results.json` — Machine-readable per-turn results
- `reports/MAC-WIN-ROUTER-WORKLOAD-OPTIMIZER-1.md` — Human-readable sprint report

## Acceptance Criteria Checklist
- [x] Prior simulator remains runnable (verified)
- [x] Workload profiles are configurable
- [x] At least 9 Librarian workload types modeled
- [x] At least 5 context routes modeled (7 total)
- [x] Graduated freshness/provenance states modeled (6 states)
- [x] At least 4 hardware/runtime profiles modeled (4 total)
- [x] At least 6 strategies compared (6 total)
- [x] At least 8 scenarios simulated (8 total)
- [x] Every decision includes receipt-like explanation
- [x] Report compares generic scheduler vs workload-aware optimizer
- [x] Report specifically addresses Scenario C stale-cache regression improvement
- [x] Report identifies where performance must lose to governance
- [x] No production router behavior changed
- [x] No GPU/RDMA/KV-cache acceleration claims
- [x] Existing prior simulator still runs

## Files
- `config/router_workload_optimizer.default.json` — Configuration
- `scripts/simulators/router_workload_optimizer.py` — Simulator
- `reports/router-workload-optimizer-results.json` — Results
- `reports/MAC-WIN-ROUTER-WORKLOAD-OPTIMIZER-1.md` — Report
- `docs/sprints/MAC-WIN-ROUTER-WORKLOAD-OPTIMIZER-1.md` — This doc
