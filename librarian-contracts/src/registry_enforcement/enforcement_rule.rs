use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RuleScope {
    Registration,
    Capability,
    Candidate,
    Evidence,
}

impl std::fmt::Display for RuleScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleScope::Registration => write!(f, "registration"),
            RuleScope::Capability => write!(f, "capability"),
            RuleScope::Candidate => write!(f, "candidate"),
            RuleScope::Evidence => write!(f, "evidence"),
        }
    }
}

impl From<&str> for RuleScope {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "registration" => RuleScope::Registration,
            "capability" => RuleScope::Capability,
            "candidate" => RuleScope::Candidate,
            "evidence" => RuleScope::Evidence,
            _ => RuleScope::Registration,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EnforcementAction {
    Block,
    Degrade,
    Warn,
    Log,
}

impl std::fmt::Display for EnforcementAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnforcementAction::Block => write!(f, "block"),
            EnforcementAction::Degrade => write!(f, "degrade"),
            EnforcementAction::Warn => write!(f, "warn"),
            EnforcementAction::Log => write!(f, "log"),
        }
    }
}

impl From<&str> for EnforcementAction {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "block" => EnforcementAction::Block,
            "degrade" => EnforcementAction::Degrade,
            "warn" => EnforcementAction::Warn,
            "log" => EnforcementAction::Log,
            _ => EnforcementAction::Log,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnforcementRule {
    pub rule_id: String,
    pub name: String,
    pub scope: RuleScope,
    pub condition: serde_json::Value,
    pub action: EnforcementAction,
    pub enabled: bool,
}
