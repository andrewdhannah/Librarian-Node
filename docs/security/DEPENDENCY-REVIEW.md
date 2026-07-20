# Dependency Review

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  

---

## 1. Purpose

Review all dependencies for security, licensing, and maintenance risks.

---

## 2. Rust Dependencies

| Dependency | Version | License | Risk | Notes |
|------------|---------|---------|------|-------|
| axum | 0.8 | MIT | Low | Well-maintained |
| tokio | 1.x | MIT | Low | Well-maintained |
| tower | 0.5 | MIT | Low | Well-maintained |
| tower-http | 0.6 | MIT | Low | Well-maintained |
| reqwest | 0.12 | MIT/Apache-2.0 | Low | Well-maintained |
| serde | 1.x | MIT/Apache-2.0 | Low | Well-maintained |
| serde_json | 1.x | MIT/Apache-2.0 | Low | Well-maintained |
| tracing | 0.1 | MIT | Low | Well-maintained |
| tracing-subscriber | 0.1 | MIT | Low | Well-maintained |
| clap | 4.x | MIT/Apache-2.0 | Low | Well-maintained |
| chrono | 0.4 | MIT/Apache-2.0 | Low | Well-maintained |
| uuid | 1.x | MIT/Apache-2.0 | Low | Well-maintained |
| rusqlite | 0.31 | MIT | Low | Bundled SQLite |

**Findings:** All dependencies are well-maintained, MIT/Apache-2.0 licensed, with low security risk.

---

## 3. Runtime Dependencies

| Dependency | Version | License | Risk | Notes |
|------------|---------|---------|------|-------|
| llama-server.exe | Current | MIT | Low | Well-maintained |
| Vulkan loader | System | MIT | Low | System library |
| SQLite | Bundled | Public Domain | Low | Bundled with rusqlite |

**Findings:** All runtime dependencies are low risk.

---

## 4. Operational Dependencies

| Dependency | Version | Risk | Notes |
|------------|---------|------|-------|
| PowerShell | 5.1+ | Low | System component |
| NSSM | Latest | Low | Well-maintained |

**Findings:** All operational dependencies are low risk.

---

## 5. Dependency Vulnerabilities

| Vulnerability | Dependency | Severity | Status |
|--------------|------------|----------|--------|
| None identified | — | — | — |

**Note:** Regular `cargo audit` should be run to identify new vulnerabilities.

---

## 6. References

- DEPENDENCY-MAP.md — Full dependency map
- SECURITY-BASELINE.md — Security baseline
- THREAT-MODEL.md — Threat model
