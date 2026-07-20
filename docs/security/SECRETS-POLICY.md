# Secrets Policy

**Status:** Active  
**Repository:** Librarian-Windows-Runtime-Node  

---

## 1. Purpose

Define how secrets are handled in the Windows Runtime Node repository. This repository contains audit artifacts and documentation only — no secrets should ever be committed.

---

## 2. Secret Types

| Type | Example | Risk if Exposed |
|------|---------|-----------------|
| API keys | `sk-...` (OpenAI), `anthropic-...` | Unauthorized API usage |
| Tokens | GitHub PAT, MCP tokens | Unauthorized access |
| Private keys | RSA, ECDSA keys | Identity compromise |
| Certificates | `.pfx`, `.pem` files | TLS compromise |
| Connection strings | Database URLs | Data access |
| Passwords | Service account passwords | System access |

---

## 3. Prohibited Locations

Secrets must NOT appear in:
- Source code (`.rs`, `.ps1`, `.js`, `.py`)
- Configuration files (`.json`, `.toml`, `.yaml`, `.xml`)
- Documentation (`.md`, `.txt`)
- Log files
- Evidence files
- Issue comments or descriptions
- Commit messages

---

## 4. Allowed Storage

Secrets may be stored in:
- Environment variables
- Windows Credential Manager
- Secure vault services (e.g., HashiCorp Vault)
- Encrypted configuration files (outside repository)

---

## 5. Redaction Rules

If a secret must be referenced in documentation:

```json
{
  "connection_string": "REDACTED",
  "api_key": "REDACTED",
  "token": "REDACTED"
}
```

**Never include the actual secret value.**

---

## 6. Incident Response

If a secret is accidentally committed:

1. **Immediately** revoke the secret
2. Remove the secret from the repository history
3. Rotate the secret
4. Record incident evidence
5. Update processes to prevent recurrence

---

## 7. Commit Checklist

Before every commit:

- [ ] No API keys in code
- [ ] No tokens in code
- [ ] No passwords in code
- [ ] No connection strings with credentials
- [ ] No certificates
- [ ] No secrets in documentation
- [ ] `.gitignore` covers secret patterns

---

## 8. References

- SECURITY-BASELINE.md — Security baseline
- THREAT-MODEL.md — Threat model
- `.gitignore` — Git ignore patterns
