# MODEL-QUALIFICATION-ROUTER-AUTHORITY-MAP

**Sprint:** MODEL-QUALIFICATION-ROUTER-BASELINE-INTEGRATION-PLAN-1
**Gate:** MQR-BI-4

---

## Purpose

Explicit ownership matrix covering every record type in the qualification and routing system. Defines canonical owner, producer, consumer, mutation authority, and cross-node transfer direction.

---

## Authority Domains

### Mac Librarian — Canonical Authority

Owns: intent, packets, context, qualification policy, routing, Owner authority, capability manifests, router projection.

### Windows Runtime Node — Execution Authority

Owns: installed model inventory, runtime profiles, hardware profiles, residency leases, runtime runs, managed process state, load/unload lifecycle, GPU release verification, machine-local lifecycle evidence.

---

## Ownership Matrix

### Windows-Owned Records

| Record | Canonical Owner | Producer | Consumer | Mutation Authority | Cross-Node Transfer |
|--------|----------------|----------|----------|-------------------|-------------------|
| local_models | Windows | Model installer / Sprint 1 | Mac model_identity_record | Windows only | Windows → Mac (read-only reference) |
| runtime_profiles | Windows | Profiler / Sprint 1 | Mac execution_profile | Windows only | Windows → Mac (read-only reference) |
| hardware_profiles | Windows | HW qualification / Sprint 2 | Mac system_profile | Windows only | Windows → Mac (read-only reference) |
| job_leases | Windows | Residency supervisor | Mac scheduler (query only) | Windows only | Windows → Mac (status query) |
| runtime_runs | Windows | Residency supervisor | Mac qualification_run | Windows (append + update) | Windows → Mac (evidence transfer) |
| lifecycle_evidence | Windows | Runtime node | Mac qualification intake | Windows (append-only) | Windows → Mac (evidence transfer) |
| schema_migrations | Windows | Migration system | — | Windows only | Never transferred |

### Mac-Owned Records

| Record | Canonical Owner | Producer | Consumer | Mutation Authority | Cross-Node Transfer |
|--------|----------------|----------|----------|-------------------|-------------------|
| model_identity_record | Mac | Intake agent | qualification_run, execution_profile | Mac only | Never to Windows |
| task_pack | Mac | Task author / Owner | qualification_run, validator | Mac only | Never to Windows |
| validator_pack | Mac | Validator author | validator execution | Mac only | Never to Windows |
| qualification_request | Mac | Qualification runner | Windows execution agent | Mac (create), Windows (ack) | Mac → Windows |
| qualification_run | Mac | Qualification runner | capability_manifest, comparative_analysis | Mac only | Never to Windows |
| evidence_packet | Mac | Evidence intake | qualification_run | Mac only | Never to Windows |
| capability_manifest | Mac | Role classifier + Owner decision | router_projection | Mac only | Never to Windows |
| owner_decision | Mac | Owner | capability_manifest, router_projection | Mac only | Never to Windows |
| execution_profile | Mac | Execution profiler | router_projection | Mac only | Never to Windows |
| comparative_analysis | Mac | Roster analyzer | owner_decision | Mac only | Never to Windows |
| router_projection | Mac | Owner-approved promotion | packet router | Mac only | Never to Windows |
| system_profile | Mac | System intake | execution_profile | Mac only | Never to Windows |

### Bridge Records (Cross-Node)

| Bridge | Direction | Producer | Consumer | Mutation Authority |
|--------|-----------|----------|----------|-------------------|
| qualification_request | Mac → Windows | Mac qualification runner | Windows execution agent | Mac (create), Windows (status update) |
| evidence_packet | Windows → Mac | Windows evidence export | Mac evidence intake | Windows (create), Mac (consume) |
| residency_status_query | Mac → Windows → Mac | Mac scheduler | Windows supervisor | Windows (respond), Mac (read) |

---

## Transfer Direction Rules

1. **Windows → Mac:** Evidence flows one direction. Windows produces execution evidence; Mac consumes it for qualification decisions.

2. **Mac → Windows:** Commands flow one direction. Mac sends qualification requests and residency commands; Windows executes or rejects.

3. **No reverse flow:** Windows never receives capability status, role assignments, or router decisions. Windows does not know whether a model is "qualified."

4. **No shared mutation:** No record is mutated by both nodes. Each record has exactly one canonical mutation authority.

---

## Q8 Canary Preservation

The Q8_0 canary proves the authority split:

| Event | Who owns | What happens |
|-------|----------|-------------|
| Q8 loads successfully | Windows | runtime_run records load, lifecycle_evidence records process_started |
| Q8 generates tokens | Windows | runtime_run records output_tokens, lifecycle_evidence records generation_completed |
| Q8 releases VRAM | Windows | lifecycle_evidence records gpu_release_verified |
| Q8 assigned a work role | **NEVER** | Windows has no capability tables; no role assignment is possible |

The Mac qualification system may later evaluate Q8 through its own qualification protocol. Windows execution success does not influence that evaluation.

---

## Failure Mode: Authority Leakage

| Leakage Pattern | Prevention |
|----------------|------------|
| Windows DB gains a capability column | DB-13 enforced; migration audit prevents new authority tables on Windows |
| Router reads Windows runtime_runs directly | Router consumes only Mac-side router_projection; no direct Windows DB access |
| Qualification runner uses HTTP 200 as capability signal | Stage 1 consumes execution evidence without promotion; capability requires Stages 2-7 |
| Mac sends role assignment to Windows | No API endpoint exists for Windows to receive capability data |
