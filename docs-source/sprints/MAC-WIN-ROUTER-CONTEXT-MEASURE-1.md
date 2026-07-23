# MAC/WIN-ROUTER-CONTEXT-MEASURE-1 — Sprint Closeout

## Sprint Details

| Field | Value |
|-------|-------|
| Sprint ID | MAC/WIN-ROUTER-CONTEXT-MEASURE-1 |
| Type | Measurement / calibration |
| Status | **Complete — Promote** |
| Starting HEAD | f5d09f0 |
| Final HEAD | f5d09f0 |
| Platform | Windows 10 (AMD64) |
| Date | 2026-06-28 |

## What Was Done

Measured real context-movement costs on the actual Librarian hardware stack:

1. **File I/O** — warm reads of 32K-token payloads take ~0.28ms (35x faster than simulator assumed)
2. **JSON processing** — parse/serialize of 8K-token payloads takes ~0.05-0.11ms (negligible)
3. **Recall packets** — serialize+compress+decompress for 32K tokens costs ~0.4ms total
4. **Canonical evidence** — git status costs ~71ms, git rev-parse costs ~55ms (2-3x higher than assumed)
5. **Runtime health** — stopped nodes cost ~4 seconds (TCP timeout)
6. **LAN round-trip** — unreachable/refused connections cost ~4 seconds
7. **Small append** — serialize+write+read for 429 tokens costs ~0.8ms
8. **Large context** — full transfer pipeline for 32K tokens costs ~1.9ms
9. **Degraded handling** — all failure modes cost ~4 seconds (TCP timeout dominates)

## What Was NOT Changed

- Production router behavior: **UNCHANGED**
- Model execution behavior: **UNCHANGED**
- Runtime-node behavior: **UNCHANGED**
- No cache engine added
- No GPU/RDMA/KV-cache claims made

## Acceptance Criteria Check

| # | Criterion | Status |
|---|-----------|--------|
| 1 | No production router behavior changed | PASS |
| 2 | No model execution behavior changed | PASS |
| 3 | Measurements in machine-readable JSON | PASS — `reports/router-context-measure-results.json` |
| 4 | Calibrated hardware profiles produced | PASS — `config/measured_hardware_profiles.json` |
| 5 | Cold and warm paths captured | PASS — file I/O measured cold and warm |
| 6 | Small append and large context measured | PASS — 429tok, 32K, 64K measured |
| 7 | Runtime health/round-trip measured | PASS — all endpoints measured (expected failures) |
| 8 | Degraded/unavailable node measured | PASS — stopped, unavailable, wrong port measured |
| 9 | Synthetic assumptions compared | PASS — 5 comparisons in Section 4 of report |
| 10 | Report recommends prototype | PASS — recommends MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1 |
| 11 | Service/process cleanup verified | PASS — no orphan processes, ports free |
| 12 | No GPU/RDMA/KV-cache claims | PASS |
| 13 | Working tree documented | PASS — untracked measurement files only |

## Files Created

| File | Type |
|------|------|
| `scripts/measurements/measure-router-context.py` | Measurement harness |
| `scripts/measurements/measure-fast.py` | Optimized measurement script |
| `scripts/measurements/measure-router-context.ps1` | PowerShell wrapper |
| `reports/router-context-measure-results.json` | Machine-readable results |
| `config/measured_hardware_profiles.json` | Calibrated profiles |
| `reports/MAC-WIN-ROUTER-CONTEXT-MEASURE-1.md` | Human-readable report |
| `docs/sprints/MAC-WIN-ROUTER-CONTEXT-MEASURE-1.md` | This closeout doc |

## State Verification

```
Starting HEAD: f5d09f0
Final HEAD: f5d09f0
Git status: clean (measurement files are untracked)
Service: LibrarianRunTimeNode — Stopped / Manual
Router processes: 0
Orphan processes: 0
Ports free: 8080, 9120-9124 (all free)
```

## Result Classification

### **Promote**

Measurements are clean, consistent, and sufficient to calibrate the optimizer and proceed to a prototype sprint.

## Recommended Next Sprint

```
MAC/WIN-ROUTER-CONTEXT-PROTOTYPE-1
```

## Key Calibration Changes for Next Sprint

1. Reduce SSD cache base latency from 10ms to ~0.3ms (warm path)
2. Increase canonical evidence read cost from 25ms to ~65ms
3. Add 4,000ms degraded-node penalty
4. Split recall packet cost into local (~0.4ms) + network (variable)
5. Add Mac measurements when hardware is available
