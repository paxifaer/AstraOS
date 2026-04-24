//! AstraOS Kernel
//! 
//! The core kernel module that provides the foundation for the robot operating system.
//! This module handles system initialization, lifecycle management, and core services.

pub mod error;
pub mod config;
pub mod lifecycle;
pub mod services;
pub mod node;
pub mod graph;

pub use error::{KernelError, Result};
pub use config::Config;
pub use lifecycle::{LifecycleManager, State};
pub use services::ServiceRegistry;
pub use node::{Node, NodeHandle, NodeInfo};
pub use graph::SystemGraph;

/// Kernel entry point
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Load configuration
    let config = Config::default();
    
    // Initialize lifecycle manager
    let mut lifecycle = LifecycleManager::new(config.clone());
    
    // Start the kernel
    lifecycle.start().await?;
    
    // Main loop
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        
        // Check system health
        if let Err(e) = lifecycle.check_health().await {
            tracing::error!("System health check failed: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kernel_initialization() {
        let config = Config::default();
        let mut lifecycle = LifecycleManager::new(config);
        assert!(lifecycle.start().await.is_ok());
    }
}