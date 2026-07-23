# MAC/WIN-ROUTER-WORKLOAD-OPTIMIZER-1 — Sprint Report

**Router Workload Optimizer Results**

*Librarian Workload-Aware Context Route Scheduling*


> Exploratory research only. No production cache behavior. No GPU/RDMA/KV acceleration claims.


Simulator version: `2.0.0` — extends `context_reuse_simulator.py (MAC/WIN-CONTEXT-REUSE-SIMULATOR-0)`


Workload profiles: 9 | Context routes: 7 | Hardware profiles: 4 | Strategies: 6


---

## Cross-Scenario Strategy Comparison

| Scenario | Strategy | Avg Latency (ms) | Throughput (t/s) | Safe | Warning | Revalidate | Blocked | Perf Sacrificed |
|----------|----------|------------------:|------------------:|-----:|--------:|-----------:|--------:|----------------:|
| Scenario A — Sprint Planning | AlwaysFast | 2.05 | 487.8049 | 15 | 0 | 0 | 0 | 0 |
| Scenario A — Sprint Planning | AlwaysSafe | 278.0 | 3.5971 | 15 | 0 | 0 | 0 | 0 |
| Scenario A — Sprint Planning | RecomputeHiRisk | 2.05 | 487.8049 | 15 | 0 | 0 | 0 | 0 |
| Scenario A — Sprint Planning | AlwaysRecall | 236.0 | 4.2373 | 15 | 0 | 0 | 0 | 0 |
| Scenario A — Sprint Planning | GenericSched | 2.05 | 487.8049 | 15 | 0 | 0 | 0 | 0 |
| Scenario A — Sprint Planning | WorkloadAware | 2.05 | 487.8049 | 15 | 0 | 0 | 0 | 0 |
| Scenario B — Sprint Closeout | AlwaysFast | 298.4 | 3.3512 | 7 | 5 | 13 | 0 | 0 |
| Scenario B — Sprint Closeout | AlwaysSafe | 377.0 | 2.6525 | 25 | 0 | 0 | 0 | 0 |
| Scenario B — Sprint Closeout | RecomputeHiRisk | 2262.0 | 0.4421 | 25 | 0 | 0 | 0 | 0 |
| Scenario B — Sprint Closeout | AlwaysRecall | 1197.8 | 0.8349 | 7 | 2 | 16 | 0 | 0 |
| Scenario B — Sprint Closeout | GenericSched | 312.5 | 3.2 | 15 | 2 | 8 | 0 | 0 |
| Scenario B — Sprint Closeout | WorkloadAware | 328.78 | 3.0415 | 25 | 0 | 0 | 0 | 0 |
| Scenario C — Receipt Generatio | AlwaysFast | 292.85 | 3.4147 | 7 | 2 | 11 | 0 | 0 |
| Scenario C — Receipt Generatio | AlwaysSafe | 226.5 | 4.415 | 20 | 0 | 0 | 0 | 0 |
| Scenario C — Receipt Generatio | RecomputeHiRisk | 1509.0 | 0.6627 | 20 | 0 | 0 | 0 | 0 |
| Scenario C — Receipt Generatio | AlwaysRecall | 1092.75 | 0.9151 | 7 | 2 | 11 | 0 | 0 |
| Scenario C — Receipt Generatio | GenericSched | 272.36 | 3.6716 | 10 | 2 | 8 | 0 | 0 |
| Scenario C — Receipt Generatio | WorkloadAware | 218.31 | 4.5806 | 20 | 0 | 0 | 0 | 0 |
| Scenario D — Agent Handoff | AlwaysFast | 69.42 | 14.4058 | 30 | 0 | 0 | 0 | 0 |
| Scenario D — Agent Handoff | AlwaysSafe | 353.0 | 2.8329 | 30 | 0 | 0 | 0 | 0 |
| Scenario D — Agent Handoff | RecomputeHiRisk | 69.42 | 14.4058 | 30 | 0 | 0 | 0 | 0 |
| Scenario D — Agent Handoff | AlwaysRecall | 544.87 | 1.8353 | 30 | 0 | 0 | 0 | 0 |
| Scenario D — Agent Handoff | GenericSched | 69.42 | 14.4058 | 30 | 0 | 0 | 0 | 0 |
| Scenario D — Agent Handoff | WorkloadAware | 69.42 | 14.4058 | 30 | 0 | 0 | 0 | 0 |
| Scenario E — Long Session Cont | AlwaysFast | 15.5 | 64.5161 | 25 | 35 | 0 | 0 | 0 |
| Scenario E — Long Session Cont | AlwaysSafe | 426.5 | 2.3447 | 60 | 0 | 0 | 0 | 0 |
| Scenario E — Long Session Cont | RecomputeHiRisk | 15.5 | 64.5161 | 25 | 35 | 0 | 0 | 0 |
| Scenario E — Long Session Cont | AlwaysRecall | 336.33 | 2.9732 | 25 | 35 | 0 | 0 | 0 |
| Scenario E — Long Session Cont | GenericSched | 285.17 | 3.5067 | 60 | 0 | 0 | 0 | 0 |
| Scenario E — Long Session Cont | WorkloadAware | 15.5 | 64.5161 | 25 | 35 | 0 | 0 | 35 |
| Scenario F — Runtime Node Qual | AlwaysFast | 202.69 | 4.9336 | 20 | 0 | 0 | 0 | 0 |
| Scenario F — Runtime Node Qual | AlwaysSafe | 202.69 | 4.9336 | 20 | 0 | 0 | 0 | 0 |
| Scenario F — Runtime Node Qual | RecomputeHiRisk | 1446.12 | 0.6915 | 20 | 0 | 0 | 0 | 0 |
| Scenario F — Runtime Node Qual | AlwaysRecall | 1221.59 | 0.8186 | 7 | 2 | 11 | 0 | 0 |
| Scenario F — Runtime Node Qual | GenericSched | 202.69 | 4.9336 | 20 | 0 | 0 | 0 | 0 |
| Scenario F — Runtime Node Qual | WorkloadAware | 202.69 | 4.9336 | 20 | 0 | 0 | 0 | 0 |
| Scenario G — UI Review / Desig | AlwaysFast | 2.1 | 476.1905 | 20 | 0 | 0 | 0 | 0 |
| Scenario G — UI Review / Desig | AlwaysSafe | 249.0 | 4.0161 | 20 | 0 | 0 | 0 | 0 |
| Scenario G — UI Review / Desig | RecomputeHiRisk | 2.1 | 476.1905 | 20 | 0 | 0 | 0 | 0 |
| Scenario G — UI Review / Desig | AlwaysRecall | 220.0 | 4.5455 | 20 | 0 | 0 | 0 | 0 |
| Scenario G — UI Review / Desig | GenericSched | 2.1 | 476.1905 | 20 | 0 | 0 | 0 | 0 |
| Scenario G — UI Review / Desig | WorkloadAware | 2.1 | 476.1905 | 20 | 0 | 0 | 0 | 0 |
| Scenario H — Parallel Mixed Wo | AlwaysFast | 206.19 | 4.8499 | 35 | 13 | 77 | 0 | 0 |
| Scenario H — Parallel Mixed Wo | AlwaysSafe | 342.1 | 2.9231 | 125 | 0 | 0 | 0 | 0 |
| Scenario H — Parallel Mixed Wo | RecomputeHiRisk | 2090.2 | 0.4784 | 100 | 25 | 0 | 0 | 0 |
| Scenario H — Parallel Mixed Wo | AlwaysRecall | 792.44 | 1.2619 | 35 | 10 | 80 | 0 | 0 |
| Scenario H — Parallel Mixed Wo | GenericSched | 323.58 | 3.0904 | 75 | 10 | 40 | 0 | 0 |
| Scenario H — Parallel Mixed Wo | WorkloadAware | 269.78 | 3.7068 | 121 | 4 | 0 | 0 | 4 |


