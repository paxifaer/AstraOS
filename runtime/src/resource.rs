use astraos_kernel::error::KernelError;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::info;
use serde::{Serialize, Deserialize};
use chrono;
use serde_json::Value;

/// Resource allocation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceStatus {
    Free,
    Allocated { owner: Uuid, allocated_at: chrono::DateTime<chrono::Utc> },
    Released,
}

/// Resource type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceType {
    CPU,
    Memory,
    Disk,
    Network,
    GPU,
    Custom(String),
}

/// Resource representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Resource ID
    pub id: Uuid,
    
    /// Resource name
    pub name: String,
    
    /// Resource type
    pub resource_type: ResourceType,
    
    /// Resource capacity
    pub capacity: f64,
    
    /// Resource unit
    pub unit: String,
    
    /// Resource usage
    pub usage: f64,
    
    /// Resource status
    pub status: ResourceStatus,
    
    /// Resource owner
    pub owner: Option<Uuid>,
    
    /// Allocation timestamp
    pub allocated_at: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Release timestamp
    pub released_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Resource {
    /// Create a new resource
    pub fn new(resource_type: ResourceType, capacity: f64, unit: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: format!("{:?}", resource_type).to_lowercase(),
            resource_type,
            capacity,
            unit: unit.to_string(),
            usage: 0.0,
            status: ResourceStatus::Free,
            owner: None,
            allocated_at: None,
            released_at: None,
        }
    }
    
    /// Allocate resource to owner
    pub fn allocate(&mut self, owner_id: Uuid) -> Result<(), KernelError> {
        if self.status != ResourceStatus::Free {
            return Err(KernelError::Service("Resource already allocated".to_string()));
        }
        
        self.status = ResourceStatus::Allocated { 
            owner: owner_id, 
            allocated_at: chrono::Utc::now() 
        };
        self.owner = Some(owner_id);
        self.allocated_at = Some(chrono::Utc::now());
        Ok(())
    }
    
    /// Release resource
    pub fn release(&mut self) {
        self.status = ResourceStatus::Released;
        self.released_at = Some(chrono::Utc::now());
        self.owner = None;
    }
    
    /// Check if resource is free
    pub fn is_free(&self) -> bool {
        matches!(self.status, ResourceStatus::Free)
    }
    
    /// Get utilization percentage
    pub fn utilization(&self) -> f64 {
        if self.capacity > 0.0 {
            (self.usage / self.capacity) * 100.0
        } else {
            0.0
        }
    }
}

/// Resource manager
pub struct ResourceManager {
    resources: Arc<RwLock<std::collections::HashMap<Uuid, Resource>>>,
    allocations: Arc<RwLock<std::collections::HashMap<Uuid, Vec<Uuid>>>>,
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(std::collections::HashMap::new())),
            allocations: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    /// Add a resource
    pub async fn add_resource(&self, resource: Resource) -> Result<(), KernelError> {
        let mut resources = self.resources.write().await;
        resources.insert(resource.id, resource.clone());
        info!("Resource added: {}", resources.len());
        Ok(())
    }
    
    /// Get resource by ID
    pub async fn get_resource(&self, resource_id: Uuid) -> Option<Resource> {
        let resources = self.resources.read().await;
        resources.get(&resource_id).cloned()
    }
    
    /// Allocate resources to owner
    pub async fn allocate_resources(&self, owner_id: Uuid, resource_ids: Vec<Uuid>) -> Result<Vec<Resource>, KernelError> {
        let mut resources = self.resources.write().await;
        let mut allocations = self.allocations.write().await;
        
        let mut allocated_resources = Vec::new();
        
        for resource_id in &resource_ids {
            if let Some(resource) = resources.get_mut(resource_id) {
                if resource.is_free() {
                    resource.allocate(owner_id)?;
                    allocated_resources.push(resource.clone());
                }
            }
        }
        
        // Update allocation map
        allocations.entry(owner_id).or_insert_with(Vec::new).extend(resource_ids.clone());
        
        info!("Resources allocated to {}: {:?}", owner_id, resource_ids);
        Ok(allocated_resources)
    }
    
    /// Release resources from owner
    pub async fn release_resources(&self, owner_id: Uuid) -> Result<Vec<Resource>, KernelError> {
        let mut resources = self.resources.write().await;
        let mut allocations = self.allocations.write().await;
        
        let released_resources = if let Some(resource_ids) = allocations.remove(&owner_id) {
            let mut released = Vec::new();
            
            for resource_id in &resource_ids {
                if let Some(resource) = resources.get_mut(resource_id) {
                    resource.release();
                    released.push(resource.clone());
                }
            }
            
            info!("Resources released from {}: {:?}", owner_id, resource_ids);
            released
        } else {
            Vec::new()
        };
        
        Ok(released_resources)
    }
    
    /// Get owner's resources
    pub async fn get_owner_resources(&self, owner_id: Uuid) -> Vec<Resource> {
        let resources = self.resources.read().await;
        let allocations = self.allocations.read().await;
        
        if let Some(resource_ids) = allocations.get(&owner_id) {
            resource_ids
                .iter()
                .filter_map(|id| resources.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get free resources
    pub async fn get_free_resources(&self) -> Vec<Resource> {
        let resources = self.resources.read().await;
        resources
            .values()
            .filter(|r| r.is_free())
            .cloned()
            .collect()
    }
    
    /// Get all resources
    pub async fn get_all_resources(&self) -> Vec<Resource> {
        let resources = self.resources.read().await;
        resources.values().cloned().collect()
    }
    
    /// Get resource count
    pub async fn get_resource_count(&self) -> usize {
        let resources = self.resources.read().await;
        resources.len()
    }
    
    /// Get allocated resource count
    pub async fn get_allocated_count(&self) -> usize {
        let resources = self.resources.read().await;
        resources.values().filter(|r| !r.is_free()).count()
    }
    
    /// Get resource usage as JSON
    pub async fn get_resource_usage(&self) -> Value {
        let resources = self.resources.read().await;
        
        let mut usage_map = serde_json::Map::new();
        let mut total_capacity = 0.0;
        let mut total_usage = 0.0;
        
        for resource in resources.values() {
            let resource_key = format!("{:?}", resource.resource_type).to_lowercase();
            usage_map.insert(
                resource_key.clone(),
                serde_json::json!({
                    "capacity": resource.capacity,
                    "usage": resource.usage,
                    "unit": resource.unit,
                    "utilization": resource.utilization(),
                    "status": format!("{:?}", resource.status)
                })
            );
            
            total_capacity += resource.capacity;
            total_usage += resource.usage;
        }
        
        serde_json::json!({
            "resources": usage_map,
            "summary": {
                "total_capacity": total_capacity,
                "total_usage": total_usage,
                "total_utilization": if total_capacity > 0.0 { (total_usage / total_capacity) * 100.0 } else { 0.0 },
                "resource_count": resources.len()
            }
        })
    }
}
