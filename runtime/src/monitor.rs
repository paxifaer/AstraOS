use astraos_kernel::error::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use serde::{Serialize, Deserialize, Deserializer};
use tracing::{info, warn, error};
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use serde_json::Value;

/// Health status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Health check result
#[derive(Debug, Clone, Serialize)]
pub struct HealthCheck {
    /// Check ID
    pub id: Uuid,

    /// Check name
    pub name: String,

    /// Status
    pub status: HealthStatus,

    /// Message
    pub message: Option<String>,

    /// Timestamp (not serialized)
    #[serde(skip)]
    pub timestamp: Instant,

    /// Duration
    pub duration: Duration,

    /// Serialized timestamp for serde
    #[serde(rename = "timestamp_utc")]
    pub timestamp_utc: DateTime<Utc>,

    /// Additional metadata
    pub metadata: Option<Value>,
}

/// Helper struct for deserializing HealthCheck without the Instant field
#[derive(Deserialize)]
struct HealthCheckHelper {
    id: Uuid,
    name: String,
    status: HealthStatus,
    message: Option<String>,
    duration: Duration,
    timestamp_utc: Option<DateTime<Utc>>,
    metadata: Option<Value>,
}

impl<'de> Deserialize<'de> for HealthCheck {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = HealthCheckHelper::deserialize(deserializer)?;
        Ok(Self {
            id: helper.id,
            name: helper.name,
            status: helper.status,
            message: helper.message,
            timestamp: Instant::now(),
            duration: helper.duration,
            timestamp_utc: helper.timestamp_utc.unwrap_or_else(Utc::now),
            metadata: helper.metadata,
        })
    }
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            status: HealthStatus::Unknown,
            message: None,
            timestamp: Instant::now(),
            duration: Duration::from_secs(0),
            timestamp_utc: Utc::now(),
            metadata: None,
        }
    }
}

impl HealthCheck {
    /// Create a new health check
    pub fn new(name: String, status: HealthStatus, duration: Duration) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            status,
            message: None,
            timestamp: Instant::now(),
            duration,
            timestamp_utc: Utc::now(),
            metadata: None,
        }
    }

    /// Set message
    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Monitor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Check interval
    pub check_interval: Duration,

    /// Maximum restart count
    pub max_restart_count: usize,

    /// Minimum restart interval
    pub min_restart_interval: Duration,

    /// Recovery timeout
    pub recovery_timeout: Duration,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            max_restart_count: 3,
            min_restart_interval: Duration::from_secs(60),
            recovery_timeout: Duration::from_secs(300),
        }
    }
}

/// Health monitor
pub struct Monitor {
    config: MonitorConfig,
    health_checks: Arc<RwLock<std::collections::HashMap<String, Box<dyn HealthCheckFn + Send + Sync>>>>,
    health_results: Arc<RwLock<std::collections::HashMap<String, Vec<HealthCheck>>>>,
    restart_counts: Arc<RwLock<std::collections::HashMap<String, usize>>>,
    last_restart_times: Arc<RwLock<std::collections::HashMap<String, Instant>>>,
    recovery_handlers: Arc<RwLock<std::collections::HashMap<String, Box<dyn RecoveryHandler + Send + Sync>>>>,
    simple_health_checks: Arc<RwLock<std::collections::HashMap<String, HealthStatus>>>,
}

impl Clone for Monitor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            health_checks: Arc::clone(&self.health_checks),
            health_results: Arc::clone(&self.health_results),
            restart_counts: Arc::clone(&self.restart_counts),
            last_restart_times: Arc::clone(&self.last_restart_times),
            recovery_handlers: Arc::clone(&self.recovery_handlers),
            simple_health_checks: Arc::clone(&self.simple_health_checks),
        }
    }
}

/// Health check function type
#[async_trait::async_trait]
pub trait HealthCheckFn: Send + Sync {
    /// Check name
    fn name(&self) -> &'static str;

    /// Perform health check
    async fn check(&self) -> Result<HealthCheck>;
}

/// Recovery handler trait
#[async_trait::async_trait]
pub trait RecoveryHandler: Send + Sync {
    /// Handler name
    fn name(&self) -> &'static str;

    /// Handle recovery
    async fn handle(&self, check_name: &str, health_check: &HealthCheck) -> Result<()>;

    /// Check if handler can handle this check
    fn can_handle(&self, check_name: &str) -> bool;
}

/// Helper trait for cloning boxed recovery handlers
#[allow(dead_code)]
trait RecoveryHandlerClone: RecoveryHandler + Send + Sync {
    fn clone_box(&self) -> Box<dyn RecoveryHandler + Send + Sync>;
}

impl<T: RecoveryHandler + Clone + Send + Sync + 'static> RecoveryHandlerClone for T {
    fn clone_box(&self) -> Box<dyn RecoveryHandler + Send + Sync> {
        Box::new(self.clone())
    }
}

impl Monitor {
    /// Create a new monitor
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            config,
            health_checks: Arc::new(RwLock::new(std::collections::HashMap::new())),
            health_results: Arc::new(RwLock::new(std::collections::HashMap::new())),
            restart_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            last_restart_times: Arc::new(RwLock::new(std::collections::HashMap::new())),
            recovery_handlers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            simple_health_checks: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Register a simple health check with a name and status
    pub async fn register_health_check(&self, name: String, status: HealthStatus) -> Result<()> {
        let mut checks = self.simple_health_checks.write().await;
        checks.insert(name.clone(), status);
        info!("Simple health check registered: {}", name);
        Ok(())
    }

    /// Register a health check function
    pub async fn register_health_check_fn(&self, check: Box<dyn HealthCheckFn + Send + Sync>) -> Result<()> {
        let mut checks = self.health_checks.write().await;
        checks.insert(check.name().to_string(), check);
        info!("Health check function registered: {}", checks.len());
        Ok(())
    }