## Generic Scheduler vs Workload-Aware Optimizer


### Scenario A — Sprint Planning

- **Generic scheduler latency:** 2.05 ms
- **Workload-aware latency:** 2.05 ms
- **Latency change:** 0.0%
- **Generic governance:** safe=15, warning=0, revalidate=0, blocked=0
- **Workload-aware governance:** safe=15, warning=0, revalidate=0, blocked=0
- **Stale-cache regression improved:** False
- **Generic hard blocks/revalidations:** 0
- **Workload-aware hard blocks/revalidations:** 0

### Scenario B — Sprint Closeout

- **Generic scheduler latency:** 312.5 ms
- **Workload-aware latency:** 328.78 ms
- **Latency change:** -5.21%
- **Generic governance:** safe=15, warning=2, revalidate=8, blocked=0
- **Workload-aware governance:** safe=25, warning=0, revalidate=0, blocked=0
- **Stale-cache regression improved:** False
- **Generic hard blocks/revalidations:** 8
- **Workload-aware hard blocks/revalidations:** 0

### Scenario C — Receipt Generation

- **Generic scheduler latency:** 272.36 ms
- **Workload-aware latency:** 218.31 ms
- **Latency change:** 19.85%
- **Generic governance:** safe=10, warning=2, revalidate=8, blocked=0
- **Workload-aware governance:** safe=20, warning=0, revalidate=0, blocked=0
- **Stale-cache regression improved:** True
- **Generic hard blocks/revalidations:** 8
- **Workload-aware hard blocks/revalidations:** 0

