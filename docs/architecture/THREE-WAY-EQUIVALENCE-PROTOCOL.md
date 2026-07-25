# THREE-WAY-EQUIVALENCE-PROTOCOL.md — Cross-Platform Equivalence Validation

**Version:** 1.0.0
**Status:** Canonical
**Last Updated:** 2026-07-24

---

## Purpose

This document defines the protocol for validating equivalence across three platform implementations (Windows, Linux, macOS). It establishes how to prove that same governance input produces same governance outcome on all platforms.

---

## Equivalence Principle

The goal is NOT to prove exact equality of receipts. The goal IS to prove semantic equivalence of governance decisions and outcomes.

```
Same Governance Input → Same Governance Outcome
```

---

## Equivalence Checks

### Check 1: Startup Sequence Equivalence

All platforms must execute the same 6-phase startup sequence:

1. Identity Loading
2. Governance Verification
3. Capability Loading
4. Environment Validation
5. Startup Receipt Generation
6. Enter Governed Mode

**Validation:**
- All platforms must pass all phases
- All platforms must produce startup receipts
- All platforms must enter GOVERNED_EXECUTION state

### Check 2: Receipt Deterministic Field Equivalence

All platforms must produce startup receipts with identical deterministic fields:

| Field | Expected Value | Validation |
|-------|----------------|------------|
| `governance_commit` | `<canonical-commit-sha>` | Exact match |
| `identity_loaded` | `true` | Exact match |
| `governance_verified` | `true` | Exact match |
| `capabilities_loaded` | `true` | Exact match |
| `environment_validated` | `true` | Exact match |
| `checks_passed` | `>= 5` | Equivalent (minimum) |
| `checks_failed` | `0` | Exact match |
| `status` | `GOVERNED_EXECUTION` | Exact match |

### Check 3: Governance Decision Equivalence

All platforms must make identical governance decisions:

| Decision | Expected Outcome | Validation |
|----------|------------------|------------|
| Identity valid? | `true` | Exact match |
| Governance verified? | `true` | Exact match |
| Capabilities present? | `true` | Exact match |
| Environment valid? | `true` | Exact match |
| Enter governed mode? | `true` | Exact match |

### Check 4: Capability Declaration Equivalence

All platforms must declare equivalent capabilities:

| Capability | Expected Value | Validation |
|------------|----------------|------------|
| `governance_read` | `true` | Exact match |
| `governance_verify` | `true` | Exact match |
| `execution_allowed` | Platform-dependent | Equivalent |
| `governance_write` | `false` | Exact match |

### Check 5: Divergence Detection

The equivalence validation must correctly identify when equality fails:

- If any deterministic field differs, report divergence
- If any governance decision differs, report divergence
- If any capability declaration differs, report divergence
- If divergence is detected, report which specific fields differ

---

## Equivalence Validation Protocol

### Step 1: Prepare Environment

- Create equivalent node configurations on all three platforms
- Ensure same governance commit is available
- Ensure same node identity format is used

### Step 2: Execute Startup

- Run startup sequence on all three platforms
- Capture startup receipts from all three platforms

### Step 3: Validate Receipts

- Extract deterministic fields from all receipts
- Compare deterministic fields across platforms
- Report any divergences

### Step 4: Validate Governance Decisions

- Extract governance decisions from all receipts
- Compare decisions across platforms
- Report any divergences

### Step 5: Validate Capabilities

- Extract capability declarations from all nodes
- Compare capabilities across platforms
- Report any divergences

### Step 6: Generate Equivalence Report

- Summarize all checks performed
- Report pass/fail status for each check
- Report overall equivalence status

---

## Equivalence Report Format

```json
{
  "report_id": "EQUIVALENCE-<timestamp>",
  "platforms_tested": ["windows", "linux", "macos"],
  "checks_performed": [
    {
      "check_name": "startup_sequence_equivalence",
      "status": "passed|failed",
      "details": {
        "windows_phases_passed": 6,
        "linux_phases_passed": 6,
        "macos_phases_passed": 6
      }
    },
    {
      "check_name": "receipt_deterministic_field_equivalence",
      "status": "passed|failed",
      "details": {
        "governance_commit": {
          "windows": "<commit-sha>",
          "linux": "<commit-sha>",
          "macos": "<commit-sha>",
          "equivalent": true
        },
        "identity_loaded": {
          "windows": true,
          "linux": true,
          "macos": true,
          "equivalent": true
        }
      }
    }
  ],
  "divergences_detected": [],
  "overall_status": "EQUIVALENT|DIVERGENT",
  "timestamp": "<iso-8601-timestamp>"
}
```

---

## References

- Node Reference Architecture: `NODE-REFERENCE-ARCHITECTURE.md`
- Platform Adapter Boundary: `PLATFORM-ADAPTER-BOUNDARY.md`
- Startup Protocol: `contracts/startup/STARTUP-PROTOCOL.md`
- Startup Output Contract: `contracts/startup/STARTUP-OUTPUT-CONTRACT.md`
