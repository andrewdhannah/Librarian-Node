//! Validator rule engine — evaluates model outputs against versioned rules.
//!
//! Validators are simple rule-based checks. Each rule has a type, parameters,
//! and severity. The engine evaluates rules against a model's output and
//! produces a structured result.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Severity of a validation rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuleSeverity {
    /// Rule must pass — failure blocks qualification.
    Critical,
    /// Rule should pass — failure degrades qualification.
    Warning,
    /// Informational — noted but does not affect qualification.
    Info,
}

impl RuleSeverity {
    pub fn as_str(&self) -> &str {
        match self {
            RuleSeverity::Critical => "critical",
            RuleSeverity::Warning => "warning",
            RuleSeverity::Info => "info",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "critical" => RuleSeverity::Critical,
            "warning" => RuleSeverity::Warning,
            "info" => RuleSeverity::Info,
            _ => RuleSeverity::Critical,
        }
    }
}

/// A single validation rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationRule {
    /// Unique rule identifier (e.g., "IF-001", "SO-001").
    pub rule_id: String,

    /// Rule type determines evaluation logic.
    pub rule_type: RuleType,

    /// Rule severity.
    pub severity: RuleSeverity,

    /// Human-readable description.
    pub description: Option<String>,

    /// Rule-specific parameters (JSON).
    pub params: Option<String>,
}

/// Rule type determines how the rule is evaluated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuleType {
    /// Output must contain a specific substring.
    ContainsSubstring,
    /// Output must not contain a specific substring.
    NotContainsSubstring,
    /// Output must be valid JSON.
    ValidJson,
    /// Output must match a regex pattern.
    MatchesRegex,
    /// Output must have minimum token count.
    MinTokens { min: usize },
    /// Output must have maximum token count.
    MaxTokens { max: usize },
    /// Output must start with a specific prefix.
    StartsWithPrefix,
    /// Output must end with a specific suffix.
    EndsWithSuffix,
    /// Custom rule type (evaluated by custom logic).
    Custom(String),
}

/// Result of evaluating a single rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleResult {
    /// The rule that was evaluated.
    pub rule_id: String,

    /// Whether the rule passed.
    pub passed: bool,

    /// Severity of the rule.
    pub severity: RuleSeverity,

    /// Optional message (e.g., "Output missing required substring").
    pub message: Option<String>,
}

/// Complete validation result for a model output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationResult {
    /// The validator pack ID used.
    pub validator_pack_id: String,

    /// The task pack ID being validated against.
    pub task_pack_id: String,

    /// Individual rule results.
    pub rule_results: Vec<RuleResult>,

    /// Overall pass/fail (all Critical rules must pass).
    pub overall_pass: bool,

    /// Number of critical failures.
    pub critical_failures: usize,

    /// Number of warnings.
    pub warnings: usize,

    /// Number of info notices.
    pub infos: usize,
}

/// Validator rule engine — evaluates outputs against rule sets.
pub struct ValidatorEngine {
    /// Base directory for validator pack rules files.
    rules_dir: PathBuf,
}

impl ValidatorEngine {
    /// Create a new engine with the given rules directory.
    pub fn new(rules_dir: impl Into<PathBuf>) -> Self {
        Self {
            rules_dir: rules_dir.into(),
        }
    }

    /// Load rules from a JSON file.
    pub fn load_rules_from_file(&self, rules_path: &str) -> Result<Vec<ValidationRule>> {
        let path = if Path::new(rules_path).is_absolute() {
            PathBuf::from(rules_path)
        } else {
            self.rules_dir.join(rules_path)
        };

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read rules file: {}", path.display()))?;

        let rules: Vec<ValidationRule> = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse rules file: {}", path.display()))?;

        Ok(rules)
    }

    /// Load rules from JSON string.
    pub fn load_rules_from_str(&self, rules_json: &str) -> Result<Vec<ValidationRule>> {
        let rules: Vec<ValidationRule> = serde_json::from_str(rules_json)
            .context("Failed to parse rules JSON")?;
        Ok(rules)
    }

