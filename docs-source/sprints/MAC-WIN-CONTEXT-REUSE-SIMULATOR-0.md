# MAC/WIN-CONTEXT-REUSE-SIMULATOR-0 — Context Reuse Scheduling Simulator

## Sprint Summary
- **Sprint Type**: Exploratory Research
- **Repository**: `librarian-runtime-node`
- **Starting HEAD**: `261c250`
- **Goal**: Explore whether The Librarian can benefit from DualPath-inspired context reuse scheduling for long agentic sessions.

## Honesty Statement
This sprint is a **local simulator** for Librarian architecture research only.
- No production cache behavior is added.
- No GPU RDMA support is claimed.
- No real KV-cache acceleration is claimed.
- No model-server internals are modified.

## Key Question
> When an agent session has large reused history and small new appends, can The Librarian reduce latency or improve routing decisions by choosing between RAM cache, SSD cache, remote Windows runtime cache, recomputation, and compressed recall packets?

## Workload Assumptions (configurable defaults)
| Parameter | Value |
|-----------|-------|
| Average reused context | 32,700 tokens |
| Average append length | 429 tokens |
| Cache-hit rate | 98.7% |
| Workload shape | Many turns, large reused history, short appends |
| Expected bottleneck | Context movement / retrieval / reuse |

*These values are in `config/context_reuse_simulator.default.json` and are configurable.*

## Simulated Reuse Paths
1. **RAM cache** — fastest, low capacity, volatile
2. **SSD cache** — slower, higher capacity, persistent
3. **Remote Windows Runtime cache** — LAN-accessible, network-dependent
4. **Recomputation from source** — always available, always fresh, most expensive
5. **Compressed recall packet** — moderate cost, includes compression/decompression overhead

## Compared Strategies
1. Always RAM when available
2. Always SSD after first turn
3. Always remote Windows runtime cache
4. Always recompute
5. Always compressed recall packet
6. **Local Scheduler** — chooses best available path based on configurable estimated costs

## Latency Model
```
estimated_latency =
  base_path_latency
  + context_tokens × context_transfer_cost
  + append_tokens × append_processing_cost
  + cache_miss_penalty
  + freshness_penalty (if enabled)
  + provenance_penalty (if enabled)
  + network_latency_and_jitter (if applicable)
  + compression/decompression cost (recall packet only)
```

## Scenarios Simulated
| ID | Scenario | Sessions | Turns | Context | Appends | Special |
|----|----------|----------|-------|---------|---------|---------|
| A | Normal Long Agent Session | 1 | 50 | 32.7K | 429 | High cache hit |
| B | Parallel Agent Sessions | 5 | 30 | 32.7K | 429 | Shared cache contention |
| C | Stale Cache / Provenance Risk | 1 | 50 | 32.7K | 429 | Freshness decay + provenance checks |
| D | Weak Windows Runtime Link | 1 | 50 | 32.7K | 429 | High LAN jitter |
| E | Recall Packet Advantage | 1 | 50 | 50K | 150 | Large context, lower hit rate |

## Files Created
- `config/context_reuse_simulator.default.json` — Configuration with all assumptions
- `scripts/simulators/context_reuse_simulator.py` — Deterministic Python simulator
- `reports/context-reuse-simulator-results.json` — Machine-readable per-turn results
- `reports/MAC-WIN-CONTEXT-REUSE-SIMULATOR-0.md` — Human-readable sprint report

## Acceptance Criteria Checklist
- [x] Simulator runs locally without external services
- [x] Simulator has deterministic output
- [x] All assumptions are documented and configurable
- [x] At least five reuse paths modeled (RAM, SSD, Remote, Recompute, Recall)
- [x] At least six strategies compared (RAM, SSD, Remote, Recompute, Recall, Scheduler)
- [x] Per-turn decision explanations generated
- [x] At least five required scenarios simulated
- [x] Human-readable sprint report created
- [x] Report states whether to graduate, hold, or archive
- [x] No production cache behavior added
- [x] No GPU/RDMA/KV acceleration claims
- [x] Existing harness/tests pass (no modifications to existing code)

## Result Classification
See `reports/MAC-WIN-CONTEXT-REUSE-SIMULATOR-0.md` for detailed analysis.