### Scenario D — Agent Handoff

- **Generic scheduler latency:** 69.42 ms
- **Workload-aware latency:** 69.42 ms
- **Latency change:** 0.0%
- **Generic governance:** safe=30, warning=0, revalidate=0, blocked=0
- **Workload-aware governance:** safe=30, warning=0, revalidate=0, blocked=0
- **Stale-cache regression improved:** False
- **Generic hard blocks/revalidations:** 0
- **Workload-aware hard blocks/revalidations:** 0

### Scenario E — Long Session Continuation

- **Generic scheduler latency:** 285.17 ms
- **Workload-aware latency:** 15.5 ms
- **Latency change:** 94.56%
- **Generic governance:** safe=60, warning=0, revalidate=0, blocked=0
- **Workload-aware governance:** safe=25, warning=35, revalidate=0, blocked=0
- **Stale-cache regression improved:** True
- **Generic hard blocks/revalidations:** 0
- **Workload-aware hard blocks/revalidations:** 0

### Scenario F — Runtime Node Qualification

- **Generic scheduler latency:** 202.69 ms
- **Workload-aware latency:** 202.69 ms
- **Latency change:** 0.0%
- **Generic governance:** safe=20, warning=0, revalidate=0, blocked=0
- **Workload-aware governance:** safe=20, warning=0, revalidate=0, blocked=0
- **Stale-cache regression improved:** False
- **Generic hard blocks/revalidations:** 0
- **Workload-aware hard blocks/revalidations:** 0

### Scenario G — UI Review / Design Planning

- **Generic scheduler latency:** 2.1 ms
- **Workload-aware latency:** 2.1 ms
- **Latency change:** 0.0%
- **Generic governance:** safe=20, warning=0, revalidate=0, blocked=0
- **Workload-aware governance:** safe=20, warning=0, revalidate=0, blocked=0
- **Stale-cache regression improved:** False
- **Generic hard blocks/revalidations:** 0
- **Workload-aware hard blocks/revalidations:** 0

### Scenario H — Parallel Mixed Workload

- **Generic scheduler latency:** 323.58 ms
- **Workload-aware latency:** 269.78 ms
- **Latency change:** 16.63%
- **Generic governance:** safe=75, warning=10, revalidate=40, blocked=0
- **Workload-aware governance:** safe=121, warning=4, revalidate=0, blocked=0
- **Stale-cache regression improved:** True
- **Generic hard blocks/revalidations:** 40
- **Workload-aware hard blocks/revalidations:** 0

