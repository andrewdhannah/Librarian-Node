/// Validator pack — Mac canonical.
///
/// Versioned validation rules for qualification runs.
/// Each validator_pack captures the rules that evaluate a model's output
/// for a specific role.

/// Validator pack status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatorPackStatus {
    /// Active — available for validation.
    Active,
    /// Deprecated — no longer used for new validations.
    Deprecated,
}

impl ValidatorPackStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ValidatorPackStatus::Active => "active",
            ValidatorPackStatus::Deprecated => "deprecated",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => ValidatorPackStatus::Active,
            "deprecated" => ValidatorPackStatus::Deprecated,
            _ => ValidatorPackStatus::Active,
        }
    }
}

/// Validator pack — versioned validation rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorPack {
    /// Unique identifier for this validator pack.
    pub validator_pack_id: String,

    /// Version number (monotonically increasing).
    pub version: i64,

    /// Target role for validation (e.g., "implementer", "researcher").
    pub role: String,

    /// Human-readable description.
    pub description: Option<String>,

    /// SHA-256 hash of the rules content.
    pub rules_hash: String,

    /// Filesystem path to the rules file.
    pub rules_path: Option<String>,

    /// Validator pack status.
    pub status: ValidatorPackStatus,

    /// When this validator pack was created.
    pub created_at: String,
}

impl ValidatorPack {
    /// Create a new validator pack.
    pub fn new(
        validator_pack_id: String,
        version: i64,
        role: String,
        rules_hash: String,
    ) -> Self {
        Self {
            validator_pack_id,
            version,
            role,
            description: None,
            rules_hash,
            rules_path: None,
            status: ValidatorPackStatus::Active,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_validator_pack() {
        let pack = ValidatorPack::new(
            "vp-1".to_string(),
            1,
            "implementer".to_string(),
            "def456".to_string(),
        );
        assert_eq!(pack.validator_pack_id, "vp-1");
        assert_eq!(pack.version, 1);
        assert_eq!(pack.role, "implementer");
        assert_eq!(pack.status, ValidatorPackStatus::Active);
    }

    #[test]
    fn test_validator_pack_status_roundtrip() {
        let active = ValidatorPackStatus::Active;
        assert_eq!(ValidatorPackStatus::from_str(active.as_str()), active);

        let deprecated = ValidatorPackStatus::Deprecated;
        assert_eq!(ValidatorPackStatus::from_str(deprecated.as_str()), deprecated);
    }
}