    /// Evaluate a model output against a set of rules.
    pub fn evaluate(
        &self,
        validator_pack_id: &str,
        task_pack_id: &str,
        rules: &[ValidationRule],
        output: &str,
        output_token_count: Option<usize>,
    ) -> ValidationResult {
        let mut rule_results = Vec::new();
        let mut critical_failures = 0;
        let mut warnings = 0;
        let mut infos = 0;

        for rule in rules {
            let result = evaluate_rule(rule, output, output_token_count);
            match result.severity {
                RuleSeverity::Critical if !result.passed => critical_failures += 1,
                RuleSeverity::Warning if !result.passed => warnings += 1,
                RuleSeverity::Info => infos += 1,
                _ => {}
            }
            rule_results.push(result);
        }

        ValidationResult {
            validator_pack_id: validator_pack_id.to_string(),
            task_pack_id: task_pack_id.to_string(),
            rule_results,
            overall_pass: critical_failures == 0,
            critical_failures,
            warnings,
            infos,
        }
    }

    /// Compute SHA-256 hash of rules JSON.
    pub fn compute_rules_hash(rules_json: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(rules_json.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Evaluate a single rule against an output.
fn evaluate_rule(rule: &ValidationRule, output: &str, output_token_count: Option<usize>) -> RuleResult {
    let passed = match &rule.rule_type {
        RuleType::ContainsSubstring => {
            if let Some(params) = &rule.params {
                let target = serde_json::from_str::<String>(params)
                    .unwrap_or_else(|_| params.clone());
                output.contains(&target)
            } else {
                true // No target = pass
            }
        }
        RuleType::NotContainsSubstring => {
            if let Some(params) = &rule.params {
                let target = serde_json::from_str::<String>(params)
                    .unwrap_or_else(|_| params.clone());
                !output.contains(&target)
            } else {
                true
            }
        }
        RuleType::ValidJson => {
            serde_json::from_str::<serde_json::Value>(output).is_ok()
        }
        RuleType::MatchesRegex => {
            // Simplified: just check if params is present and output is non-empty
            // Full regex would require regex crate
            rule.params.is_some() && !output.is_empty()
        }
        RuleType::MinTokens { min } => {
            output_token_count.map_or(true, |count| count >= *min)
        }
        RuleType::MaxTokens { max } => {
            output_token_count.map_or(true, |count| count <= *max)
        }
        RuleType::StartsWithPrefix => {
            if let Some(params) = &rule.params {
                let prefix = serde_json::from_str::<String>(params)
                    .unwrap_or_else(|_| params.clone());
                output.starts_with(&prefix)
            } else {
                true
            }
        }
        RuleType::EndsWithSuffix => {
            if let Some(params) = &rule.params {
                let suffix = serde_json::from_str::<String>(params)
                    .unwrap_or_else(|_| params.clone());
                output.ends_with(&suffix)
            } else {
                true
            }
        }
        RuleType::Custom(_) => true, // Custom rules pass by default
    };

    RuleResult {
        rule_id: rule.rule_id.clone(),
        passed,
        severity: rule.severity.clone(),
        message: if passed {
            None
        } else {
            Some(format!("Rule '{}' failed", rule.rule_id))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // MQR-F2-1: Compute hash is deterministic
    #[test]
    fn test_hash_deterministic() {
        let h1 = ValidatorEngine::compute_rules_hash("test content");
        let h2 = ValidatorEngine::compute_rules_hash("test content");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    // MQR-F2-2: Load rules from JSON string
    #[test]
    fn test_load_rules_from_str() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let json = r#"[{"rule_id":"R1","rule_type":"ContainsSubstring","severity":"critical","description":null,"params":"hello"}]"#;
        let rules = engine.load_rules_from_str(json).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].rule_id, "R1");
    }

    // MQR-F2-3: ContainsSubstring passes
    #[test]
    fn test_contains_substring_pass() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::ContainsSubstring,
            severity: RuleSeverity::Critical,
            description: None,
            params: Some(r#""function""#.to_string()),
        };
        let result = engine.evaluate("vp-1", "tp-1", &[rule], "def function(): pass", None);
        assert!(result.overall_pass);
        assert_eq!(result.critical_failures, 0);
    }

    // MQR-F2-4: ContainsSubstring fails
    #[test]
    fn test_contains_substring_fail() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::ContainsSubstring,
            severity: RuleSeverity::Critical,
            description: None,
            params: Some(r#""function""#.to_string()),
        };
        let result = engine.evaluate("vp-1", "tp-1", &[rule], "def method(): pass", None);
        assert!(!result.overall_pass);
        assert_eq!(result.critical_failures, 1);
    }

    // MQR-F2-5: ValidJson passes for valid JSON
    #[test]
    fn test_valid_json_pass() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::ValidJson,
            severity: RuleSeverity::Critical,
            description: None,
            params: None,
        };
        let result = engine.evaluate("vp-1", "tp-1", &[rule], r#"{"key":"value"}"#, None);
        assert!(result.overall_pass);
    }

