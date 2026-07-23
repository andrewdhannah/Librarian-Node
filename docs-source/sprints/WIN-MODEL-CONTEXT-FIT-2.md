# Sprint: WIN-MODEL-CONTEXT-FIT-2
## Goal: Determine safe context-size limits for Windows RX 570 runtime profiles.

### Environment
- **GPU**: AMD Radeon RX 570 (4 GB Polaris)
- **Runtime Node**: `LibrarianRunTimeNode` (Windows Service)
- **Router Port**: `9130`
- **Baseline Context**: `1024`
- **Baseline NGL**: `99`

### Test Matrix
For each profile, test context sizes: `1024` (Baseline), `2048`, `3072`, `4096`.

| Profile | 1024 | 2048 | 3072 | 4096 | Safe Cap |
| :--- | :---: | :---: | :---: | :---: | :---: |
| phi-4 | PASS | PASS | PASS | PASS | 4096 |
| qwen-coder | PASS | PASS | PASS | PASS | 4096 |
| llama-3.2 | OOM | OOM | OOM | OOM | 1024* |
| qwen3 | OOM | OOM | OOM | OOM | 1024* |
| gemma-3 | OOM | OOM | OOM | OOM | 1024* |

\* Baseline 1024 is a **legacy/default value**, not a verified safe cap at ngl=99. 
These models OOM'd at load even at context=1024. A reduced-offload fit test 
(e.g., REDUCED-OFFLOAD-FIT-1) is needed to determine actual safe operating points.

### Test Method
1. Start `LibrarianRunTimeNode` (or manual router for debugging).
2. For each profile/context:
   - Update `config/model-profiles.json` with target context.
   - Trigger `/backend/restart` for the profile.
   - Send chat request: "Reply with OK only."
   - Record: Launch success, Response success, Crash/OOM, Cleanup.
3. Stop service and verify no orphans.

### ⚠ Important Discovery (FIT-EVIDENCE-RECONCILE-1)

The router reads `model-profiles.json` **at startup only** and caches profile
data in memory. Changing the config file while the router is running has no
effect on in-memory profile data. The test method above (step 2) writes changes
to disk and calls `/backend/restart`, but the restart re-launches the backend
using the **stale in-memory profile values**, not the updated config file.

**Impact**: The PASS results for `phi-4` and `qwen-coder` may have used the
originally intended context values (since those two profiles were already
configured at ngl=99, context=4096 in the starting config and remained at
those values). However, the evidence for these profiles is not methodologically
sound — it was not produced with a restart-per-config-change procedure.

**Correction**: `FIT-EVIDENCE-RECONCILE-1` retested phi-4 and qwen-coder using
the correct restart-per-config-change method. The new evidence files are at:
- `fixtures/windows-runtime-node/model-fit/evidence/phi-4-ngl99.json`
- `fixtures/windows-runtime-node/model-fit/evidence/qwen-coder-ngl99.json`

The OOM results for llama-3.2, qwen3, and gemma-3 are **not affected** by the
stale-cache issue because:
- The OOM at ngl=99 was consistent regardless of in-memory vs on-disk config.
- These profiles were re-verified in `REDUCED-OFFLOAD-FIT-1` using the correct
  restart-per-config-change method.

### Future Use

The `scripts/test-context-fit.ps1` script is **deprecated** because it uses
the stale-cache approach. Use `scripts/test-reduced-offload-fit.ps1` (which
correctly restarts the router between config changes) instead.

### Results
- **phi-4**: Reported stable up to 4096 (originally tested with stale-cache method).
  - **CORRECTED in FIT-EVIDENCE-RECONCILE-1**: Confirmed stable at ngl=99, context=4096
    using restart-per-config-change method. ✅
- **qwen-coder**: Reported stable up to 4096 (originally tested with stale-cache method).
  - **CORRECTED in FIT-EVIDENCE-RECONCILE-1**: Confirmed stable at ngl=99, context=4096
    using restart-per-config-change method. ✅
- **llama-3.2 / qwen3 / gemma-3**: All encountered `ErrorOutOfDeviceMemory` (OOM) when
  attempting to load with `ngl=99` and `context=1024`. These OOM results are **not affected**
  by the stale-cache issue (OOM happened consistently regardless of in-memory vs on-disk
  config). These profiles were re-verified in `REDUCED-OFFLOAD-FIT-1` using the correct
  restart-per-config-change method.

### Final Recommendations (as of FIT-EVIDENCE-RECONCILE-1)
- **phi-4**: `context: 4096`, `ngl: 99` — verified with corrected method.
- **qwen-coder**: `context: 4096`, `ngl: 99` — verified with corrected method.
- **llama-3.2 / qwen3 / gemma-3**: Reduced to `ngl: 80`, then `context: 4096` — verified
  in REDUCED-OFFLOAD-FIT-1 with corrected method.
- All 5 profiles now have methodologically sound evidence.
