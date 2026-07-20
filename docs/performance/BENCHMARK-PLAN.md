# Benchmark Plan

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  

---

## 1. Purpose

Define the benchmark plan for the Windows Runtime Node. Benchmarks measure performance characteristics and produce evidence for qualification.

---

## 2. Benchmark Types

### 2.1 Model Benchmarks

| Benchmark | Metric | Context Sizes |
|-----------|--------|---------------|
| Load time | Time from select to health OK | N/A |
| Token generation | Tokens per second | 4K, 8K, 16K |
| First-token latency | Time to first token | 4K, 8K, 16K |
| VRAM usage | MB allocated | 4K, 8K, 16K |
| Context allocation | Maximum context window | Progressive |

### 2.2 System Benchmarks

| Benchmark | Metric | Description |
|-----------|--------|-------------|
| Startup time | Seconds | Service start to health OK |
| Evidence write | Milliseconds per event | Evidence write latency |
| Export time | Seconds per bundle | Evidence export latency |
| Health check | Milliseconds | HTTP health endpoint latency |

### 2.3 Hardware Benchmarks

| Benchmark | Metric | Description |
|-----------|--------|-------------|
| GPU compute | GFLOPS | Vulkan compute performance |
| Memory bandwidth | GB/s | System RAM bandwidth |
| Disk I/O | MB/s | Sequential and random I/O |

---

## 3. Benchmark Protocol

1. **Preparation** — Ensure Node is in READY state
2. **Warm-up** — 3+ inference rounds before measurement
3. **Measurement** — 10+ runs per benchmark
4. **Recording** — Results recorded in evidence store
5. **Analysis** — Min, max, mean, median, p95, p99

---

## 4. Benchmark Schedule

| Frequency | Benchmarks | Purpose |
|-----------|------------|---------|
| Per install | System benchmarks | Baseline verification |
| Per model | Model benchmarks | Model qualification |
| Per hardware change | Hardware benchmarks | Requalification |
| Per upgrade | All benchmarks | Regression detection |

---

## 5. References

- BASELINE.md — Performance baseline
- QUALIFICATION.md — Qualification documentation
- DATA-FLOW.md — Data flow documentation
