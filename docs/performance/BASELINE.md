# Performance Baseline

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  

---

## 1. Purpose

Define the performance baseline for the Windows Runtime Node. This document captures the current performance characteristics before any optimization work begins.

---

## 2. Hardware Baseline

| Component | Specification |
|-----------|---------------|
| CPU | Intel i5-3570K (4 cores, 3.4GHz) |
| GPU | AMD RX 570 (4GB VRAM, Vulkan) |
| RAM | 16GB DDR3 |
| Storage | 500GB SSD |
| OS | Windows 10 Pro Workstations |

---

## 3. Model Performance Baseline

### MiniCPM5 1B Q4_K_M

| Metric | Value | Notes |
|--------|-------|-------|
| Model size | ~600MB | |
| Quantization | Q4_K_M | |
| GPU layers | TBD | Requires measurement |
| VRAM usage | TBD | Requires measurement |
| Token throughput | TBD | Requires measurement |
| First-token latency | TBD | Requires measurement |
| Context allocation | TBD | 4K, 8K, 16K |

### MiniCPM5 1B Q8_0

| Metric | Value | Notes |
|--------|-------|-------|
| Model size | ~1GB | |
| Quantization | Q8_0 | |
| GPU layers | TBD | Requires measurement |
| VRAM usage | TBD | Requires measurement |
| Token throughput | TBD | Requires measurement |
| First-token latency | TBD | Requires measurement |
| Context allocation | TBD | 4K, 8K, 16K |

---

## 4. System Performance Baseline

| Metric | Value | Notes |
|--------|-------|-------|
| Node startup time | TBD | From service start to health OK |
| Model load time | TBD | From select to health OK |
| Model unload time | TBD | From stop to process exit |
| VRAM release time | TBD | From unload to VRAM available |
| Evidence write time | TBD | Per event |
| Health check response | TBD | GET /health latency |

---

## 5. Performance Constraints

| Constraint | Limit | Notes |
|------------|-------|-------|
| VRAM reserve | 400-700 MB | On 4GB RX 570 |
| Single concurrent request | 1 | By design (serialized) |
| Request timeout | 120s | Configurable |
| Context window | TBD | Depends on model and VRAM |

---

## 6. Measurement Methodology

- All measurements taken after warm-up (3+ inference rounds)
- Measurements averaged over 10+ runs
- Context sizes: 4K, 8K, 16K
- GPU layers: progressive (full offload, partial offload, CPU only)

---

## 7. References

- ADR-PLATFORM-002 — Platform Lifecycle
- BENCHMARK-PLAN.md — Benchmark plan
- QUALIFICATION.md — Qualification documentation
- DEPENDENCY-MAP.md — Hardware dependencies
