use crate::{error::Result, config::Config};
use crate::services::ServiceRegistry;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use chrono;
use serde_json;

/// Node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node UUID
    pub id: Uuid,
    
    /// Node name
    pub name: String,
    
    /// Node type
    pub node_type: String,
    
    /// Node status
    pub status: NodeStatus,
    
    /// Node capabilities
    pub capabilities: Vec<String>,
    
    /// Node metadata
    pub metadata: serde_json::Value,
    
    /// Creation time
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// Last heartbeat
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}

/// Node status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeStatus {
    Initializing,
    Running,
    Paused,
    Error(String),
    Shutdown,
}

/// Node handle for external references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHandle {
    pub id: Uuid,
    pub name: String,
}

/// Node implementation
pub struct Node {
    info: RwLock<NodeInfo>,
    services: ServiceRegistry,
    config: Arc<Config>,
}

impl Node {
    /// Create a new node
    pub fn new(name: String, node_type: String, config: Config) -> Self {
        let info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type,
            status: NodeStatus::Initializing,
            capabilities: Vec::new(),
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
        };
        
        Self {
            info: RwLock::new(info),
            services: ServiceRegistry::new(config.clone()),
            config: Arc::new(config),
        }
    }
    
    /// Start the node
    pub async fn start(&self) -> Result<()> {
        let mut info = self.info.write().await;
        info.status = NodeStatus::Initializing;
        drop(info);
        
        info!("Starting node: {}", self.info.read().await.name);
        
        // Start services
        self.services.start_all().await?;
        
        // Update status
        let mut info = self.info.write().await;
        info.status = NodeStatus::Running;
        info.last_heartbeat = chrono::Utc::now();
        
        info!("Node started successfully: {}", info.name);
        Ok(())
    }
    
    /// Stop the node
    pub async fn stop(&self) -> Result<()> {
        let mut info = self.info.write().await;
        info.status = NodeStatus::Shutdown;
        drop(info);
        
        info!("Stopping node: {}", self.info.read().await.name);
        
        // Stop services
        self.services.stop_all().await?;
        
        // Update status
        let mut info = self.info.write().await;
        info.status = NodeStatus::Shutdown;
        
        info!("Node stopped successfully: {}", info.name);
        Ok(())
    }
    
    /// Pause the node
    pub async fn pause(&self) -> Result<()> {
        let mut info = self.info.write().await;
        if info.status != NodeStatus::Running {
            return Err(crate::error::KernelError::Node(
                "Cannot pause non-running node".to_string()
            ));
        }
        
        info.status = NodeStatus::Paused;
        info.last_heartbeat = chrono::Utc::now();
        
        info!("Node paused: {}", info.name);
        Ok(())
    }
    
    /// Resume the node
    pub async fn resume(&self) -> Result<()> {
        let mut info = self.info.write().await;
        if info.status != NodeStatus::Paused {
            return Err(crate::error::KernelError::Node(
                "Cannot resume non-paused node".to_string()
            ));
        }
        
        info.status = NodeStatus::Running;
        info.last_heartbeat = chrono::Utc::now();
        
        info!("Node resumed: {}", info.name);
        Ok(())
    }
    
    /// Update heartbeat
    pub async fn heartbeat(&self) -> Result<()> {
        let mut info = self.info.write().await;
        if info.status == NodeStatus::Running {
            info.last_heartbeat = chrono::Utc::now();
        }
        Ok(())
    }
    
    /// Get node information
    pub async fn get_info(&self) -> NodeInfo {
        self.info.read().await.clone()
    }
    
    /// Get node handle
    pub async fn get_handle(&self) -> NodeHandle {
        let info = self.info.read().await;
        NodeHandle {
            id: info.id,
            name: info.name.clone(),
        }
    }
    
    /// Add capability
    pub async fn add_capability(&self, capability: String) -> Result<()> {
        let mut info = self.info.write().await;
        if !info.capabilities.contains(&capability) {
            info.capabilities.push(capability);
        }
        Ok(())
    }
    
    /// Update metadata
    pub async fn update_metadata(&self, metadata: serde_json::Value) -> Result<()> {
        let mut info = self.info.write().await;
        info.metadata = metadata;
        Ok(())
    }
    
    /// Get service registry
    pub fn services(&self) -> &ServiceRegistry {
        &self.services
    }
}