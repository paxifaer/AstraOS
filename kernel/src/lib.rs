//! AstraOS Kernel
//!
//! The core kernel module for AstraOS robot operating system.

pub mod config;
pub mod error;
pub mod graph;
pub mod lifecycle;
pub mod node;
pub mod services;

pub use config::*;
pub use error::*;
pub use graph::*;
pub use lifecycle::*;
pub use node::*;
pub use services::*;

/// Kernel result type
pub type Result<T> = std::result::Result<T, KernelError>;

/// AstraOS version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the kernel with default configuration
pub fn init() -> Result<System> {
    let config = Config::default();
    System::new(config)
}

/// Get kernel information
pub fn info() -> KernelInfo {
    KernelInfo {
        version: VERSION,
        name: "AstraOS".to_string(),
        description: "Modern Robot Operating System".to_string(),
    }
}

/// Kernel information structure
#[derive(Debug, Clone, serde::Serialize)]
pub struct KernelInfo {
    pub version: &'static str,
    pub name: String,
    pub description: String,
}