use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FleetHealth {
    pub total_nodes: u32,
    pub healthy_nodes: u32,
    pub degraded_nodes: u32,
    pub unhealthy_nodes: u32,
    pub online_nodes: u32,
    pub offline_nodes: u32,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FleetHealthBreakdown {
    pub component: String,
    pub healthy: u32,
    pub degraded: u32,
    pub unhealthy: u32,
    pub total: u32,
}
