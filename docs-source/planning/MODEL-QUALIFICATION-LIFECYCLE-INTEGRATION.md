# MODEL-QUALIFICATION-LIFECYCLE-INTEGRATION

**Sprint:** MODEL-QUALIFICATION-ROUTER-BASELINE-INTEGRATION-PLAN-1
**Gate:** MQR-BI-8, MQR-BI-9

---

## Purpose

Define how the planning pack's 8-stage qualification protocol integrates with the sealed Windows runtime primitives.

---

## Qualification Stages → Runtime Integration

### Stage 0: Identity Check

**Planning pack:** Verify model artifact identity — filename, SHA-256, GGUF metadata, quantization, tokenizer/chat-template identity.

**Windows integration:** Queries local_models for model_id, filename, sha256, quantization, file_size_bytes. Queries hardware_profiles for GPU context.

**Mac-side:** Queries model_identity_record for extended identity (GGUF metadata hash, chat_template_id, license SPDX).

**Input to Windows:** None. Pure read.

**Output:** model_identity_record created or verified.

---

### Stage 1: Smoke / Viability Check

**Planning pack:** Confirm model loads, runs at least one basic prompt, does not crash, and releases resources cleanly. This is a pass/fail gate. **Not** a qualification statement.

**Windows integration:**

1. Mac sends qualification_request to Windows
2. Windows executes:
   - Residency acquire → `job_leases` INSERT
   - Process start → `lifecycle_evidence` INSERT (process_started)
   - Load → `runtime_runs` INSERT (load_duration_ms)
   - Basic generation → `runtime_runs` UPDATE (output_tokens, generation_duration_ms)
   - `lifecycle_evidence` INSERT (generation_completed)
   - Unload → `lifecycle_evidence` INSERT (process_killed)
   - GPU release verification → `lifecycle_evidence` INSERT (gpu_release_verified)
   - Residency release → `job_leases` UPDATE (state=unloaded)
3. Windows creates evidence_packet with full lifecycle
4. Mac receives evidence_packet
5. Mac validates packet integrity
6. Mac records qualification_run (status=smoke_pass or smoke_fail)

**Pass criteria:**
- load_duration_ms < 60000 (60 seconds)
- generation_duration_ms > 0
- exit_status = "clean" or "stopped"
- release_verification.within_tolerance = true
- No lifecycle_evidence with event_type = process_crashed

**Fail criteria:**
- Process crash before generation
- VRAM not released within tolerance
- Load timeout (60s)
- Generation timeout (120s)

**Key constraint:** Stage 1 success does NOT imply role qualification. It proves execution viability only.

---

### Stage 2: Primitive Probes

**Planning pack:** Execute versioned prompt fixtures for each work role. Measure token generation, latency, stability across multiple runs. Apply threshold tests.

**Windows integration:** Each probe is a separate qualification request to Windows. Windows executes independently; Mac records evidence.

**Probe types:**
| Probe | Fixture | Measures |
|-------|---------|----------|
| IF (Instruction Following) | IF-001 through IF-010 | Prompt adherence, output structure |
| SO (Structured Output) | SO-001 through SO-005 | JSON/tool-call format compliance |
| CL (Consistency) | CL-001 through CL-003 | Same prompt, multiple runs, output similarity |
| EX (Extraction) | EX-001 through EX-005 | Information extraction accuracy |
| NT (Long-context / Needle) | NT-001 through NT-005 | Information retrieval from long contexts |
| EC (Error Correctness) | EC-001 through EC-003 | Bug identification and fix correctness |
| SR (System Reliability) | SR-001 through SR-003 | Multi-turn conversation, state management |

**Each probe execution:**
1. Mac creates task_pack (versioned fixture)
2. Mac sends qualification_request
3. Windows executes run
4. Windows returns evidence_packet
5. Mac records qualification_run (linked to task_pack)
6. Mac applies validator_pack rules to raw output
7. Mac records stage results in qualification_stage_log

---

### Stage 3: Role Trials

**Planning pack:** Execute role-specific work scenarios. Evaluate not just prompt compliance but quality of work output.

**Windows integration:** Same as Stage 2, but with more complex, multi-turn scenarios.

**Key difference from Stage 2:** Stage 2 tests individual prompt-response pairs. Stage 3 tests sustained work sessions.

---

### Stage 4: Cross-role Consistency

**Planning pack:** Ensure model behavior is consistent across roles. A model qualified for "implementer" should not exhibit contradictory behavior when tested as "researcher."

**Windows integration:** Runs across multiple roles sequentially. Same Windows runtime; different task packs.

---

### Stage 5: Shadow Work

**Planning pack:** Observe model behavior in realistic work contexts. Measure not just output quality but process quality — does the model work in a way that's compatible with the system?

**Windows integration:** Extended runtime sessions. May span multiple runs with different task packs.

---

### Stage 6: Comparative Roster

**Planning pack:** Compare candidate model against other qualified models for the same role. Produce relative ranking.

**Windows integration:** Runs competitor models through same Stage 2-5 protocols. Each run is a separate qualification request.

**Note:** Comparative analysis is Mac-side only. Windows executes each model independently; Mac compares results.

---

### Stage 7: Owner Decision

**Planning pack:** Human or automated decision to promote, reject, or conditionally approve the model for a role.

**Windows integration:** None. Pure Mac-side decision recorded in capability_manifest.

---

## Stage Gate Rules

| Rule | Description |
|------|-------------|
| Stages are sequential | Cannot skip a stage |
| Each stage must complete before next begins | qualification_stage_log tracks progression |
| Stage failure stops the pipeline | No Stage 2+ if Stage 1 fails |
| Owner can override at any stage | owner_decision can halt pipeline |
| Evidence is immutable | qualification_run records are append-only |

---

## Windows Execution Pattern

Every qualification run on Windows follows this exact pattern:

```
1. qualification_request received
2. job_leases INSERT (state=acquiring)
3. lifecycle_evidence INSERT (process_started)
4. runtime_runs INSERT
5. [execute task]
6. runtime_runs UPDATE (exit_status, timing, tokens)
7. lifecycle_evidence INSERT (generation_completed)
8. lifecycle_evidence INSERT (process_killed)
9. lifecycle_evidence INSERT (gpu_release_verified)
10. job_leases UPDATE (state=unloaded, released_at, vram_released_at)
11. evidence_packet created and returned
```

This pattern is identical to the Sprint 3 residency supervisor flow. No Windows code changes required.

---

## Key Integration Points

| Integration | From | To | Mechanism |
|-------------|------|----|-----------|
| Model lookup | Mac | Windows | local_models query via API |
| Hardware lookup | Mac | Windows | hardware_profiles query via API |
| Execution request | Mac | Windows | qualification_request API |
| Evidence transfer | Windows | Mac | evidence_packet API |
| Residency query | Mac | Windows | /residency/status API |
| Status feedback | Windows | Mac | qualification_request status updates |
