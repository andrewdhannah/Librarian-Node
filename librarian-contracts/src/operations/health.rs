use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeHealth {
    pub overall_status: String,
    pub components: Vec<ComponentHealth>,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComponentHealth {
    pub component: String,
    pub status: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthSummary {
    pub status: String,
    pub healthy_count: u32,
    pub degraded_count: u32,
    pub unhealthy_count: u32,
    pub total_components: u32,
}
