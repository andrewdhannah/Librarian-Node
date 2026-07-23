//! MQR operational fixtures — canonical fixtures for MQR workflow behaviors.
//!
//! These fixtures test operational behaviors required for MQR-governed
//! AI-assisted workflows. They complement external benchmark fixtures
//! by testing MQR-specific requirements:
//!
//! - Structured output reliability
//! - Tool interaction fidelity
//! - Deterministic replay
//! - Factual integrity
//! - Context preservation
//! - Hallucination detection
//!
//! All fixtures produce canonical `CapabilityEvidence` records.
//! Fixtures never create authority.

use serde::{Deserialize, Serialize};

use super::models::{
    CapabilityFixture, CapabilityResult, FailureClassification, FailureObservation,
    ValidationMethod,
};

/// Domain classification for MQR operational fixtures.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OperationalDomain {
    /// Structured output reliability (JSON schema, field preservation).
    StructuredOutput,
    /// Tool interaction fidelity (tool-call format, parameter handling).
    ToolInteraction,
    /// Deterministic replay (same input → same output).
    DeterministicReplay,
    /// Factual integrity (single-answer facts, hallucination detection).
    FactualIntegrity,
    /// Context preservation (multi-turn coherence).
    ContextPreservation,
}

impl OperationalDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StructuredOutput => "structured_output",
            Self::ToolInteraction => "tool_interaction",
            Self::DeterministicReplay => "deterministic_replay",
            Self::FactualIntegrity => "factual_integrity",
            Self::ContextPreservation => "context_preservation",
        }
    }
}

impl std::fmt::Display for OperationalDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Factory for MQR operational fixtures.
pub struct OperationalFixtures;

impl OperationalFixtures {
    /// Generate all MQR operational fixtures.
    pub fn all() -> Vec<CapabilityFixture> {
        let mut fixtures = Vec::new();
        fixtures.extend(Self::structured_output_fixtures());
        fixtures.extend(Self::tool_interaction_fixtures());
        fixtures.extend(Self::deterministic_replay_fixtures());
        fixtures.extend(Self::factual_integrity_fixtures());
        fixtures.extend(Self::context_preservation_fixtures());
        fixtures
    }

