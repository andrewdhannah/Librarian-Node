# Threat Model

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  

---

## 1. Trust Boundaries

```
External Network
    │
    │── MCP (authenticated)
    ▼
┌─────────────────────┐
│   Core (trusted)    │
│                     │
│  - Canonical truth  │
│  - Owner decisions  │
└───────┬─────────────┘
        │
        │ Packet contracts (verified types)
        ▼
┌─────────────────────┐
│   Node (partially   │
│   trusted)          │
│                     │
│  - Execution        │
│  - Evidence         │
└───────┬─────────────┘
        │
        │ Child process (isolated)
        ▼
┌─────────────────────┐
│ llama-server.exe    │
│ (minimally trusted) │
└─────────────────────┘
```

---

## 2. Threat Scenarios

### T1: Unauthorized Node Admission

**Threat:** A malicious Node self-admits or bypasses Core admission.

**Impact:** Unauthorized execution, false evidence, data exposure.

**Mitigation:**
- Admission requires Core/Owner authorization
- Node cannot self-authorize (N-I3)
- Discovery does not imply trust (C-I5)
- Admission evidence recorded

**Severity:** Critical

### T2: Evidence Tampering

**Threat:** Evidence is modified or deleted after recording.

**Impact:** Loss of audit trail, inability to verify work.

**Mitigation:**
- Evidence is append-only (P-I1)
- Evidence is cryptographically linked
- Recovery evidence is recorded
- Database is WAL-mode with integrity verification

**Severity:** Critical

### T3: Authority Escalation

**Threat:** Node accesses Core functions or modifies canonical state.

**Impact:** Canonical truth corruption, governance bypass.

**Mitigation:**
- Crate-level separation (compile-time enforcement)
- Packet contracts with type-level enforcement
- Node cannot write to canonical DB
- Node cannot modify governance rules

**Severity:** Critical

### T4: Process Injection

**Threat:** Malicious code injected into llama-server process.

**Impact:** Arbitrary code execution, data theft, GPU access.

**Mitigation:**
- Process isolation (child process)
- Child process has minimum privileges
- Health monitoring detects anomalies
- Crash recovery with evidence recording

**Severity:** High

### T5: Network Eavesdropping

**Threat:** Network traffic between Core and Node intercepted.

**Impact:** Data exposure, packet manipulation.

**Mitigation:**
- Localhost-only by default
- Future: TLS for remote communication
- No sensitive data in transit (evidence is advisory)

**Severity:** Medium

### T6: Identity Theft

**Threat:** Node identity stolen and used to impersonate Node.

**Impact:** Unauthorized execution, false evidence, trust violation.

**Mitigation:**
- Identity stored in protected database
- Identity keypair generated per installation
- Identity recovery requires authorization
- Trust state can be revoked by Core

**Severity:** High

### T7: Configuration Tampering

**Threat:** Configuration files modified without authorization.

**Impact:** Changed behavior, security bypass, denial of service.

**Mitigation:**
- Configuration validation on startup
- Configuration changes require impact analysis
- Evidence recorded for configuration changes

**Severity:** Medium

### T8: Denial of Service

**Threat:** Overwhelm Node with requests, causing resource exhaustion.

**Impact:** Service unavailability, delayed evidence processing.

**Mitigation:**
- Single concurrent request (by design)
- Request timeout (120s)
- Health monitoring detects degradation
- Process restart on failure

**Severity:** Medium

---

## 3. Attack Surface

| Surface | Exposure | Risk |
|---------|----------|------|
| HTTP API (localhost) | Low | Medium |
| MCP (future) | Medium | High |
| File system | Low | Low |
| GPU | Low | Low |
| Database | Low | Medium |

---

## 4. Security Assumptions

1. **Localhost is trusted** — Node API bound to 127.0.0.1
2. **Core is authoritative** — Core decisions are not overridden by Node
3. **Evidence is verifiable** — Evidence chain can be audited
4. **Identity is unique** — Each Node has a unique, persistent identity
5. **Configuration is valid** — Configuration is validated before application

---

## 5. References

- SECURITY-BASELINE.md — Security baseline
- DEPENDENCY-REVIEW.md — Dependency review
- SECRETS-POLICY.md — Secrets policy
- ADR-PLATFORM-001 — Core / Node Authority Architecture
