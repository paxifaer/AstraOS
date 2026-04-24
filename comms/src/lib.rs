pub mod message;
pub mod topic;
pub mod service;
pub mod action;
pub mod transport;
pub mod discovery;

pub use message::*;
pub use topic::*;
pub use service::{Service, ServiceManager, ServiceHandler, ServiceBuilder, Request, Response, ServiceClient};
pub use action::*;
pub use transport::*;
pub use discovery::{DiscoveryManager, ServiceInfo as DiscoveryServiceInfo, NodeInfo, TopicInfo};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommsError {
    #[error("Topic not found: {0}")]
    TopicNotFound(String),
    
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    
    #[error("Transport error: {0}")]
    TransportError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Timeout error: {0}")]
    TimeoutError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type CommsResult<T> = Result<T, CommsError>;

impl From<CommsError> for std::io::Error {
    fn from(err: CommsError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, err.to_string())
    }
}