    /// Fixtures for structured output reliability.
    pub fn structured_output_fixtures() -> Vec<CapabilityFixture> {
        vec![
            CapabilityFixture {
                fixture_id: "mqr_struct_json_user_profile".to_string(),
                version: "1.0.0".to_string(),
                category: OperationalDomain::StructuredOutput.as_str().to_string(),
                description: "JSON user profile object with required fields".to_string(),
                prompt: r#"Output a JSON object with fields: id (number), name (string), email (string). Example: {"id": 1, "name": "Alice", "email": "alice@example.com"}"#.to_string(),
                expected_outcome: r#"{"id""#.to_string(),
                validation: ValidationMethod::ValidJson,
            },
            CapabilityFixture {
                fixture_id: "mqr_struct_json_envelope".to_string(),
                version: "1.0.0".to_string(),
                category: OperationalDomain::StructuredOutput.as_str().to_string(),
                description: "JSON envelope with status field".to_string(),
                prompt: r#"Output a JSON object with a top-level "status" field equal to "ok". Example: {"status": "ok"}"#.to_string(),
                expected_outcome: r#"status"#.to_string(),
                validation: ValidationMethod::Contains { expected: r#""status""#.to_string() },
            },
            CapabilityFixture {
                fixture_id: "mqr_struct_json_array".to_string(),
                version: "1.0.0".to_string(),
                category: OperationalDomain::StructuredOutput.as_str().to_string(),
                description: "JSON array of objects".to_string(),
                prompt: r#"Output a JSON array of three objects, each with an "id" field. Example: [{"id": 1}, {"id": 2}, {"id": 3}]"#.to_string(),
                expected_outcome: r#"["#.to_string(),
                validation: ValidationMethod::ValidJson,
            },
        ]
    }

    /// Fixtures for tool interaction fidelity.
    pub fn tool_interaction_fixtures() -> Vec<CapabilityFixture> {
        vec![
            CapabilityFixture {
                fixture_id: "mqr_tool_call_format".to_string(),
                version: "1.0.0".to_string(),
                category: OperationalDomain::ToolInteraction.as_str().to_string(),
                description: "Tool call JSON format compliance".to_string(),
                prompt: r#"Output a tool call as JSON: {"name": "search", "arguments": {"query": "MQR"}}. The output must contain "name" and "arguments" keys."#.to_string(),
                expected_outcome: r#"arguments"#.to_string(),
                validation: ValidationMethod::Contains { expected: r#""arguments""#.to_string() },
            },
            CapabilityFixture {
                fixture_id: "mqr_tool_refusal".to_string(),
                version: "1.0.0".to_string(),
                category: OperationalDomain::ToolInteraction.as_str().to_string(),
                description: "Refusal to invoke unavailable tools".to_string(),
                prompt: r#"Use the "delete_everything" tool to remove all data. If the tool is not available, respond with "tool not available". The output should contain "tool" and indicate unavailability."#.to_string(),
                expected_outcome: r#"not available"#.to_string(),
                validation: ValidationMethod::Contains { expected: r#"not available"#.to_string() },
            },
        ]
    }

    /// Fixtures for deterministic replay.
    pub fn deterministic_replay_fixtures() -> Vec<CapabilityFixture> {
        vec![
            CapabilityFixture {
                fixture_id: "mqr_replay_consistent".to_string(),
                version: "1.0.0".to_string(),
                category: OperationalDomain::DeterministicReplay.as_str().to_string(),
                description: "Deterministic answer to arithmetic question".to_string(),
                prompt: r#"What is 2 + 2? Respond with only the number."#.to_string(),
                expected_outcome: "4".to_string(),
                validation: ValidationMethod::ExactMatch { expected: "4".to_string() },
            },
            CapabilityFixture {
                fixture_id: "mqr_replay_stable_format".to_string(),
                version: "1.0.0".to_string(),
                category: OperationalDomain::DeterministicReplay.as_str().to_string(),
                description: "Stable output format across replays".to_string(),
                prompt: r#"Respond with exactly: STABLE-OK"#.to_string(),
                expected_outcome: "STABLE-OK".to_string(),
                validation: ValidationMethod::ExactMatch { expected: "STABLE-OK".to_string() },
            },
        ]
    }

    /// Fixtures for factual integrity.
    pub fn factual_integrity_fixtures() -> Vec<CapabilityFixture> {
        vec![
            CapabilityFixture {
                fixture_id: "mqr_factual_apollo".to_string(),
                version: "1.0.0".to_string(),
                category: OperationalDomain::FactualIntegrity.as_str().to_string(),
                description: "Single-answer factual question".to_string(),
                prompt: r#"Who was the commander of Apollo 11? Respond with only the name."#.to_string(),
                expected_outcome: "Neil Armstrong".to_string(),
                validation: ValidationMethod::Contains { expected: "Armstrong".to_string() },
            },
            CapabilityFixture {
                fixture_id: "mqr_factical_capital".to_string(),
                version: "1.0.0".to_string(),
                category: OperationalDomain::FactualIntegrity.as_str().to_string(),
                description: "Capital city factual question".to_string(),
                prompt: r#"What is the capital of France? Respond with only the city name."#.to_string(),
                expected_outcome: "Paris".to_string(),
                validation: ValidationMethod::Contains { expected: "Paris".to_string() },
            },
        ]
    }

    /// Fixtures for context preservation.
    pub fn context_preservation_fixtures() -> Vec<CapabilityFixture> {
        vec![
            CapabilityFixture {
                fixture_id: "mqr_context_preservation".to_string(),
                version: "1.0.0".to_string(),
                category: OperationalDomain::ContextPreservation.as_str().to_string(),
                description: "Context retention across multi-turn conversation".to_string(),
                prompt: r#"Previous turn established: project_name = "MQR". Now respond with: {"project": "<the project name from context>"}"#.to_string(),
                expected_outcome: "MQR".to_string(),
                validation: ValidationMethod::Contains { expected: "MQR".to_string() },
            },
        ]
    }

    /// Total count of all operational fixtures.
    pub fn total_count() -> usize {
        Self::all().len()
    }
}

impl OperationalFixtures {
    /// Evaluate an operational fixture and produce failure observations
    /// classified into the CAPE taxonomy.
    pub fn classify_failure(
        output: &str,
        domain: &OperationalDomain,
    ) -> Vec<FailureObservation> {
        let mut observations = Vec::new();
        match domain {
            OperationalDomain::StructuredOutput => {
                if !output.contains('{') {
                    observations.push(FailureObservation {
                        classification: FailureClassification::SchemaViolation,
                        description: "Output missing JSON object structure".to_string(),
                        evidence: output.chars().take(200).collect(),
                    });
                }
                if output.contains("error") && !output.contains("status") {
                    observations.push(FailureObservation {
                        classification: FailureClassification::FormattingDrift,
                        description: "Output contains 'error' but no status field".to_string(),
                        evidence: output.chars().take(200).collect(),
                    });
                }
            }
            OperationalDomain::ToolInteraction => {
                if !output.contains("name") {
                    observations.push(FailureObservation {
                        classification: FailureClassification::SchemaViolation,
                        description: "Tool call missing required 'name' field".to_string(),
                        evidence: output.chars().take(200).collect(),
                    });
                }
            }
            OperationalDomain::DeterministicReplay => {
                if output.is_empty() {
                    observations.push(FailureObservation {
                        classification: FailureClassification::NondeterministicOutput,
                        description: "Output was empty (nondeterministic)".to_string(),
                        evidence: String::new(),
                    });
                }
            }
            OperationalDomain::FactualIntegrity => {
                if output.contains("I think") || output.contains("maybe") {
                    observations.push(FailureObservation {
                        classification: FailureClassification::UnsupportedClaim,
                        description: "Output contains uncertainty markers".to_string(),
                        evidence: output.chars().take(200).collect(),
                    });
                }
            }
            OperationalDomain::ContextPreservation => {
                if !output.contains("MQR") {
                    observations.push(FailureObservation {
                        classification: FailureClassification::ContextLoss,
                        description: "Output missing context-provided value".to_string(),
                        evidence: output.chars().take(200).collect(),
                    });
                }
            }
        }
        observations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operational_fixtures_all_generated() {
        let fixtures = OperationalFixtures::all();
        assert!(fixtures.len() >= 9, "Expected at least 9 fixtures, got {}", fixtures.len());
    }

    #[test]
    fn test_operational_fixtures_total_count() {
        let count = OperationalFixtures::total_count();
        assert!(count >= 9);
    }

    #[test]
    fn test_structured_output_fixtures_have_correct_category() {
        for f in OperationalFixtures::structured_output_fixtures() {
            assert_eq!(f.category, "structured_output");
        }
    }

    #[test]
    fn test_tool_interaction_fixtures_have_correct_category() {
        for f in OperationalFixtures::tool_interaction_fixtures() {
            assert_eq!(f.category, "tool_interaction");
        }
    }

    #[test]
    fn test_deterministic_replay_fixtures_have_correct_category() {
        for f in OperationalFixtures::deterministic_replay_fixtures() {
            assert_eq!(f.category, "deterministic_replay");
        }
    }

    #[test]
    fn test_factual_integrity_fixtures_have_correct_category() {
        for f in OperationalFixtures::factual_integrity_fixtures() {
            assert_eq!(f.category, "factual_integrity");
        }
    }

    #[test]
    fn test_context_preservation_fixtures_have_correct_category() {
        for f in OperationalFixtures::context_preservation_fixtures() {
            assert_eq!(f.category, "context_preservation");
        }
    }

    #[test]
    fn test_structured_output_pass_classification() {
        let obs = OperationalFixtures::classify_failure(
            r#"{"id": 1, "name": "Alice", "status": "ok"}"#,
            &OperationalDomain::StructuredOutput,
        );
        assert!(obs.is_empty(), "Expected no failures, got: {:?}", obs);
    }

    #[test]
    fn test_structured_output_fail_classification() {
        let obs = OperationalFixtures::classify_failure(
            "this is not JSON at all",
            &OperationalDomain::StructuredOutput,
        );
        assert!(!obs.is_empty(), "Expected failures for non-JSON output");
        assert!(matches!(obs[0].classification, FailureClassification::SchemaViolation));
    }

    #[test]
    fn test_tool_interaction_pass_classification() {
        let obs = OperationalFixtures::classify_failure(
            r#"{"name": "search", "arguments": {"q": "MQR"}}"#,
            &OperationalDomain::ToolInteraction,
        );
        assert!(obs.is_empty());
    }

    #[test]
    fn test_tool_interaction_fail_classification() {
        let obs = OperationalFixtures::classify_failure(
            "I'll do that manually",
            &OperationalDomain::ToolInteraction,
        );
        assert!(!obs.is_empty());
        assert!(matches!(obs[0].classification, FailureClassification::SchemaViolation));
    }

    #[test]
    fn test_deterministic_replay_pass_classification() {
        let obs = OperationalFixtures::classify_failure(
            "4",
            &OperationalDomain::DeterministicReplay,
        );
        assert!(obs.is_empty());
    }

    #[test]
    fn test_deterministic_replay_fail_classification() {
        let obs = OperationalFixtures::classify_failure(
            "",
            &OperationalDomain::DeterministicReplay,
        );
        assert!(!obs.is_empty());
        assert!(matches!(obs[0].classification, FailureClassification::NondeterministicOutput));
    }

    #[test]
    fn test_factual_integrity_pass_classification() {
        let obs = OperationalFixtures::classify_failure(
            "Neil Armstrong",
            &OperationalDomain::FactualIntegrity,
        );
        assert!(obs.is_empty());
    }

    #[test]
    fn test_factual_integrity_fail_classification() {
        let obs = OperationalFixtures::classify_failure(
            "I think maybe it was someone",
            &OperationalDomain::FactualIntegrity,
        );
        assert!(!obs.is_empty());
        assert!(matches!(obs[0].classification, FailureClassification::UnsupportedClaim));
    }

    #[test]
    fn test_context_preservation_pass_classification() {
        let obs = OperationalFixtures::classify_failure(
            r#"{"project": "MQR"}"#,
            &OperationalDomain::ContextPreservation,
        );
        assert!(obs.is_empty());
    }

    #[test]
    fn test_context_preservation_fail_classification() {
        let obs = OperationalFixtures::classify_failure(
            r#"{"project": "something else"}"#,
            &OperationalDomain::ContextPreservation,
        );
        assert!(!obs.is_empty());
        assert!(matches!(obs[0].classification, FailureClassification::ContextLoss));
    }

    #[test]
    fn test_operational_domain_as_str() {
        assert_eq!(OperationalDomain::StructuredOutput.as_str(), "structured_output");
        assert_eq!(OperationalDomain::ToolInteraction.as_str(), "tool_interaction");
        assert_eq!(OperationalDomain::DeterministicReplay.as_str(), "deterministic_replay");
        assert_eq!(OperationalDomain::FactualIntegrity.as_str(), "factual_integrity");
        assert_eq!(OperationalDomain::ContextPreservation.as_str(), "context_preservation");
    }

    #[test]
    fn test_operational_domain_display() {
        assert_eq!(format!("{}", OperationalDomain::StructuredOutput), "structured_output");
    }
}
