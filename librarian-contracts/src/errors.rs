//! # Contract Error Types
//!
//! Typed error definitions for the contract layer.
//! These errors are contract-level — they indicate contract violations,
//! not runtime failures.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Schema version for error contracts.
pub const ERROR_CONTRACT_VERSION: &str = "1.0.0";

/// A contract-level error.
/// Maps to typed error enums from Swift (e.g., `MCPCustodyError`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "detail")]
pub enum ContractError {
    /// A required field is missing.
    MissingField {
        /// Name of the missing field.
        field: String,
        /// Context about what was being validated.
        context: String,
    },
    /// A field has an invalid value.
    InvalidField {
        /// Name of the field.
        field: String,
        /// The invalid value.
        value: String,
        /// What was expected.
        expected: String,
    },
    /// A contract version mismatch.
    VersionMismatch {
        /// Expected version.
        expected: String,
        /// Actual version found.
        actual: String,
    },
    /// A required contract type was not found.
    MissingContract {
        /// Name of the missing contract.
        contract: String,
    },
    /// A serialization error occurred.
    SerializationError {
        /// Description of the error.
        description: String,
    },
    /// A validation error occurred.
    ValidationError {
        /// Description of the error.
        description: String,
        /// Number of validation failures.
        failure_count: u32,
    },
    /// An identity verification error.
    IdentityError {
        /// Node ID that failed verification.
        node_id: String,
        /// Reason for failure.
        reason: String,
    },
    /// A custody operation error.
    CustodyError {
        /// The custody action attempted.
        action: String,
        /// Reason for failure.
        reason: String,
    },
    /// An authority check error.
    AuthorityError {
        /// The action that was denied.
        action: String,
        /// Current authority level.
        current_authority: String,
        /// Required authority level.
        required_authority: String,
    },
}

impl ContractError {
    /// Whether this error indicates the operation should be retried.
    pub fn is_retryable(&self) -> bool {
        matches!(self, ContractError::SerializationError { .. })
    }

    /// Human-readable summary.
    pub fn summary(&self) -> String {
        match self {
            ContractError::MissingField { field, context } => {
                format!("Missing field '{}' in {}", field, context)
            }
            ContractError::InvalidField { field, value, expected } => {
                format!("Invalid field '{}': got '{}', expected {}", field, value, expected)
            }
            ContractError::VersionMismatch { expected, actual } => {
                format!("Version mismatch: expected {}, got {}", expected, actual)
            }
            ContractError::MissingContract { contract } => {
                format!("Missing contract: {}", contract)
            }
            ContractError::SerializationError { description } => {
                format!("Serialization error: {}", description)
            }
            ContractError::ValidationError { description, failure_count } => {
                format!("Validation error ({} failures): {}", failure_count, description)
            }
            ContractError::IdentityError { node_id, reason } => {
                format!("Identity error for node {}: {}", node_id, reason)
            }
            ContractError::CustodyError { action, reason } => {
                format!("Custody error on '{}': {}", action, reason)
            }
            ContractError::AuthorityError { action, current_authority, required_authority } => {
                format!("Authority error for '{}': has {}, requires {}", action, current_authority, required_authority)
            }
        }
    }
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.summary())
    }
}

impl std::error::Error for ContractError {}

/// A validation result — either passes or contains a list of errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed.
    pub passed: bool,
    /// List of validation errors (empty if passed).
    pub errors: Vec<ValidationError>,
    /// Number of checks performed.
    pub checks_performed: u32,
}

/// A single validation error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Field or check name.
    pub field: String,
    /// Error message.
    pub message: String,
    /// Severity.
    pub severity: ValidationSeverity,
}

/// Severity of a validation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationSeverity {
    /// Error — blocks acceptance.
    Error,
    /// Warning — acceptable with review.
    Warning,
    /// Info — informational.
    Info,
}

/// A typed result for contract operations.
pub type ContractResult<T> = Result<T, ContractError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_error_display() {
        let err = ContractError::MissingField {
            field: "node_id".into(),
            context: "NodeIdentity".into(),
        };
        assert!(err.summary().contains("node_id"));
        assert!(err.summary().contains("NodeIdentity"));
    }

    #[test]
    fn test_validation_result() {
        let result = ValidationResult {
            passed: false,
            errors: vec![
                ValidationError {
                    field: "contract_version".into(),
                    message: "Version mismatch".into(),
                    severity: ValidationSeverity::Error,
                },
            ],
            checks_performed: 5,
        };
        assert!(!result.passed);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_retryable() {
        let ser_err = ContractError::SerializationError {
            description: "JSON parse error".into(),
        };
        assert!(ser_err.is_retryable());

        let missing = ContractError::MissingField {
            field: "id".into(),
            context: "test".into(),
        };
        assert!(!missing.is_retryable());
    }

    #[test]
    fn test_contract_result_type() {
        let ok: ContractResult<i32> = Ok(42);
        assert!(ok.is_ok());

        let err: ContractResult<i32> = Err(ContractError::MissingContract {
            contract: "Receipt".into(),
        });
        assert!(err.is_err());
    }
}
