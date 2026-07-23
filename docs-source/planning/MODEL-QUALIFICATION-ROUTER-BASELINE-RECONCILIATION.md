# MODEL-QUALIFICATION-ROUTER-BASELINE-RECONCILIATION

**Sprint:** MODEL-QUALIFICATION-ROUTER-BASELINE-INTEGRATION-PLAN-1
**Gate:** MQR-BI-1, MQR-BI-2, MQR-BI-3

---

## Purpose

Direct mapping between the Model Qualification Router planning pack's conceptual structures and the sealed Big Pickle Windows runtime primitives.

The planning pack predates the sealed runtime. This report identifies what already exists, what must be created on the Mac side, what bridge records are required, and what planning-pack assumptions are superseded by the implemented reality.

---

## 1. Planning Pack Concepts → Sealed Runtime Mapping

### 1.1 Execution Profile

**Planning pack definition:** How an exact model artifact executes on a specific system/runtime configuration. Contains identity (model, system, runtime, backend, quant) and measurements (load time, VRAM, throughput, stability).

**Sealed Windows reality:** Three tables partially cover this:

| Execution Profile Field | Windows Table | Column | Status |
|------------------------|---------------|--------|--------|
| execution_profile_id | runtime_profiles | profile_id | **PARTIAL** — profile_id exists but is per-model, not per-artifact+system |
| model_artifact_id | runtime_profiles | model_id → local_models | **YES** via FK |
| system_profile_id | hardware_profiles | hw_profile_id | **YES** — exists but not linked to runtime_profiles |
| runtime name/revision | local_models | source_repository, sha256 | **PARTIAL** — no explicit runtime executable identity in profile |
| backend | runtime_profiles | device_backend | **YES** |
| quant | local_models | quantization | **YES** |
| measurements | runtime_profiles | measured_vram_mb, measured_tokens_per_sec | **PARTIAL** — only two measurements, no load time, no throughput breakdown |

**Gap:** The planning pack's execution profile is richer than the sealed runtime_profile. The Windows runtime_profile is a minimal deployment configuration. The full execution profile (with Pareto sweeps, stability, variance) must be a Mac-side structure that references the Windows runtime_profile as input.

**Decision:** runtime_profiles remain Windows-owned deployment configs. execution_profiles become Mac-owned qualification records that consume runtime_profiles.

### 1.2 Model Artifact Identity

**Planning pack definition:** Exact identity chain including model family, repository, revision, artifact filename, SHA-256, GGUF metadata, quantization, tokenizer/chat-template identity, license, source URI.

**Sealed Windows reality:**

| Identity Field | Windows Table | Column | Status |
|---------------|---------------|--------|--------|
| model_id | local_models | model_id | **YES** |
| display_name | local_models | display_name | **YES** |
| family | local_models | family | **YES** |
| source_repository | local_models | source_repository | **YES** |
| filename | local_models | filename | **YES** |
| quantization | local_models | quantization | **YES** |
| sha256 | local_models | sha256 | **YES** |
| file_size_bytes | local_models | file_size_bytes | **YES** |
| GGUF metadata | — | — | **MISSING** |
| chat template identity | — | — | **MISSING** |
| license metadata | — | — | **MISSING** |

**Gap:** Windows local_models captures artifact identity sufficient for execution binding but not for full qualification identity. GGUF metadata, chat template identity, and license are not stored.

**Decision:** The qualification identity contract must extend local_models with a Mac-side model_identity_record that adds: gguf_metadata_hash, chat_template_id, license SPDX, and qualification scope. This is a Mac canonical table, not a Windows alteration.

### 1.3 Capability Manifest

**Planning pack definition:** Machine-readable evidence-to-routing bridge. Contains manifest_id, model_artifact_id, protocol_version, status, roles (each with role, status, constraints, known_failures, evidence_run_ids), owner_decision_id.

**Sealed Windows reality:** No equivalent exists. The planning pack correctly identifies this as a Mac-side structure.

**Decision:** capability_manifests is a new Mac canonical table. The Windows runtime has no role in capability assessment. The Q8 canary is preserved: Windows runtime_success events never populate this table.

### 1.4 Qualification Run

