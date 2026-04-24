use thiserror::Error;

/// Kernel-level errors
#[derive(Error, Debug)]
pub enum KernelError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Node error: {0}")]
    Node(String),
    
    #[error("Service error: {0}")]
    Service(String),
    
    #[error("Lifecycle error: {0}")]
    Lifecycle(String),
    
    #[error("System graph error: {0}")]
    Graph(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
}

/// Result type for kernel operations
pub type Result<T> = std::result::Result<T, KernelError>;