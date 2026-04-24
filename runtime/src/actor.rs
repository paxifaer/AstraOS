use astraos_kernel::error::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

/// Actor message type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorMessage {
    /// Execute a task
    ExecuteTask { task_id: Uuid, payload: serde_json::Value },
    /// Get actor status
    GetStatus,
    /// Shutdown actor
    Shutdown,
}

/// Actor trait
#[async_trait::async_trait]
pub trait Actor: Send + Sync {
    /// Actor type name
    fn name(&self) -> &'static str;
    
    /// Actor unique ID
    fn id(&self) -> Uuid;
    
    /// Initialize the actor
    async fn init(&mut self) -> Result<()>;
    
    /// Handle incoming messages
    async fn handle_message(&mut self, message: ActorMessage) -> Result<()>;
    
    /// Cleanup resources
    async fn cleanup(&mut self) -> Result<()>;
    
    /// Check if actor is healthy
    fn is_healthy(&self) -> bool;
}

/// Actor handle for sending messages
#[derive(Clone)]
pub struct ActorHandle {
    id: Uuid,
    name: String,
    sender: mpsc::UnboundedSender<ActorMessage>,
}

impl ActorHandle {
    /// Create a new actor handle
    pub fn new(id: Uuid, name: String, sender: mpsc::UnboundedSender<ActorMessage>) -> Self {
        Self { id, name, sender }
    }
    
    /// Send a message to the actor
    pub async fn send_message(&self, message: ActorMessage) -> Result<()> {
        self.sender.send(message)
            .map_err(|_| astraos_kernel::error::KernelError::Service("Actor disconnected".to_string()))?;
        Ok(())
    }
    
    /// Get actor ID
    pub fn id(&self) -> Uuid {
        self.id
    }
    
    /// Get actor name
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Actor manager
pub struct ActorManager {
    actors: Arc<RwLock<std::collections::HashMap<Uuid, ActorHandle>>>,
}

impl ActorManager {
    /// Create a new actor manager
    pub fn new() -> Self {
        Self {
            actors: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    /// Register an actor
    pub async fn register_actor(&self, handle: ActorHandle) -> Result<()> {
        let mut actors = self.actors.write().await;
        actors.insert(handle.id, handle);
        Ok(())
    }
    
    /// Unregister an actor
    pub async fn unregister_actor(&self, actor_id: Uuid) -> Result<()> {
        let mut actors = self.actors.write().await;
        actors.remove(&actor_id);
        Ok(())
    }
    
    /// Get actor by ID
    pub async fn get_actor(&self, actor_id: Uuid) -> Option<ActorHandle> {
        let actors = self.actors.read().await;
        actors.get(&actor_id).cloned()
    }
    
    /// Get all actors
    pub async fn get_all_actors(&self) -> Vec<ActorHandle> {
        let actors = self.actors.read().await;
        actors.values().cloned().collect()
    }
    
    /// Get actor count
    pub async fn get_actor_count(&self) -> usize {
        let actors = self.actors.read().await;
        actors.len()
    }
}