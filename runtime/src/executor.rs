use astraos_kernel::error::Result;
use crate::scheduler::TaskResult;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::info;
use chrono::Utc;

/// Executor handle for sending tasks
#[derive(Clone)]
pub struct ExecutorHandle {
    sender: tokio::sync::mpsc::UnboundedSender<TaskCommand>,
}

impl ExecutorHandle {
    /// Create a new executor handle
    pub fn new(sender: tokio::sync::mpsc::UnboundedSender<TaskCommand>) -> Self {
        Self { sender }
    }

    /// Send a task to execute
    pub async fn send_task(&self, task_id: Uuid, payload: serde_json::Value) -> Result<()> {
        self.sender.send(TaskCommand::Execute { task_id, payload })
            .map_err(|_| astraos_kernel::error::KernelError::Service("Executor disconnected".to_string()))?;
        Ok(())
    }

    /// Send a task result
    pub async fn send_result(&self, result: TaskResult) -> Result<()> {
        self.sender.send(TaskCommand::Result { result })
            .map_err(|_| astraos_kernel::error::KernelError::Service("Executor disconnected".to_string()))?;
        Ok(())
    }

    /// Shutdown executor
    pub async fn shutdown(&self) -> Result<()> {
        self.sender.send(TaskCommand::Shutdown)
            .map_err(|_| astraos_kernel::error::KernelError::Service("Executor disconnected".to_string()))?;
        Ok(())
    }
}

/// Executor commands
#[derive(Debug)]
pub enum TaskCommand {
    Execute { task_id: Uuid, payload: serde_json::Value },
    Result { result: TaskResult },
    Shutdown,
}

/// Task executor
pub struct Executor {
    command_receiver: tokio::sync::mpsc::UnboundedReceiver<TaskCommand>,
    task_handlers: Arc<RwLock<std::collections::HashMap<String, Arc<dyn TaskHandler + Send + Sync>>>>,
    running_tasks: Arc<RwLock<std::collections::HashSet<Uuid>>>,
}

/// Task handler trait
#[async_trait::async_trait]
pub trait TaskHandler: Send + Sync {
    /// Handler name
    fn name(&self) -> &'static str;

    /// Handle task execution
    async fn execute(&self, task_id: Uuid, payload: serde_json::Value) -> Result<serde_json::Value>;

    /// Check if handler can handle this task type
    fn can_handle(&self, task_type: &str) -> bool;
}

impl Executor {
    /// Create a new executor
    pub fn new() -> (Self, ExecutorHandle) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let handle = ExecutorHandle::new(sender);

        let executor = Self {
            command_receiver: receiver,
            task_handlers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            running_tasks: Arc::new(RwLock::new(std::collections::HashSet::new())),
        };

        (executor, handle)
    }

    /// Register a task handler
    pub async fn register_handler(&self, handler: Box<dyn TaskHandler + Send + Sync>) -> Result<()> {
        let mut handlers = self.task_handlers.write().await;
        handlers.insert(handler.name().to_string(), Arc::from(handler));
        info!("Task handler registered: {}", handlers.len());
        Ok(())
    }

    /// Unregister a task handler
    pub async fn unregister_handler(&self, handler_name: &str) -> Result<()> {
        let mut handlers = self.task_handlers.write().await;
        handlers.remove(handler_name);
        info!("Task handler unregistered: {}", handler_name);
        Ok(())
    }

    /// Run the executor
    pub async fn run(&mut self) -> Result<()> {
        info!("Executor started");

        while let Some(command) = self.command_receiver.recv().await {
            match command {
                TaskCommand::Execute { task_id, payload } => {
                    self.handle_task_execution(task_id, payload).await;
                }
                TaskCommand::Result { result } => {
                    self.handle_task_result(result).await;
                }
                TaskCommand::Shutdown => {
                    info!("Executor shutting down");
                    break;
                }
            }
        }

        info!("Executor stopped");
        Ok(())
    }

    /// Handle task execution
    async fn handle_task_execution(&self, task_id: Uuid, payload: serde_json::Value) {
        let task_type = payload.get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("unknown");

        let handlers = self.task_handlers.read().await;

        if let Some(handler) = handlers.values().find(|h| h.can_handle(task_type)) {
            // Clone the Arc (cheap operation)
            let handler_clone = Arc::clone(handler);
            drop(handlers);

            let mut running = self.running_tasks.write().await;
            running.insert(task_id);
            drop(running);

            // Execute task in background
            tokio::spawn(async move {
                let start_time = std::time::Instant::now();
                let result = match handler_clone.execute(task_id, payload).await {
                    Ok(result) => TaskResult {
                        task_id,
                        status: crate::scheduler::TaskStatus::Completed,
                        result: Some(result),
                        error: None,
                        execution_time: start_time.elapsed(),
                        completed_at: std::time::Instant::now(),
                        completed_at_utc: Utc::now(),
                    },
                    Err(e) => {
                        let error_msg: String = e.to_string();
                        TaskResult {
                            task_id,
                            status: crate::scheduler::TaskStatus::Failed,
                            result: None,
                            error: Some(error_msg),
                            execution_time: start_time.elapsed(),
                            completed_at: std::time::Instant::now(),
                            completed_at_utc: Utc::now(),
                        }
                    },
                };

                // Send result back
                // Note: In a real implementation, we'd need a way to send this back to the scheduler
                tracing::info!("Task {} completed with status: {:?}", task_id, result.status);
            });
        } else {
            tracing::warn!("No handler found for task type: {}", task_type);
        }
    }

    /// Handle task result
    async fn handle_task_result(&self, result: TaskResult) {
        let mut running = self.running_tasks.write().await;
        running.remove(&result.task_id);
        drop(running);

        tracing::info!("Task result received for task: {:?}", result.task_id);
    }
}
