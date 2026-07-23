use serde::{Deserialize, Serialize};

use super::fleet_health::FleetHealth;
use super::node_inventory::{FleetInventory, NodeInventoryEntry};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FleetOverview {
    pub inventory: FleetInventory,
    pub health: FleetHealth,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryScanRequest {
    pub scan_id: String,
    pub initiated_by: String,
    pub scan_method: String,
    pub initiated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryScanResult {
    pub scan_id: String,
    pub nodes_found: u32,
    pub nodes: Vec<NodeInventoryEntry>,
    pub new_nodes: Vec<String>,
    pub completed_at: String,
}
