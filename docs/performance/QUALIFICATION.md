# Qualification

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  

---

## 1. Purpose

Define the qualification process for the Windows Runtime Node. Qualification measures and validates that the Node meets performance, reliability, and security requirements.

---

## 2. Qualification Types

### 2.1 Hardware Qualification

| Check | Method | Pass Criteria |
|-------|--------|---------------|
| GPU presence | Vulkan device discovery | GPU detected |
| GPU capabilities | Vulkan API query | Vulkan 1.2+ |
| VRAM | Vulkan memory query | Minimum 4GB |
| CPU | System query | x64, 4+ cores |
| RAM | System query | 16GB+ |
| Storage | File system query | 10GB+ free |

### 2.2 Runtime Qualification

| Check | Method | Pass Criteria |
|-------|--------|---------------|
| llama.cpp | Binary version check | Version captured |
| SHA-256 | Binary hash | Hash verified |
| Vulkan | Runtime device test | Device enumerated |
| Inference | Test query | Model loads and responds |

### 2.3 Model Qualification

| Check | Method | Pass Criteria |
|-------|--------|---------------|
| Load | Load test | Model loads within time limit |
| Inference | Inference test | Response received within timeout |
| Unload | Unload test | Process exits, VRAM released |
| Context | Context allocation | 4K+, 8K+, 16K context window |
| VRAM | VRAM measurement | Within 4GB limit |
| Throughput | Token benchmark | Tokens per second measured |

---

## 3. Qualification Evidence

Every qualification produces evidence:

| Qualification | Evidence Type | Contents |
|---------------|--------------|----------|
| Hardware | `hardware_qualified` | Device info, VRAM, driver version |
| Runtime | `runtime_qualified` | Binary version, hash, Vulkan status |
| Model | `model_qualified` | Load time, VRAM, throughput, context |

---

## 4. Qualification Schedule

| Trigger | Qualification | Action |
|---------|---------------|--------|
| Initial installation | Full | All checks |
| Hardware change | Hardware + Model | Requalification |
| Binary update | Runtime | Binary verification |
| Model addition | Model | Model-specific checks |
| Upgrade | Full | Compare against baseline |

---

## 5. References

- BASELINE.md — Performance baseline
- BENCHMARK-PLAN.md — Benchmark plan
- ADR-PLATFORM-002 — Platform Lifecycle
- DATA-FLOW.md — Data flow documentation
