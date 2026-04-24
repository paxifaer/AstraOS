use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock as TokioRwLock;
use tracing::{debug, info};

use crate::{CommsError, CommsResult};

pub struct DiscoveryManager {
    services: Arc<TokioRwLock<HashMap<String, ServiceInfo>>>,
    nodes: Arc<TokioRwLock<HashMap<String, NodeInfo>>>,
    topics: Arc<TokioRwLock<HashMap<String, TopicInfo>>>,
    heartbeat_interval: Duration,
}

impl DiscoveryManager {
    pub fn new() -> Self {
        Self {
            services: Arc::new(TokioRwLock::new(HashMap::new())),
            nodes: Arc::new(TokioRwLock::new(HashMap::new())),
            topics: Arc::new(TokioRwLock::new(HashMap::new())),
            heartbeat_interval: Duration::from_secs(5),
        }
    }

    pub async fn register_service(&self, service_info: ServiceInfo) -> CommsResult<()> {
        let mut services = self.services.write().await;
        services.insert(service_info.name.clone(), service_info.clone());
        info!("Registered service: {}", service_info.name);
        Ok(())
    }

    pub async fn unregister_service(&self, service_name: &str) -> CommsResult<()> {
        let mut services = self.services.write().await;
        if services.remove(service_name).is_some() {
            info!("Unregistered service: {}", service_name);
            Ok(())
        } else {
            Err(CommsError::ServiceNotFound(service_name.to_string()))
        }
    }

    pub async fn find_service(&self, service_name: &str) -> Option<ServiceInfo> {
        let services = self.services.read().await;
        services.get(service_name).cloned()
    }

    pub async fn list_services(&self) -> Vec<ServiceInfo> {
        let services = self.services.read().await;
        services.values().cloned().collect()
    }

    pub async fn register_node(&self, node_info: NodeInfo) -> CommsResult<()> {
        let mut nodes = self.nodes.write().await;
        nodes.insert(node_info.name.clone(), node_info.clone());
        info!("Registered node: {}", node_info.name);
        Ok(())
    }

    pub async fn unregister_node(&self, node_name: &str) -> CommsResult<()> {
        let mut nodes = self.nodes.write().await;
        if nodes.remove(node_name).is_some() {
            info!("Unregistered node: {}", node_name);
            Ok(())
        } else {
            Err(CommsError::ConnectionError(format!("Node not found: {}", node_name)))
        }
    }

    pub async fn find_node(&self, node_name: &str) -> Option<NodeInfo> {
        let nodes = self.nodes.read().await;
        nodes.get(node_name).cloned()
    }

    pub async fn list_nodes(&self) -> Vec<NodeInfo> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }

    pub async fn register_topic(&self, topic_info: TopicInfo) -> CommsResult<()> {
        let mut topics = self.topics.write().await;
        topics.insert(topic_info.name.clone(), topic_info.clone());
        info!("Registered topic: {}", topic_info.name);
        Ok(())
    }

    pub async fn unregister_topic(&self, topic_name: &str) -> CommsResult<()> {
        let mut topics = self.topics.write().await;
        if topics.remove(topic_name).is_some() {
            info!("Unregistered topic: {}", topic_name);
            Ok(())
        } else {
            Err(CommsError::TopicNotFound(topic_name.to_string()))
        }
    }

    pub async fn find_topic(&self, topic_name: &str) -> Option<TopicInfo> {
        let topics = self.topics.read().await;
        topics.get(topic_name).cloned()
    }

    pub async fn list_topics(&self) -> Vec<TopicInfo> {
        let topics = self.topics.read().await;
        topics.values().cloned().collect()
    }

    pub async fn heartbeat(&self, node_name: &str) -> CommsResult<()> {
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(node_name) {
            node.last_heartbeat = SystemTime::now();
        }
        Ok(())
    }

    pub async fn cleanup_stale(&self, timeout: Duration) -> CommsResult<()> {
        let now = SystemTime::now();
        
        let mut nodes = self.nodes.write().await;
        nodes.retain(|_name, node| {
            if let Ok(duration) = now.duration_since(node.last_heartbeat) {
                duration < timeout
            } else {
                true
            }
        });
        
        debug!("Cleaned up stale nodes. Remaining nodes: {}", nodes.len());
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub node: String,
    pub type_: String,
    pub address: String,
    pub port: u16,
    pub metadata: HashMap<String, String>,
    pub registered_at: SystemTime,
    pub last_seen: SystemTime,
}

impl ServiceInfo {
    pub fn new(name: String, node: String, type_: String, address: String, port: u16) -> Self {
        Self {
            name,
            node,
            type_,
            address,
            port,
            metadata: HashMap::new(),
            registered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub name: String,
    pub address: String,
    pub port: u16,
    pub capabilities: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub registered_at: SystemTime,
    pub last_heartbeat: SystemTime,
}

impl NodeInfo {
    pub fn new(name: String, address: String, port: u16) -> Self {
        Self {
            name,
            address,
            port,
            capabilities: Vec::new(),
            metadata: HashMap::new(),
            registered_at: SystemTime::now(),
            last_heartbeat: SystemTime::now(),
        }
    }

    pub fn with_capability(mut self, capability: String) -> Self {
        self.capabilities.push(capability);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

#[derive(Debug, Clone)]
pub struct TopicInfo {
    pub name: String,
    pub node: String,
    pub type_: String,
    pub subscribers: Vec<String>,
    pub publishers: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub registered_at: SystemTime,
}

impl TopicInfo {
    pub fn new(name: String, node: String, type_: String) -> Self {
        Self {
            name,
            node,
            type_,
            subscribers: Vec::new(),
            publishers: Vec::new(),
            metadata: HashMap::new(),
            registered_at: SystemTime::now(),
        }
    }

    pub fn with_subscriber(mut self, subscriber: String) -> Self {
        self.subscribers.push(subscriber);
        self
    }

    pub fn with_publisher(mut self, publisher: String) -> Self {
        self.publishers.push(publisher);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}
