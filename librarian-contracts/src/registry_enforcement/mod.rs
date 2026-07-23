pub mod enforcement_rule;
pub mod enforcement_event;

pub use enforcement_rule::*;
pub use enforcement_event::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnforcementPolicy {
    pub rules: Vec<EnforcementRule>,
    pub version: u32,
}
