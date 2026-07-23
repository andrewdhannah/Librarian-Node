# MODEL-QUALIFICATION-ROUTER-SCHEMA-ALLOCATION

**Sprint:** MODEL-QUALIFICATION-ROUTER-BASELINE-INTEGRATION-PLAN-1
**Gate:** MQR-BI-6, MQR-BI-7

---

## Purpose

Define the exact schema allocation across Mac canonical DB and Windows operational DB. Show bridge packet schemas. No generic monolith — every table has a defined owner, purpose, and consumption pattern.

---

## Windows Operational DB (Sealed)

Path: `G:\openwork\librarian-runtime-node\data\runtime-operational.db`

### Existing Tables (Sealed — No Alteration)

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| local_models | Installed model inventory | model_id, display_name, family, source_repository, filename, quantization, sha256, file_size_bytes |
| runtime_profiles | Deployment configurations | profile_id, model_id, device_backend, gpu_layers, context_tokens, measured_vram_mb, measured_tokens_per_sec |
| hardware_profiles | GPU/system hardware evidence | hw_profile_id, device_name, vulkan_device, total_vram_mb, available_vram_mb, driver_version |
| job_leases | Residency lease lifecycle | lease_id, model_id, profile_id, port, process_id, state, loaded_at, released_at |
| runtime_runs | Execution lifecycle records | run_id, lease_id, packet_id, input_tokens, output_tokens, load_duration_ms, generation_duration_ms, exit_status |
| lifecycle_evidence | Append-only event chain | evidence_id, event_type, model_id, lease_id, run_id, process_id, observed_state, observation_json |
| schema_migrations | Migration tracking | version, applied_at |

### Windows API Endpoints (Existing + Planned)

| Endpoint | Method | Purpose |
|----------|--------|---------|
| /backend/status | GET | Current residency state |
| /backend/select | POST | Request model load (residency acquire) |
| /backend/stop | POST | Request model unload (residency drain+release) |
| /backend/health | GET | Process health check |
| /v1/chat/completions | POST | Generation request |
| /evidence/runs | GET | **PLANNED:** Query runtime_runs for evidence export |
| /evidence/lifecycle | GET | **PLANNED:** Query lifecycle_evidence for a run or lease |
| /residency/status | GET | **PLANNED:** Query current residency state for Mac scheduler |

---

## Mac Canonical DB (New)

Path: TBD (e.g., `~/Library/Application Support/Librarian/librarian-canonical.db`)

### Model Identity

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| model_identity_record | Extended artifact identity for qualification | identity_id, model_id_ref, gguf_metadata_hash, chat_template_id, license_spdx, qualification_scope, created_at |
| system_profile | Mac-side system description | system_profile_id, os, cpu, ram_mb, gpu_description, notes, created_at |

### Qualification Lifecycle

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| task_pack | Versioned work fixtures | task_pack_id, version, role, description, fixture_hash, fixture_path, created_at |
| validator_pack | Versioned validation rules | validator_pack_id, version, role, description, rules_hash, rules_path, created_at |
| qualification_request | Request sent to Windows | request_id, identity_id, task_pack_id, runtime_profile_id, status, requested_at, completed_at |
| qualification_run | Qualification-specific run record | qual_run_id, request_id, windows_run_id, identity_id, task_pack_id, validator_pack_id, status, raw_output_path, tool_trace_path, validator_results_json, created_at |
| qualification_stage_log | Stage progression tracking | stage_log_id, qual_run_id, stage, status, started_at, completed_at, notes |

### Capability and Routing

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| capability_manifest | Role qualification results | manifest_id, identity_id, protocol_version, status, roles_json, owner_decision_id, created_at, updated_at |
| owner_decision | Owner promotion/rejection decisions | decision_id, manifest_id, decision, rationale, roles_affected_json, decided_at |
| execution_profile | Full execution characterization | exec_profile_id, identity_id, system_profile_id, windows_runtime_profile_id, measurements_json, pareto_class, created_at |
| comparative_analysis | Roster comparison results per role | analysis_id, role, candidate_identity_id, findings_json, compared_at |
| router_projection | Approved model→role assignments | projection_id, manifest_id, identity_id, roles_json, constraints_json, priority, approved_at, expires_at |

### Router

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| routing_log | Packet routing decisions | routing_log_id, packet_id, required_role, selected_identity_id, selected_exec_profile_id, projection_id, rationale, routed_at |

---

## Bridge Packet Schemas

### Evidence Packet (Windows → Mac)

Exported by Windows evidence API, consumed by Mac qualification intake.

```json
{
  "packet_type": "evidence_packet",
  "packet_version": "1",
  "exported_at": "2026-07-11T12:00:00Z",
  "qualification_request_id": "qr-...",
  "identity": {
    "model_id": "minicpm5-1b-q4km",
    "sha256": "81B64D05A23B...",
    "filename": "MiniCPM5-1B-Q4_K_M.gguf",
    "quantization": "Q4_K_M"
  },
  "execution": {
    "runtime_profile_id": "prof-q4km",
    "hardware_profile_id": "hw-rx570",
    "runtime_executable_sha256": "0D496467CFD9...",
    "runtime_executable_version": "c85e97a"
  },
  "lease": {
    "lease_id": "lease-...",
    "port": 9120,
    "state": "unloaded",
    "loaded_at": "...",
    "released_at": "...",
    "vram_released_at": "..."
  },
  "run": {
    "run_id": "run-...",
    "input_tokens": 10,
    "output_tokens": 32,
    "load_duration_ms": 2187,
    "generation_duration_ms": 385,
    "exit_status": "clean",
    "started_at": "...",
    "ended_at": "..."
  },
  "lifecycle_events": [
    {
      "event_type": "runtime_ready",
      "process_id": 10804,
      "observed_state": "ready",
      "observation": {"load_duration_ms": 2187},
      "occurred_at": "..."
    }
  ],
  "release_verification": {
    "pid_exit_verified": true,
    "gpu_release_verified": true,
    "free_vram_mb": 3433,
    "baseline_vram_mb": 3433,
    "within_tolerance": true
  }
}
```

### Qualification Request (Mac → Windows)

Sent by Mac qualification runner, received by Windows execution agent.

```json
{
  "packet_type": "qualification_request",
  "packet_version": "1",
  "request_id": "qr-...",
  "identity": {
    "model_id": "minicpm5-1b-q4km",
    "sha256": "81B64D05A23B..."
  },
  "execution": {
    "runtime_profile_id": "prof-q4km",
    "task_description": "Execute instruction-following fixture IF-001",
    "max_tokens": 256,
    "temperature": 0.0,
    "timeout_seconds": 120
  },
  "constraints": {
    "require_release_proof": true,
    "max_vram_mb": 4096
  }
}
```

---

## No Generic Monolith

Every table has:
- **Exactly one owner** (Mac or Windows, never shared)
- **Exactly one mutation authority** (the owner node)
- **Defined consumption pattern** (who reads it and why)
- **Transfer direction** (Windows→Mac for evidence, Mac→Windows for commands)

The Windows DB is sealed. The Mac DB is new. Bridge packets are the only cross-node data flow.