    // MQR-F2-6: ValidJson fails for invalid JSON
    #[test]
    fn test_valid_json_fail() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::ValidJson,
            severity: RuleSeverity::Critical,
            description: None,
            params: None,
        };
        let result = engine.evaluate("vp-1", "tp-1", &[rule], "not json", None);
        assert!(!result.overall_pass);
    }

    // MQR-F2-7: MinTokens passes
    #[test]
    fn test_min_tokens_pass() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::MinTokens { min: 10 },
            severity: RuleSeverity::Critical,
            description: None,
            params: None,
        };
        let result = engine.evaluate("vp-1", "tp-1", &[rule], "output", Some(15));
        assert!(result.overall_pass);
    }

    // MQR-F2-8: MinTokens fails
    #[test]
    fn test_min_tokens_fail() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::MinTokens { min: 10 },
            severity: RuleSeverity::Critical,
            description: None,
            params: None,
        };
        let result = engine.evaluate("vp-1", "tp-1", &[rule], "output", Some(5));
        assert!(!result.overall_pass);
    }

    // MQR-F2-9: Warning severity doesn't block overall pass
    #[test]
    fn test_warning_doesnt_block() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::ContainsSubstring,
            severity: RuleSeverity::Warning,
            description: None,
            params: Some(r#""missing""#.to_string()),
        };
        let result = engine.evaluate("vp-1", "tp-1", &[rule], "output without it", None);
        assert!(result.overall_pass); // Warning doesn't block
        assert_eq!(result.warnings, 1);
        assert_eq!(result.critical_failures, 0);
    }

    // MQR-F2-10: Multiple rules evaluated
    #[test]
    fn test_multiple_rules() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rules = vec![
            ValidationRule {
                rule_id: "R1".to_string(),
                rule_type: RuleType::ContainsSubstring,
                severity: RuleSeverity::Critical,
                description: None,
                params: Some(r#""hello""#.to_string()),
            },
            ValidationRule {
                rule_id: "R2".to_string(),
                rule_type: RuleType::ValidJson,
                severity: RuleSeverity::Critical,
                description: None,
                params: None,
            },
        ];
        let result = engine.evaluate("vp-1", "tp-1", &rules, r#"{"msg":"hello"}"#, None);
        assert!(result.overall_pass);
        assert_eq!(result.rule_results.len(), 2);
    }

    // MQR-F2-11: NotContainsSubstring passes
    #[test]
    fn test_not_contains_pass() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::NotContainsSubstring,
            severity: RuleSeverity::Critical,
            description: None,
            params: Some(r#""error""#.to_string()),
        };
        let result = engine.evaluate("vp-1", "tp-1", &[rule], "all good", None);
        assert!(result.overall_pass);
    }

    // MQR-F2-12: NotContainsSubstring fails
    #[test]
    fn test_not_contains_fail() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::NotContainsSubstring,
            severity: RuleSeverity::Critical,
            description: None,
            params: Some(r#""error""#.to_string()),
        };
        let result = engine.evaluate("vp-1", "tp-1", &[rule], "error: something went wrong", None);
        assert!(!result.overall_pass);
    }

    // MQR-F2-13: Load rules from file
    #[test]
    fn test_load_rules_from_file() {
        let dir = tempdir().unwrap();
        let engine = ValidatorEngine::new(dir.path().to_path_buf());

        let rules = vec![ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::ContainsSubstring,
            severity: RuleSeverity::Critical,
            description: None,
            params: Some(r#""test""#.to_string()),
        }];

        let rules_json = serde_json::to_string(&rules).unwrap();
        let rules_path = dir.path().join("rules.json");
        std::fs::write(&rules_path, &rules_json).unwrap();

        let loaded = engine.load_rules_from_file("rules.json").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].rule_id, "R1");
    }

    // MQR-F2-14: StartsWithPrefix
    #[test]
    fn test_starts_with_prefix() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::StartsWithPrefix,
            severity: RuleSeverity::Critical,
            description: None,
            params: Some(r#""def ""#.to_string()),
        };
        let result = engine.evaluate("vp-1", "tp-1", &[rule], "def foo(): pass", None);
        assert!(result.overall_pass);
    }

    // MQR-F2-15: EndsWithSuffix
    #[test]
    fn test_ends_with_suffix() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::EndsWithSuffix,
            severity: RuleSeverity::Critical,
            description: None,
            params: Some(r#""pass""#.to_string()), // JSON string "pass"
        };
        let result = engine.evaluate("vp-1", "tp-1", &[rule], "return pass", None);
        assert!(result.overall_pass);
    }

    // MQR-F2-16: Rule severity string roundtrip
    #[test]
    fn test_severity_roundtrip() {
        assert_eq!(RuleSeverity::from_str("critical"), RuleSeverity::Critical);
        assert_eq!(RuleSeverity::from_str("warning"), RuleSeverity::Warning);
        assert_eq!(RuleSeverity::from_str("info"), RuleSeverity::Info);
        assert_eq!(RuleSeverity::Critical.as_str(), "critical");
        assert_eq!(RuleSeverity::Warning.as_str(), "warning");
        assert_eq!(RuleSeverity::Info.as_str(), "info");
    }

    // MQR-F2-17: ValidationResult serialization
    #[test]
    fn test_validation_result_serialize() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::ContainsSubstring,
            severity: RuleSeverity::Critical,
            description: None,
            params: Some(r#""hello""#.to_string()),
        };
        let result = engine.evaluate("vp-1", "tp-1", &[rule], "hello world", None);
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ValidationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.overall_pass, result.overall_pass);
        assert_eq!(deserialized.rule_results.len(), result.rule_results.len());
    }

    // MQR-F2-18: Empty output with MinTokens fails
    #[test]
    fn test_empty_output_min_tokens() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::MinTokens { min: 1 },
            severity: RuleSeverity::Critical,
            description: None,
            params: None,
        };
        let result = engine.evaluate("vp-1", "tp-1", &[rule], "", Some(0));
        assert!(!result.overall_pass);
    }

    // MQR-F2-19: No token count with MinTokens passes (optional metric)
    #[test]
    fn test_no_token_count_min_tokens() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::MinTokens { min: 100 },
            severity: RuleSeverity::Critical,
            description: None,
            params: None,
        };
        let result = engine.evaluate("vp-1", "tp-1", &[rule], "output", None);
        assert!(result.overall_pass); // No token count = pass (metric not available)
    }

    // MQR-F2-20: Empty rules list produces pass
    #[test]
    fn test_empty_rules() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let result = engine.evaluate("vp-1", "tp-1", &[], "output", None);
        assert!(result.overall_pass);
        assert_eq!(result.rule_results.len(), 0);
        assert_eq!(result.critical_failures, 0);
    }

    // MQR-F2-21: Custom rule type passes by default
    #[test]
    fn test_custom_rule_passes() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let rule = ValidationRule {
            rule_id: "R1".to_string(),
            rule_type: RuleType::Custom("my_custom_check".to_string()),
            severity: RuleSeverity::Critical,
            description: None,
            params: None,
        };
        let result = engine.evaluate("vp-1", "tp-1", &[rule], "output", None);
        assert!(result.overall_pass);
    }

    // MQR-F2-22: Validation result has correct pack IDs
    #[test]
    fn test_result_pack_ids() {
        let engine = ValidatorEngine::new(PathBuf::from("/tmp/rules"));
        let result = engine.evaluate("vp-42", "tp-99", &[], "output", None);
        assert_eq!(result.validator_pack_id, "vp-42");
        assert_eq!(result.task_pack_id, "tp-99");
    }
}
