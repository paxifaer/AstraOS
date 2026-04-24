use crate::{error::Result, config::Config};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Service trait for all system services
#[async_trait::async_trait]
pub trait Service: Send + Sync {
    /// Start the service
    async fn start(&mut self) -> Result<()>;
    
    /// Stop the service
    async fn stop(&mut self) -> Result<()>;
    
    /// Get service name
    fn name(&self) -> &str;
    
    /// Get service status
    fn status(&self) -> ServiceStatus;
}

/// Service status
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}

/// Service registry for managing all system services
pub struct ServiceRegistry {
    services: RwLock<HashMap<String, Arc<RwLock<Box<dyn Service>>>>>,
    config: Arc<Config>,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new(config: Config) -> Self {
        Self {
            services: RwLock::new(HashMap::new()),
            config: Arc::new(config),
        }
    }
    
    /// Register a service
    pub async fn register_service(&self, name: String, service: Box<dyn Service>) -> Result<()> {
        let mut services = self.services.write().await;
        
        if services.contains_key(&name) {
            return Err(crate::error::KernelError::Service(
                format!("Service {} already registered", name)
            ));
        }
        
        services.insert(name.clone(), Arc::new(RwLock::new(service)));
        info!("Service registered: {}", name);
        
        Ok(())
    }
    
    /// Start a service
    pub async fn start_service(&self, name: &str) -> Result<()> {
        let services = self.services.read().await;
        
        let service = services.get(name)
            .ok_or_else(|| crate::error::KernelError::Service(
                format!("Service {} not found", name)
            ))?;
        
        let mut service = service.write().await;
        service.start().await
    }
    
    /// Stop a service
    pub async fn stop_service(&self, name: &str) -> Result<()> {
        let services = self.services.read().await;
        
        let service = services.get(name)
            .ok_or_else(|| crate::error::KernelError::Service(
                format!("Service {} not found", name)
            ))?;
        
        let mut service = service.write().await;
        service.stop().await
    }
    
    /// Start all registered services
    pub async fn start_all(&self) -> Result<()> {
        let services = self.services.read().await;
        
        for (name, service) in services.iter() {
            let mut service = service.write().await;
            info!("Starting service: {}", name);
            if let Err(e) = service.start().await {
                info!("Failed to start service {}: {}", name, e);
            }
        }
        
        Ok(())
    }
    
    /// Stop all registered services
    pub async fn stop_all(&self) -> Result<()> {
        let services = self.services.read().await;
        
        for (name, service) in services.iter() {
            let mut service = service.write().await;
            info!("Stopping service: {}", name);
            if let Err(e) = service.stop().await {
                info!("Failed to stop service {}: {}", name, e);
            }
        }
        
        Ok(())
    }
    
    /// Get service status
    pub async fn get_service_status(&self, name: &str) -> Result<ServiceStatus> {
        let services = self.services.read().await;
        
        let service = services.get(name)
            .ok_or_else(|| crate::error::KernelError::Service(
                format!("Service {} not found", name)
            ))?;
        
        let service = service.read().await;
        Ok(service.status())
    }
    
    /// List all services
    pub async fn list_services(&self) -> Vec<String> {
        let services = self.services.read().await;
        services.keys().cloned().collect()
    }
    
    /// Get service count
    pub async fn service_count(&self) -> usize {
        let services = self.services.read().await;
        services.len()
    }
}