---

## Per-Scenario Detailed Results


### Scenario A — Sprint Planning

**Workload type:** `sprint_planning` | **Hardware:** `current_mac_coordinator`

#### Strategy Comparison

| Strategy | Latency (ms) | Throughput | Safe | Warn | Reval | Blocked | Perf↓ |
|----------|-------------:|-----------:|-----:|-----:|------:|--------:|------:|
| Always Fastest Context Pa | 2.05 | 487.8049 | 15 | 0 | 0 | 0 | 0 |
| Always Safest Context Pat | 278.0 | 3.5971 | 15 | 0 | 0 | 0 | 0 |
| Always Recompute for High | 2.05 | 487.8049 | 15 | 0 | 0 | 0 | 0 |
| Always Compressed Recall  | 236.0 | 4.2373 | 15 | 0 | 0 | 0 | 0 |
| Prior Generic Scheduler ( | 2.05 | 487.8049 | 15 | 0 | 0 | 0 | 0 |
| Librarian Workload-Aware  | 2.05 | 487.8049 | 15 | 0 | 0 | 0 | 0 |

#### Workload-Aware Route Selection

**Route usage:** {
  "recent_turn_window": 15
}
**Governance:** {
  "safe": 15,
  "warning": 0,
  "requires_revalidation": 0,
  "blocked": 0
}
**Performance sacrificed for evidence:** 0 turns


### Scenario B — Sprint Closeout

**Workload type:** `sprint_closeout` | **Hardware:** `current_mac_coordinator`

#### Strategy Comparison

| Strategy | Latency (ms) | Throughput | Safe | Warn | Reval | Blocked | Perf↓ |
|----------|-------------:|-----------:|-----:|-----:|------:|--------:|------:|
| Always Fastest Context Pa | 298.4 | 3.3512 | 7 | 5 | 13 | 0 | 0 |
| Always Safest Context Pat | 377.0 | 2.6525 | 25 | 0 | 0 | 0 | 0 |
| Always Recompute for High | 2262.0 | 0.4421 | 25 | 0 | 0 | 0 | 0 |
| Always Compressed Recall  | 1197.8 | 0.8349 | 7 | 2 | 16 | 0 | 0 |
| Prior Generic Scheduler ( | 312.5 | 3.2 | 15 | 2 | 8 | 0 | 0 |
| Librarian Workload-Aware  | 328.78 | 3.0415 | 25 | 0 | 0 | 0 | 0 |

#### Workload-Aware Route Selection

**Route usage:** {
  "ram_cache": 7,
  "canonical_evidence_read": 18
}
**Governance:** {
  "safe": 25,
  "warning": 0,
  "requires_revalidation": 0,
  "blocked": 0
}
**Performance sacrificed for evidence:** 0 turns


### Scenario C — Receipt Generation

**Workload type:** `receipt_generation` | **Hardware:** `current_mac_coordinator`

#### Strategy Comparison

| Strategy | Latency (ms) | Throughput | Safe | Warn | Reval | Blocked | Perf↓ |
|----------|-------------:|-----------:|-----:|-----:|------:|--------:|------:|
| Always Fastest Context Pa | 292.85 | 3.4147 | 7 | 2 | 11 | 0 | 0 |
| Always Safest Context Pat | 226.5 | 4.415 | 20 | 0 | 0 | 0 | 0 |
| Always Recompute for High | 1509.0 | 0.6627 | 20 | 0 | 0 | 0 | 0 |
| Always Compressed Recall  | 1092.75 | 0.9151 | 7 | 2 | 11 | 0 | 0 |
| Prior Generic Scheduler ( | 272.36 | 3.6716 | 10 | 2 | 8 | 0 | 0 |
| Librarian Workload-Aware  | 218.31 | 4.5806 | 20 | 0 | 0 | 0 | 0 |

#### Workload-Aware Route Selection

**Route usage:** {
  "ram_cache": 7,
  "canonical_evidence_read": 13
}
**Governance:** {
  "safe": 20,
  "warning": 0,
  "requires_revalidation": 0,
  "blocked": 0
}
**Performance sacrificed for evidence:** 0 turns


### Scenario D — Agent Handoff

**Workload type:** `agent_handoff` | **Hardware:** `current_mac_coordinator`

#### Strategy Comparison

| Strategy | Latency (ms) | Throughput | Safe | Warn | Reval | Blocked | Perf↓ |
|----------|-------------:|-----------:|-----:|-----:|------:|--------:|------:|
| Always Fastest Context Pa | 69.42 | 14.4058 | 30 | 0 | 0 | 0 | 0 |
| Always Safest Context Pat | 353.0 | 2.8329 | 30 | 0 | 0 | 0 | 0 |
| Always Recompute for High | 69.42 | 14.4058 | 30 | 0 | 0 | 0 | 0 |
| Always Compressed Recall  | 544.87 | 1.8353 | 30 | 0 | 0 | 0 | 0 |
| Prior Generic Scheduler ( | 69.42 | 14.4058 | 30 | 0 | 0 | 0 | 0 |
| Librarian Workload-Aware  | 69.42 | 14.4058 | 30 | 0 | 0 | 0 | 0 |

#### Workload-Aware Route Selection

**Route usage:** {
  "ram_cache": 10,
  "recent_turn_window": 20
}
**Governance:** {
  "safe": 30,
  "warning": 0,
  "requires_revalidation": 0,
  "blocked": 0
}
**Performance sacrificed for evidence:** 0 turns


### Scenario E — Long Session Continuation

**Workload type:** `long_session_continuation` | **Hardware:** `current_mac_coordinator`

#### Strategy Comparison

| Strategy | Latency (ms) | Throughput | Safe | Warn | Reval | Blocked | Perf↓ |
|----------|-------------:|-----------:|-----:|-----:|------:|--------:|------:|
| Always Fastest Context Pa | 15.5 | 64.5161 | 25 | 35 | 0 | 0 | 0 |
| Always Safest Context Pat | 426.5 | 2.3447 | 60 | 0 | 0 | 0 | 0 |
| Always Recompute for High | 15.5 | 64.5161 | 25 | 35 | 0 | 0 | 0 |
| Always Compressed Recall  | 336.33 | 2.9732 | 25 | 35 | 0 | 0 | 0 |
| Prior Generic Scheduler ( | 285.17 | 3.5067 | 60 | 0 | 0 | 0 | 0 |
| Librarian Workload-Aware  | 15.5 | 64.5161 | 25 | 35 | 0 | 0 | 35 |

#### Workload-Aware Route Selection

**Route usage:** {
  "recent_turn_window": 60
}
**Governance:** {
  "safe": 25,
  "warning": 35,
  "requires_revalidation": 0,
  "blocked": 0
}
**Performance sacrificed for evidence:** 35 turns


### Scenario F — Runtime Node Qualification

**Workload type:** `runtime_node_qualification` | **Hardware:** `current_windows_runtime_node`

#### Strategy Comparison

| Strategy | Latency (ms) | Throughput | Safe | Warn | Reval | Blocked | Perf↓ |
|----------|-------------:|-----------:|-----:|-----:|------:|--------:|------:|
| Always Fastest Context Pa | 202.69 | 4.9336 | 20 | 0 | 0 | 0 | 0 |
| Always Safest Context Pat | 202.69 | 4.9336 | 20 | 0 | 0 | 0 | 0 |
| Always Recompute for High | 1446.12 | 0.6915 | 20 | 0 | 0 | 0 | 0 |
| Always Compressed Recall  | 1221.59 | 0.8186 | 7 | 2 | 11 | 0 | 0 |
| Prior Generic Scheduler ( | 202.69 | 4.9336 | 20 | 0 | 0 | 0 | 0 |
| Librarian Workload-Aware  | 202.69 | 4.9336 | 20 | 0 | 0 | 0 | 0 |

#### Workload-Aware Route Selection

**Route usage:** {
  "canonical_evidence_read": 20
}
**Governance:** {
  "safe": 20,
  "warning": 0,
  "requires_revalidation": 0,
  "blocked": 0
}
**Performance sacrificed for evidence:** 0 turns


### Scenario G — UI Review / Design Planning

**Workload type:** `ui_review_or_design_planning` | **Hardware:** `current_mac_coordinator`

#### Strategy Comparison

| Strategy | Latency (ms) | Throughput | Safe | Warn | Reval | Blocked | Perf↓ |
|----------|-------------:|-----------:|-----:|-----:|------:|--------:|------:|
| Always Fastest Context Pa | 2.1 | 476.1905 | 20 | 0 | 0 | 0 | 0 |
| Always Safest Context Pat | 249.0 | 4.0161 | 20 | 0 | 0 | 0 | 0 |
| Always Recompute for High | 2.1 | 476.1905 | 20 | 0 | 0 | 0 | 0 |
| Always Compressed Recall  | 220.0 | 4.5455 | 20 | 0 | 0 | 0 | 0 |
| Prior Generic Scheduler ( | 2.1 | 476.1905 | 20 | 0 | 0 | 0 | 0 |
| Librarian Workload-Aware  | 2.1 | 476.1905 | 20 | 0 | 0 | 0 | 0 |

#### Workload-Aware Route Selection

**Route usage:** {
  "recent_turn_window": 20
}
**Governance:** {
  "safe": 20,
  "warning": 0,
  "requires_revalidation": 0,
  "blocked": 0
}
**Performance sacrificed for evidence:** 0 turns


### Scenario H — Parallel Mixed Workload

**Workload type:** `sprint_closeout` | **Hardware:** `current_mac_coordinator`

#### Strategy Comparison

| Strategy | Latency (ms) | Throughput | Safe | Warn | Reval | Blocked | Perf↓ |
|----------|-------------:|-----------:|-----:|-----:|------:|--------:|------:|
| Always Fastest Context Pa | 206.19 | 4.8499 | 35 | 13 | 77 | 0 | 0 |
| Always Safest Context Pat | 342.1 | 2.9231 | 125 | 0 | 0 | 0 | 0 |
| Always Recompute for High | 2090.2 | 0.4784 | 100 | 25 | 0 | 0 | 0 |
| Always Compressed Recall  | 792.44 | 1.2619 | 35 | 10 | 80 | 0 | 0 |
| Prior Generic Scheduler ( | 323.58 | 3.0904 | 75 | 10 | 40 | 0 | 0 |
| Librarian Workload-Aware  | 269.78 | 3.7068 | 121 | 4 | 0 | 0 | 4 |

#### Workload-Aware Route Selection

**Route usage:** {
  "ram_cache": 39,
  "canonical_evidence_read": 86
}
**Governance:** {
  "safe": 121,
  "warning": 4,
  "requires_revalidation": 0,
  "blocked": 0
}
**Performance sacrificed for evidence:** 4 turns


---

## Scenario C (Stale-Cache Governance) Improvement Analysis

- **Generic scheduler latency:** 272.36 ms
- **Workload-aware latency:** 218.31 ms
- **Generic hard blocks/revalidations:** 8
- **Workload-aware hard blocks/revalidations:** 0
- **Result:** Scenario C stale-cache regression is **improved** — the workload-aware optimizer uses graduated penalties instead of hard blocking.

---

## Summary

- Scenarios where workload-aware improved over generic: **3**
- Scenarios where workload-aware regressed: **0**
- Scenarios where roughly tied: **5**

### Where the workload-aware optimizer helped

- **Scenario C — Receipt Generation**: 19.85% latency improvement
- **Scenario E — Long Session Continuation**: 94.56% latency improvement
- **Scenario H — Parallel Mixed Workload**: 16.63% latency improvement

### Where governance requires performance sacrifice

- **Scenario E — Long Session Continuation**: 35 turns where performance was sacrificed for evidence quality
- **Scenario H — Parallel Mixed Workload**: 4 turns where performance was sacrificed for evidence quality

---
