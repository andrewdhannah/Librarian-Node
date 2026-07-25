# STARTUP-PROTOCOL.md — Canonical Startup Protocol

**Version:** 1.0.0
**Status:** Canonical
**Last Updated:** 2026-07-24

---

## Purpose

This protocol defines the canonical startup sequence that all Librarian Node implementations must follow. It is platform-neutral and applies to Windows, Linux, and macOS nodes.

---

## Startup Sequence

Every Librarian Node must execute the following startup sequence in order:

```
Phase 1: Identity Loading
    │
    ▼
Phase 2: Governance Verification
    │
    ▼
Phase 3: Capability Loading
    │
    ▼
Phase 4: Environment Validation
    │
    ▼
Phase 5: Startup Receipt Generation
    │
    ▼
Phase 6: Enter Governed Mode
```

---

## Phase 1: Identity Loading

### Purpose
Load the node's unique identity and verify it against the governance contract.

### Required Checks

1. **Node Identity File Exists**
   - Location: Platform-specific (see Platform Adapter Boundary)
   - Format: JSON
   - Required fields: `node_id`, `node_type`, `authority`, `platform`

2. **Node Identity is Valid**
   - `node_type` must be `"librarian-runtime-node"`
   - `authority` must be `"owner-controlled"`
   - `platform` must match the running platform

3. **Node Identity is Unique**
   - `node_id` must not be empty
   - `node_id` must be a valid identifier

### Failure Modes

| Failure | Classification | Recovery |
|---------|---------------|----------|
| Identity file not found | FATAL | Cannot start without identity |
| Invalid identity format | FATAL | Cannot start with invalid identity |
| Duplicate node_id | FATAL | Cannot start with duplicate identity |

---

## Phase 2: Governance Verification

### Purpose
Verify that the node has a valid governance contract from the canonical source.

### Required Checks

1. **Governance Source Exists**
   - Location: Platform-specific (see Platform Adapter Boundary)
   - Must point to canonical GitHub repository

2. **Governance Commit is Valid**
   - Must be a valid SHA-256 hash
   - Must match the expected commit

3. **Governance Contracts are Loaded**
   - All required contracts must be present
   - All contracts must be valid

4. **Governance Core is Loaded**
   - Governance algorithms must be loaded
   - Governance state must be initialized

### Failure Modes

| Failure | Classification | Recovery |
|---------|---------------|----------|
| Governance source not found | FATAL | Cannot start without governance |
| Invalid governance commit | FATAL | Cannot start with invalid governance |
| Missing contracts | FATAL | Cannot start with incomplete governance |
| Governance verification failed | FATAL | Cannot start with unverified governance |

---

## Phase 3: Capability Loading

### Purpose
Load the node's capability declarations and verify they satisfy governance requirements.

### Required Checks

1. **Capability Declarations Exist**
   - Location: Platform-specific (see Platform Adapter Boundary)
   - Format: JSON array of capability strings

2. **Capability Declarations are Valid**
   - All capabilities must be non-empty strings
   - All capabilities must follow naming convention: `namespace.action`

3. **Capability Declarations Satisfy Governance**
   - Required capabilities must be present
   - Forbidden capabilities must not be present

### Failure Modes

| Failure | Classification | Recovery |
|---------|---------------|----------|
| Capability file not found | FATAL | Cannot start without capabilities |
| Invalid capability format | FATAL | Cannot start with invalid capabilities |
| Missing required capabilities | FATAL | Cannot start without required capabilities |
| Forbidden capability present | FATAL | Cannot start with forbidden capabilities |

---

## Phase 4: Environment Validation

### Purpose
Validate that the runtime environment meets minimum requirements.

### Required Checks

1. **Platform Requirements Met**
   - Minimum OS version
   - Required runtime (PowerShell, bash, Swift, etc.)
   - Required dependencies

2. **Storage Requirements Met**
   - Minimum free disk space
   - Write access to evidence directory
   - Write access to receipt directory

3. **Network Requirements Met** (if applicable)
   - Access to governance source
   - Access to model runtime (if using LLM)

### Failure Modes

| Failure | Classification | Recovery |
|---------|---------------|----------|
| Platform requirements not met | FATAL | Cannot start on unsupported platform |
| Storage requirements not met | WARNING | May start but evidence generation may fail |
| Network requirements not met | WARNING | May start but governance sync may fail |

---

## Phase 5: Startup Receipt Generation

### Purpose
Generate a startup receipt that captures the outcome of all startup checks.

### Required Fields

```json
{
  "receipt_id": "<unique-receipt-id>",
  "node_id": "<node-identifier>",
  "platform": "<windows|linux|macos>",
  "governance_commit": "<commit-sha>",
  "startup_phase": "complete",
  "identity_loaded": true,
  "governance_verified": true,
  "capabilities_loaded": true,
  "environment_validated": true,
  "checks_passed": 6,
  "checks_failed": 0,
  "status": "GOVERNED_EXECUTION",
  "timestamp": "<iso-8601-timestamp>"
}
```

### Failure Modes

| Failure | Classification | Recovery |
|---------|---------------|----------|
| Receipt generation failed | FATAL | Cannot start without startup receipt |
| Invalid receipt format | FATAL | Cannot start with invalid receipt |

---

## Phase 6: Enter Governed Mode

### Purpose
Transition the node from startup to governed execution mode.

### Required Actions

1. **Update Node State**
   - Set `state` to `"GOVERNED_EXECUTION"`
   - Record startup timestamp

2. **Initialize Runtime**
   - Load execution engine
   - Initialize work packet intake
   - Initialize evidence generation

3. **Signal Ready**
   - Emit "ready" signal
   - Accept work packets

### Failure Modes

| Failure | Classification | Recovery |
|---------|---------------|----------|
| State transition failed | FATAL | Cannot enter governed mode |
| Runtime initialization failed | FATAL | Cannot operate without runtime |
| Ready signal failed | WARNING | May start but clients may not detect readiness |

---

## Platform Adapter Boundary

Each platform must implement the startup protocol using platform-native mechanisms. The shared contract defines **what** must happen; the adapter defines **how**.

| Contract Requirement | Windows Adapter | Linux Adapter | macOS Adapter |
|---------------------|-----------------|---------------|---------------|
| Identity loading | Windows registry / file system | /etc/librarian/node-identity.json | ~/Library/Librarian/node-identity.json |
| Governance verification | Windows crypto APIs | OpenSSL | Security framework |
| Capability loading | Windows ACLs | Linux permissions | macOS entitlements |
| Environment validation | Windows system info | /proc/cpuinfo, uname | sysctl, sw_vers |
| Receipt generation | Windows Event Log | syslog/journald | Unified logging |
| State transition | Windows service model | systemd | launchd |

---

## Equivalence Requirements

All platform implementations must produce equivalent startup receipts. Equivalence is defined as:

**Deterministic Fields (Must Match):**
- `governance_commit`
- `identity_loaded`
- `governance_verified`
- `capabilities_loaded`
- `environment_validated`
- `checks_passed`
- `checks_failed`
- `status`

**Variable Fields (Expected to Differ):**
- `receipt_id`
- `node_id`
- `platform`
- `timestamp`

---

## References

- Node Reference Architecture: `docs/NODE-REFERENCE-ARCHITECTURE.md`
- Platform Adapter Boundary: `docs/PLATFORM-ADAPTER-BOUNDARY.md`
- Three-Way Equivalence Protocol: `docs/THREE-WAY-EQUIVALENCE-PROTOCOL.md`
