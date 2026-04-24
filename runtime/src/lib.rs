//! AstraOS Runtime
//! 
//! The runtime execution environment that manages node lifecycle,
//! resource allocation, and task execution.

pub mod actor;
pub mod executor;
pub mod resource;
pub mod scheduler;
pub mod monitor;

// Re-export common types from kernel for convenience
pub use astraos_kernel::{error::{Result, KernelError}, config::Config};

pub use actor::{Actor, ActorHandle, ActorMessage, ActorManager};
pub use executor::{Executor, ExecutorHandle, TaskHandler};
pub use resource::{ResourceManager, Resource, ResourceType};
pub use scheduler::{Task, TaskPriority, Scheduler, TaskResult, TaskStatus};
pub use monitor::{Monitor, HealthStatus, MonitorConfig, RestartRecoveryHandler};
