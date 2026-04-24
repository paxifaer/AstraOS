use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::info;
use uuid::Uuid;

use crate::{CommsError, CommsResult};

pub struct ServiceManager {
    services: DashMap<String, Service>,
}

impl ServiceManager {
    pub fn new() -> Self {
        Self {
            services: DashMap::new(),
        }
    }

    pub async fn register_service(&self, service: Service) -> CommsResult<()> {
        let service_name = service.name.clone();
        self.services.insert(service_name, service.clone());
        info!("Registered service: {}", service.name);
        Ok(())
    }

    pub async fn unregister_service(&self, service_name: &str) -> CommsResult<()> {
        if self.services.remove(service_name).is_some() {
            info!("Unregistered service: {}", service_name);
            Ok(())
        } else {
            Err(CommsError::ServiceNotFound(service_name.to_string()))
        }
    }

    pub async fn call_service(&self, service_name: &str, request: Request) -> CommsResult<Response> {
        if let Some(service) = self.services.get(service_name) {
            service.handle_request(request).await
        } else {
            Err(CommsError::ServiceNotFound(service_name.to_string()))
        }
    }

    pub fn list_services(&self) -> Vec<String> {
        self.services.iter().map(|s| s.key().clone()).collect()
    }

    pub fn get_service_info(&self, service_name: &str) -> Option<ServiceInfo> {
        self.services.get(service_name).map(|s| ServiceInfo {
            name: s.name.clone(),
            request_count: s.request_count.load(std::sync::atomic::Ordering::Relaxed),
        })
    }
}

#[derive(Clone)]
pub struct Service {
    pub name: String,
    pub handler: Arc<dyn ServiceHandler>,
    pub request_count: Arc<std::sync::atomic::AtomicU64>,
}

impl Service {
    pub fn new<T>(name: String, handler: T) -> Self
    where
        T: ServiceHandler + Send + Sync + 'static,
    {
        Self {
            name,
            handler: Arc::new(handler),
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub async fn handle_request(&self, request: Request) -> CommsResult<Response> {
        self.request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.handler.handle(request).await
    }
}

#[async_trait]
pub trait ServiceHandler: Send + Sync {
    async fn handle(&self, request: Request) -> CommsResult<Response>;
}

pub struct ServiceBuilder {
    service_name: String,
}

impl ServiceBuilder {
    pub fn new(service_name: String) -> Self {
        Self {
            service_name,
        }
    }

    pub async fn register<T>(self, handler: T) -> CommsResult<()>
    where
        T: ServiceHandler + Send + Sync + 'static,
    {
        let service = Service::new(self.service_name.clone(), handler);
        
        let service_manager = ServiceManager::new();
        service_manager.register_service(service).await
    }
}

#[derive(Debug, Clone)]
pub struct Request {
    pub id: Uuid,
    pub service: String,
    pub data: serde_json::Value,
    pub timeout: Option<std::time::Duration>,
}

impl Request {
    pub fn new(service: String, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            service,
            data,
            timeout: None,
        }
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn get_param<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.data.get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub id: Uuid,
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

impl Response {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            success: true,
            data,
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            success: false,
            data: serde_json::Value::Null,
            error: Some(error),
        }
    }

    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }
}

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub request_count: u64,
}

pub struct ServiceClient {
    service_manager: Arc<ServiceManager>,
}

impl ServiceClient {
    pub fn new() -> Self {
        Self {
            service_manager: Arc::new(ServiceManager::new()),
        }
    }

    pub async fn call(&self, service_name: &str, request: Request) -> CommsResult<Response> {
        self.service_manager.call_service(service_name, request).await
    }

    pub async fn call_with_timeout(
        &self,
        service_name: &str,
        request: Request,
        timeout: std::time::Duration,
    ) -> CommsResult<Response> {
        let request = request.with_timeout(timeout);
        tokio::time::timeout(timeout, self.call(service_name, request))
            .await
            .map_err(|_| CommsError::TimeoutError("Service call timeout".to_string()))?
    }
}

impl Default for ServiceClient {
    fn default() -> Self {
        Self::new()
    }
}
