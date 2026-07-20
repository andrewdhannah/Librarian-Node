# Security Baseline

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  

---

## 1. Purpose

Define the security baseline for the Windows Runtime Node. This document describes the security posture, controls, and policies that apply to the Node.

---

## 2. Security Principles

### 2.1 Least Privilege

The Node runs with the minimum privileges required for its function:
- No administrator privileges required for operation
- Network access limited to localhost (by default)
- File system access limited to installation directory

### 2.2 Defense in Depth

Multiple layers of security controls:
- Process isolation (child llama-server process)
- Filesystem isolation (separate directories for binaries, data, logs)
- Network isolation (localhost-only by default)
- Evidence integrity (append-only, cryptographically linked)

### 2.3 Authority Separation

Security is enforced by architectural boundaries, not just code:
- Core ↔ Node separation at crate graph level
- Packet contracts with type-level enforcement
- MCP as transport, not authority

### 2.4 Evidence Integrity

All lifecycle events produce append-only evidence:
- Evidence is never modified or deleted
- Evidence is cryptographically linked
- Recovery evidence is recorded alongside normal evidence

---

## 3. Security Controls

### 3.1 Network Controls

| Control | Description |
|---------|-------------|
| Localhost-only | HTTP API bound to 127.0.0.1 |
| Port range | 9120-9124 |
| No external endpoints | No public-facing services |
| No authentication (local) | Localhost trust model |

### 3.2 Process Controls

| Control | Description |
|---------|-------------|
| Process isolation | Child process for model execution |
| Signal handling | SIGTERM for graceful shutdown |
| CREATE_NO_WINDOW | No console window for child processes |
| Timeout | 120s request timeout |

### 3.3 File System Controls

| Control | Description |
|---------|-------------|
| Secure permissions | Installation directory accessible to service account only |
| No secrets in source | All secrets stored outside repository |
| Evidence protection | Evidence files are append-only |
| Log rotation | Logs are rotated and archived |

### 3.4 Identity Controls

| Control | Description |
|---------|-------------|
| Node identity | Unique identity generated per installation |
| Trust state | Registered, verified, quarantined, revoked |
| Identity persistence | Identity survives restart and upgrade |
| Identity recovery | Recovery path for identity loss |

---

## 4. Security Boundaries

### 4.1 Trust Model

```
Core (fully trusted)
    │
    │ Packet contracts (verified)
    ▼
Node (partially trusted)
    │
    │ Process isolation
    ▼
llama-server.exe (minimally trusted)
```

### 4.2 Trust Decisions

| Decision | Who Decides | How |
|----------|-------------|-----|
| Node admission | Core / Owner | Registration handshake, capability validation |
| Model approval | Core / Owner | Qualification, capability policy |
| Authority grant | Core / Owner | Decision workflow |
| Trust revocation | Core / Owner | Evidence review |

---

## 5. Secrets Policy

### 5.1 What is a Secret

- API keys
- Tokens
- Private keys
- Certificates
- Connection strings
- Database passwords

### 5.2 How Secrets are Handled

- Secrets are never stored in the repository
- Secrets are stored in environment variables or secure vault
- Secrets are passed at runtime, not build time
- Secrets are rotated periodically

### 5.3 Secrets in Evidence

- Evidence must not contain secrets
- Evidence exports must redact secrets
- Evidence storage must be access-controlled

---

## 6. Audit Trail

All security-relevant events are recorded:

| Event | Evidence Type |
|-------|--------------|
| Service start | `service_started` |
| Service stop | `service_stopped` |
| Service crash | `service_crashed` |
| Authentication failure | `auth_failure` |
| Authorization failure | `authz_failure` |
| Configuration change | `config_changed` |
| Security policy violation | `security_violation` |

---

## 7. Incident Response

### 7.1 Incident Detection

- Health monitoring (GET /health)
- Error logging (structured JSON)
- Evidence anomalies (unexpected state changes)
- Network anomalies (unexpected connections)

### 7.2 Incident Response

1. **Detect** — Identify the incident
2. **Contain** — Isolate affected component
3. **Analyze** — Review evidence
4. **Remediate** — Fix the issue
5. **Recover** — Restore to OPERATIONAL
6. **Learn** — Update procedures

---

## 8. References

- ADR-PLATFORM-001 — Core / Node Authority Architecture
- THREAT-MODEL.md — Threat model
- DEPENDENCY-REVIEW.md — Dependency review
- SECRETS-POLICY.md — Secrets policy
- INVARIANT-REVIEW.md — Invariant review