**Planning pack definition:** Execution of versioned work fixtures preserving fixture version, prompt, tools, workspace state, raw output, tool trace, final state, runtime telemetry, hashes.

**Sealed Windows reality:**

| Qualification Run Field | Windows Table | Column | Status |
|------------------------|---------------|--------|--------|
| run_id | runtime_runs | run_id | **YES** |
| lease_id | runtime_runs | lease_id → job_leases | **YES** |
| packet_id | runtime_runs | packet_id | **YES** (placeholder, currently unused) |
| input_tokens | runtime_runs | input_tokens | **YES** |
| output_tokens | runtime_runs | output_tokens | **YES** |
| load_duration_ms | runtime_runs | load_duration_ms | **YES** |
| generation_duration_ms | runtime_runs | generation_duration_ms | **YES** |
| exit_status | runtime_runs | exit_status | **YES** |
| fixture version | — | — | **MISSING** |
| raw output | — | — | **MISSING** |
| tool trace | — | — | **MISSING** |
| workspace state | — | — | **MISSING** |
| hashes | — | — | **MISSING** |

**Gap:** Windows runtime_runs capture execution lifecycle but not qualification-specific evidence. The qualification run is a superset that references the Windows run as its execution substrate.

**Decision:** qualification_runs is a new Mac-side table that references Windows runtime_runs via run_id. Windows owns execution; Mac owns qualification interpretation.

### 1.5 Runtime Run

**Planning pack definition:** Not explicitly named, but implied as the execution lifecycle record.

**Sealed Windows reality:** runtime_runs table exists with run_id, lease_id, packet_id, token metrics, timing, exit_status.

**Decision:** No change. runtime_runs remain Windows-owned. They are the execution evidence source for Mac-side qualification_runs.

### 1.6 Lifecycle Evidence

**Planning pack definition:** Append-only evidence chain for audit trail.

**Sealed Windows reality:** lifecycle_evidence table exists with 26+ event types, append-only, FK to lease_id and run_id.

**Decision:** No change. lifecycle_evidence remains Windows-owned. It provides the raw execution event chain that Mac-side qualification intake consumes.

### 1.7 Hardware Profile

**Planning pack definition:** System profile for the execution environment.

**Sealed Windows reality:** hardware_profiles table exists with hw_profile_id, device_name, vulkan_device, total_vram_mb, available_vram_mb, driver_version.

**Decision:** hardware_profiles remain Windows-owned. Mac-side system_profiles extend this with additional system context (OS, CPU, RAM) for cross-machine comparison.

### 1.8 Runtime Profile

**Planning pack definition:** Model+hardware deployment configuration.

**Sealed Windows reality:** runtime_profiles table exists with profile_id, model_id, device_backend, gpu_layers, context_tokens, estimated_vram_mb, measured_vram_mb, measured_tokens_per_sec, practical_context_tokens, profile_priority, enabled.

**Decision:** runtime_profiles remain Windows-owned. They are the deployment configs that the residency supervisor uses. Mac-side execution_profiles consume runtime_profiles as input.

---

## 2. Superseded Planning Pack Assumptions

| Planning Pack Assumption | Sealed Runtime Reality | Resolution |
|--------------------------|----------------------|------------|
| Execution profiler is a separate component | Windows runtime_profiles + lifecycle_evidence already capture execution data | Mac execution_profiler consumes Windows evidence; no separate Windows profiler needed |
| Model Intake Registry is a single structure | Windows local_models + Mac model_identity_record together cover intake | Split: Windows = installed inventory, Mac = qualification identity |
| Qualification Runner owns process lifecycle | Windows residency supervisor owns process lifecycle | Qualification Runner sends requests; Windows executes and returns evidence |
| Distributed evidence is the primary evidence path | Big Pickle has a sealed, trusted Windows runtime node | For Big Pickle, evidence flows directly from Windows DB. Distributed evidence is a future extension for external contributors |
| Runtime profile includes benchmark measurements | Windows runtime_profiles have minimal measurements | Full benchmark characterization is a Mac-side execution_profile; Windows profiles are deployment configs |

---

## 3. Directly Reusable Structures

These Windows structures require no alteration and are consumed directly by Mac-side qualification:

| Structure | Table | Consumption |
|-----------|-------|-------------|
| Model inventory | local_models | Mac model_identity_record references model_id + sha256 |
| Deployment configs | runtime_profiles | Mac execution_profile references profile_id |
| Hardware evidence | hardware_profiles | Mac system_profile references hw_profile_id |
| Execution lifecycle | runtime_runs | Mac qualification_run references run_id |
| Event chain | lifecycle_evidence | Mac qualification intake consumes event stream |
| Residency state | job_leases | Mac scheduler queries lease state for routing |
| Residency supervisor | process.rs + residency/ | Mac sends acquire/release requests; Windows enforces |

---

## 4. Missing Mac-Side Structures

| Structure | Purpose | Priority |
|-----------|---------|----------|
| model_identity_record | Extended artifact identity (GGUF metadata, chat template, license) | HIGH |
| qualification_run | Qualification-specific run record referencing Windows runtime_run | HIGH |
| capability_manifest | Role qualification results and Owner decisions | HIGH |
| execution_profile | Full execution characterization referencing Windows runtime_profile | HIGH |
| task_pack | Versioned work fixtures and prompts | HIGH |
| validator_pack | Versioned validation rules | HIGH |
| owner_decision | Owner promotion/rejection decisions | HIGH |
| router_projection | Approved model→role assignments for router consumption | HIGH |
| qualification_request | Request to Windows to execute a qualification run | MEDIUM |
| evidence_packet | Bounded export of Windows execution evidence for Mac intake | MEDIUM |
| comparative_analysis | Roster comparison results per role | MEDIUM |

---

## 5. Bridge Records Required

| Bridge | From | To | Purpose |
|--------|------|----|---------|
| qualification_request | Mac | Windows | Request execution of a specific model + task under residency constraints |
| evidence_packet | Windows | Mac | Export runtime_run + lifecycle_evidence for Mac qualification intake |
| residency_status_query | Mac → Windows | Windows → Mac | Query current lease state for routing decisions |
| acquire/release commands | Mac → Windows | Windows | Request model load/unload through supervisor |

---

## 6. Architecture Changes Required

| Change | Scope | Rationale |
|--------|-------|-----------|
| Mac canonical DB schema | NEW | Houses qualification lifecycle, capability manifests, router projection |
| Evidence bridge protocol | NEW | Defined transfer contract for Windows→Mac evidence flow |
| Windows evidence export API | NEW | Endpoint to query runtime_runs + lifecycle_evidence for a given run |
| Qualification request API | NEW | Endpoint for Mac to request Windows execute a specific qualification run |
| Router projection table | NEW | Mac-side approved projection consumed by packet router |

**No changes to sealed Windows tables, supervisor, or residency logic.**

---

## 7. Planning Pack Document-by-Document Reconciliation

| Document | Key Assumption | Reconciled? | Notes |
|----------|---------------|-------------|-------|
| ARCHITECTURE.md | 8 components | YES | Components 1-3 split across Mac/Windows; 4-8 are Mac-side |
| QUALIFICATION-PROTOCOL.md | Stages 0-7 | YES | Stage 1 (smoke) maps to Windows execution; Stages 2-7 are Mac-side |
| CAPABILITY-MANIFEST-CONTRACT.md | Manifest schema | YES | Schema adopted with no changes |
| EXECUTION-PROFILE-CONTRACT.md | Profile schema | YES | Schema adopted; Windows runtime_profile is input, not replacement |
| COMPARATIVE-CLASSIFICATION.md | Comparison framework | YES | Entirely Mac-side; no Windows involvement |
| DISTRIBUTED-EVIDENCE.md | Contributor packets | DEFERRED | Big Pickle is trusted node; distributed evidence is future extension |
| ROUTER-INTEGRATION.md | Router reads projection | YES | Router consumes Mac-side projection; no Windows DB access |
| VALIDATION-AND-GOVERNANCE.md | Validator design | YES | Validators are Mac-side; Windows provides raw evidence |
| SPRINT-SEQUENCE.md | 30 sprints in 9 phases | REVISED | See implementation sprint chain (Deliverable H) |
| EPIC-PLAN.md | Epic flow | YES | Flow preserved; Windows insertion point at Stage 1 |
