# MODEL-WORK-PACKET-ROUTING-INTEGRATION

**Sprint:** MODEL-QUALIFICATION-ROUTER-BASELINE-INTEGRATION-PLAN-1
**Gate:** MQR-BI-12, MQR-BI-13

---

## Purpose

Define how the packet router consumes the qualification system's approved projection to route work packets to the right model for the right role.

---

## Packet → Router → Model Flow

```
Work Packet (with required_role)
    ↓
Router (queries router_projection)
    ↓
Selected Model + Execution Profile
    ↓
Residency Check (is model loaded? can it be loaded?)
    ↓
Windows Execution (load → generate → release)
    ↓
Packet Response
```

---

## Work Packet Structure

```json
{
  "packet_id": "pkt-...",
  "required_role": "implementer",
  "prompt": "...",
  "context": [...],
  "constraints": {
    "max_tokens": 4096,
    "temperature": 0.0,
    "timeout_seconds": 120
  }
}
```

The packet declares what role it needs. It does NOT declare which model to use. The router decides.

---

## Router Selection

1. Router receives packet with `required_role`
2. Queries `router_projection` for approved projections matching the role
3. Filters by hardware constraints (VRAM, backend compatibility)
4. Filters by execution constraints (max tokens, timeout)
5. Selects best match by priority
6. Returns selected model identity + execution profile

---

## Residency Integration

After router selects a model:

1. **Check residency:** Is the model currently loaded on Windows?
   - Yes → route directly (warm path)
   - No → request load (cold path)

2. **Cold path:**
   - Router sends acquire command to Windows
   - Windows residency supervisor acquires lease, starts process
   - Router waits for Ready state
   - Router sends generation request
   - Windows executes, returns response
   - Router releases residency (or keeps warm for future packets)

3. **Warm path:**
   - Router sends generation request directly
   - Windows executes, returns response
   - Residency stays active for next packet

---

## Residency Lifecycle for Routing

| State | Router Action |
|-------|--------------|
| Unloaded | Send acquire, wait for Ready |
| Loading | Wait for Ready (timeout after 60s) |
| Ready | Send generation request |
| Running | Wait for Ready (queue packet) |
| Draining | Wait for drain complete, then acquire |
| Unloading | Wait for Unloaded, then acquire |
| VerifyingRelease | Wait for Unloaded, then acquire |
| Failed | Do not route; mark packet failed |

---

## Multi-Packet Routing

When multiple packets arrive concurrently:

| Scenario | Behavior |
|----------|----------|
| Single model loaded, single packet | Direct route |
| Single model loaded, multiple packets | Queue packets, process sequentially |
| Multiple models loaded, single packet | Select by role match + priority |
| No model loaded, single packet | Cold load, then route |
| No model loaded, multiple packets | Cold load first, queue rest |
| Model loading, packets arrive | Queue until Ready |

---

## Packet Response

```json
{
  "packet_id": "pkt-...",
  "selected_model": {
    "identity_id": "id-...",
    "display_name": "MiniCPM5-1B-Q4_K_M",
    "role": "implementer"
  },
  "execution": {
    "run_id": "run-...",
    "load_duration_ms": 2187,
    "generation_duration_ms": 385,
    "input_tokens": 10,
    "output_tokens": 32
  },
  "response": {
    "content": "...",
    "finish_reason": "stop"
  },
  "routing_log": {
    "projection_id": "proj-...",
    "selection_rationale": "highest priority implementer with Q4_K_M"
  }
}
```

---

## Failure Modes

| Failure | Router Action | User Impact |
|---------|--------------|-------------|
| No projection for role | Reject packet | "No model qualified for role X" |
| Selected model not on Windows | Cold load attempt | Cold path latency |
| Cold load fails | Try next projection | Fallback to alternate model |
| All projections fail | Reject packet | "No model available for role X" |
| Generation timeout | Kill run, try next projection | Fallback or rejection |
| Residency conflict | Wait or try alternate | Added latency |

---

## Router Ownership

| Concern | Owner | Notes |
|---------|-------|-------|
| Projection selection | Mac router | Reads router_projection, makes selection |
| Residency management | Windows supervisor | Enforces single-residency, GPU release |
| Generation execution | Windows runtime | llama-server.exe process |
| Packet routing decision | Mac router | Logs in routing_log |
| Model promotion | Mac Owner | Approves router_projection |
| Model demotion | Mac Owner | Revokes or expires router_projection |

The router is a pure consumer of Mac-side approved projections. It never interprets raw evidence, never reads Windows qualification data, and never directly accesses Windows DB.