    /// Register a recovery handler
    pub async fn register_recovery_handler(&self, handler: Box<dyn RecoveryHandler + Send + Sync>) -> Result<()> {
        let mut handlers = self.recovery_handlers.write().await;
        let handler_name = handler.name().to_string();
        handlers.insert(handler_name.clone(), handler);
        info!("Recovery handler registered: {}", handler_name);
        Ok(())
    }

    /// Update health check status
    pub async fn update_health_check(&self, name: &str, status: HealthStatus, message: Option<String>, metadata: Option<Value>) -> Result<()> {
        let mut checks = self.simple_health_checks.write().await;
        checks.insert(name.to_string(), status.clone());

        // Create a health check result
        let health_check = HealthCheck {
            id: Uuid::new_v4(),
            name: name.to_string(),
            status: status.clone(),
            message,
            timestamp: Instant::now(),
            duration: Duration::from_secs(0),
            timestamp_utc: Utc::now(),
            metadata,
        };

        // Store in results
        let mut health_results = self.health_results.write().await;
        health_results
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(health_check);

        info!("Health check updated: {} -> {:?}", name, status);
        Ok(())
    }

    /// Run all health checks
    pub async fn run_health_checks(&self) -> Vec<HealthCheck> {
        let checks = self.health_checks.read().await;
        let mut results = Vec::new();

        for check in checks.values() {
            match check.check().await {
                Ok(health_check) => {
                    results.push(health_check);
                }
                Err(e) => {
                    error!("Health check failed for {}: {}", check.name(), e);
                    results.push(HealthCheck::new(
                        check.name().to_string(),
                        HealthStatus::Critical,
                        Duration::from_secs(0),
                    ).with_message(format!("Health check failed: {}", e)));
                }
            }
        }

        // Store results
        let mut health_results = self.health_results.write().await;
        for result in &results {
            health_results
                .entry(result.name.clone())
                .or_insert_with(Vec::new)
                .push(result.clone());
        }

        results
    }

    /// Handle recovery for a component
    pub async fn handle_recovery(&self, component_name: &str, health_check: &HealthCheck) -> Result<()> {
        let handlers = self.recovery_handlers.read().await;

        // Find a handler that can handle this component
        if let Some(handler) = handlers.values().find(|h| h.can_handle(component_name)) {
            handler.handle(component_name, health_check).await?;
            info!("Recovery handled for component: {}", component_name);
        } else {
            warn!("No recovery handler found for component: {}", component_name);
        }

        Ok(())
    }

    /// Monitor and auto-recover components
    pub async fn monitor_and_recover(&self) -> Result<()> {
        let health_checks = self.run_health_checks().await;

        for health_check in &health_checks {
            if health_check.status != HealthStatus::Healthy {
                // Check restart count
                let mut restart_counts = self.restart_counts.write().await;
                let count = restart_counts.entry(health_check.name.clone()).or_insert(0);

                // Check restart interval
                let mut last_restart_times = self.last_restart_times.write().await;
                let last_restart = last_restart_times.get(&health_check.name).cloned().unwrap_or_else(|| Instant::now() - Duration::from_secs(3600));

                if *count < self.config.max_restart_count
                    && health_check.timestamp.duration_since(last_restart) > self.config.min_restart_interval {

                    // Try to recover
                    if let Err(e) = self.handle_recovery(&health_check.name, health_check).await {
                        error!("Recovery failed for {}: {}", health_check.name, e);
                        *count += 1;
                        last_restart_times.insert(health_check.name.clone(), Instant::now());
                    }
                } else {
                    error!("Component {} exceeded restart limit or interval", health_check.name);
                }
            }
        }

        Ok(())
    }

    /// Start monitoring
    pub async fn start(&self) -> Result<()> {
        info!("Monitor started");

        let mut interval = tokio::time::interval(self.config.check_interval);

        loop {
            interval.tick().await;

            if let Err(e) = self.monitor_and_recover().await {
                error!("Monitor error: {}", e);
            }
        }
    }

    /// Start monitoring (non-blocking version that returns after starting)
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("Monitoring started");

        // Run initial health checks
        let results = self.run_health_checks().await;
        for result in &results {
            info!("Health check result: {} -> {:?}", result.name, result.status);
        }

        Ok(())
    }

    /// Get health check results
    pub async fn get_health_results(&self, check_name: &str) -> Option<Vec<HealthCheck>> {
        let health_results = self.health_results.read().await;
        health_results.get(check_name).cloned()
    }

    /// Get all health checks
    pub async fn get_all_health_checks(&self) -> Vec<String> {
        let checks = self.health_checks.read().await;
        checks.keys().cloned().collect()
    }

    /// Get restart counts
    pub async fn get_restart_counts(&self) -> std::collections::HashMap<String, usize> {
        let restart_counts = self.restart_counts.read().await;
        restart_counts.clone()
    }
}

/// Simple recovery handler that restarts components
#[derive(Clone)]
pub struct RestartRecoveryHandler {
    #[allow(dead_code)]
    max_attempts: usize,
}

impl RestartRecoveryHandler {
    /// Create a new restart recovery handler
    pub fn new(max_attempts: usize) -> Self {
        Self { max_attempts }
    }
}

#[async_trait::async_trait]
impl RecoveryHandler for RestartRecoveryHandler {
    fn name(&self) -> &'static str {
        "restart"
    }

    async fn handle(&self, check_name: &str, _health_check: &HealthCheck) -> Result<()> {
        info!("Attempting to restart component: {}", check_name);
        // In a real implementation, this would restart the component
        Ok(())
    }

    fn can_handle(&self, _check_name: &str) -> bool {
        // This handler can handle all components
        true
    }
}
