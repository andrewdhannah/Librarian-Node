/// Model identity record — Mac canonical.
///
/// Extends Windows local_models with qualification-specific identity:
/// GGUF metadata hash, chat template ID, license SPDX, qualification scope.
///
/// This is the authoritative identity for qualification results. Every
/// qualification_run, capability_manifest, and router_projection references
/// this record.

/// Qualification scope: what this identity is being qualified for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualificationScope {
    /// Full qualification — all roles.
    Full,
    /// Specific roles only.
    Roles(Vec<String>),
    /// Experimental — no commitment.
    Experimental,
}

impl QualificationScope {
    pub fn as_str(&self) -> &str {
        match self {
            QualificationScope::Full => "full",
            QualificationScope::Roles(_) => "roles",
            QualificationScope::Experimental => "experimental",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "full" => QualificationScope::Full,
            "experimental" => QualificationScope::Experimental,
            // "roles" — caller should pass roles_json separately and use from_str_with_roles
            "roles" => QualificationScope::Roles(vec![]),
            _ => QualificationScope::Full,
        }
    }

    /// Parse scope from string + roles_json.
    pub fn from_str_with_roles(s: &str, roles_json: Option<&str>) -> Self {
        match s {
            "roles" => {
                if let Some(json) = roles_json {
                    if let Ok(roles) = serde_json::from_str::<Vec<String>>(json) {
                        QualificationScope::Roles(roles)
                    } else {
                        QualificationScope::Roles(vec![])
                    }
                } else {
                    QualificationScope::Roles(vec![])
                }
            }
            "full" => QualificationScope::Full,
            "experimental" => QualificationScope::Experimental,
            _ => QualificationScope::Full,
        }
    }
}

/// Model identity record — Mac canonical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelIdentityRecord {
    /// Unique identifier for this identity record.
    pub identity_id: String,

    /// Reference to Windows local_models.model_id (cross-node FK).
    pub model_id_ref: String,

    /// SHA-256 of GGUF metadata section (hex).
    /// None if GGUF metadata not yet extracted.
    pub gguf_metadata_hash: Option<String>,

    /// Chat template identifier (normalized name or hash).
    /// None if chat template not yet characterized.
    pub chat_template_id: Option<String>,

    /// License SPDX identifier (e.g., "Apache-2.0", "MIT").
    /// None if license not yet identified.
    pub license_spdx: Option<String>,

    /// Qualification scope for this identity.
    pub qualification_scope: QualificationScope,

    /// Roles JSON (when scope is "roles"): ["implementer", "researcher"]
    pub roles_json: Option<String>,

    /// When this identity record was created.
    pub created_at: String,

    /// When this identity record was last updated.
    pub updated_at: String,
}

impl ModelIdentityRecord {
    /// Create a new identity record.
    pub fn new(identity_id: String, model_id_ref: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            identity_id,
            model_id_ref,
            gguf_metadata_hash: None,
            chat_template_id: None,
            license_spdx: None,
            qualification_scope: QualificationScope::Full,
            roles_json: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_identity_record() {
        let record = ModelIdentityRecord::new(
            "id-1".to_string(),
            "minicpm5-1b-q4km".to_string(),
        );
        assert_eq!(record.identity_id, "id-1");
        assert_eq!(record.model_id_ref, "minicpm5-1b-q4km");
        assert!(record.gguf_metadata_hash.is_none());
        assert!(record.chat_template_id.is_none());
        assert!(record.license_spdx.is_none());
        assert_eq!(record.qualification_scope, QualificationScope::Full);
    }

    #[test]
    fn test_qualification_scope_roundtrip() {
        let full = QualificationScope::Full;
        assert_eq!(QualificationScope::from_str(full.as_str()), full);

        let exp = QualificationScope::Experimental;
        assert_eq!(QualificationScope::from_str(exp.as_str()), exp);
    }

    #[test]
    fn test_roles_scope() {
        let roles = QualificationScope::Roles(vec![
            "implementer".to_string(),
            "researcher".to_string(),
        ]);
        assert_eq!(roles.as_str(), "roles");
    }
}
