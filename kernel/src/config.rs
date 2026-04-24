use serde::{Deserialize, Serialize};
use crate::node::Node;
use dashmap::DashMap;
use crate::error::Result;

/// System configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Maximum number of nodes
    pub max_nodes: usize,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval: u64,
    /// Node timeout in milliseconds
    pub node_timeout: u64,
    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_nodes: 1000,
            heartbeat_interval: 1000,
            node_timeout: 5000,
            verbose: false,
        }
    }
}

/// System representation
pub struct System {
    config: Config,
    nodes: DashMap<String, Node>,
}

impl System {
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            config,
            nodes: DashMap::new(),
        })
    }
}