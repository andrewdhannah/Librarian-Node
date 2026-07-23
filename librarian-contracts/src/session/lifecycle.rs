use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionState {
    Created,
    Active,
    Closed,
    Expired,
}

impl SessionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionState::Created => "created",
            SessionState::Active => "active",
            SessionState::Closed => "closed",
            SessionState::Expired => "expired",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "created" => Some(SessionState::Created),
            "active" => Some(SessionState::Active),
            "closed" => Some(SessionState::Closed),
            "expired" => Some(SessionState::Expired),
            _ => None,
        }
    }
}
