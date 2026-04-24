use astraos_kernel::error::Result;
use std::time::{Duration, Instant};
use uuid::Uuid;
use serde::{Serialize, Deserialize, Deserializer};
use tracing::info;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use priority_queue::PriorityQueue;
use chrono::{DateTime, Utc};
use crate::executor::ExecutorHandle;

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Critical = 0,   // Highest priority
    High = 1,       // High priority
    Normal = 2,     // Normal priority
    Low = 3,       // Background tasks
    Idle = 4,      // Lowest priority
}

/// Task definition
#[derive(Debug, Clone, Serialize)]
pub struct Task {
    /// Task ID
    pub id: Uuid,

    /// Task name
    pub name: String,

    /// Task priority
    pub priority: TaskPriority,

    /// Task payload
    pub payload: serde_json::Value,

    /// Task timeout
    pub timeout: Duration,

    /// Task creation time (not serialized)
    #[serde(skip)]
    pub created_at: Instant,

    /// Task dependencies
    pub dependencies: Vec<Uuid>,

    /// Task metadata
    pub metadata: serde_json::Value,

    /// Serialized timestamp for serde
    #[serde(rename = "created_at_utc")]
    pub created_at_utc: DateTime<Utc>,
}

/// Helper struct for deserializing Task without the Instant field
#[derive(Deserialize)]
struct TaskHelper {
    id: Uuid,
    name: String,
    priority: TaskPriority,
    payload: serde_json::Value,
    timeout: Duration,
    dependencies: Vec<Uuid>,
    metadata: serde_json::Value,
    created_at_utc: Option<DateTime<Utc>>,
}

impl<'de> Deserialize<'de> for Task {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = TaskHelper::deserialize(deserializer)?;
        Ok(Self {
            id: helper.id,
            name: helper.name,
            priority: helper.priority,
            payload: helper.payload,
            timeout: helper.timeout,
            created_at: Instant::now(),
            dependencies: helper.dependencies,
            metadata: helper.metadata,
            created_at_utc: helper.created_at_utc.unwrap_or_else(Utc::now),
        })
    }
}

impl Default for Task {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            priority: TaskPriority::Normal,
            payload: serde_json::json!(null),
            timeout: Duration::from_secs(60),
            created_at: Instant::now(),
            dependencies: Vec::new(),
            metadata: serde_json::json!({}),
            created_at_utc: Utc::now(),
        }
    }
}

impl Task {
    /// Create a new task
    pub fn new(name: String, priority: TaskPriority, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            priority,
            payload,
            timeout: Duration::from_secs(60),
            created_at: Instant::now(),
            dependencies: Vec::new(),
            metadata: serde_json::json!({}),
            created_at_utc: Utc::now(),
        }
    }

    /// Add a dependency
    pub fn with_dependency(mut self, dependency_id: Uuid) -> Self {
        self.dependencies.push(dependency_id);
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Check if task can be executed (dependencies met)
    pub fn can_execute(&self, completed_tasks: &[Uuid]) -> bool {
        self.dependencies.iter().all(|dep| completed_tasks.contains(dep))
    }

    /// Get task age
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

/// Task status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Timeout,
    Cancelled,
}

/// Task result
#[derive(Debug, Clone, Serialize)]
pub struct TaskResult {
    /// Task ID
    pub task_id: Uuid,

    /// Task status
    pub status: TaskStatus,

    /// Result payload
    pub result: Option<serde_json::Value>,

    /// Error message
    pub error: Option<String>,

    /// Execution time
    pub execution_time: Duration,

    /// Completion time (not serialized)
    #[serde(skip)]
    pub completed_at: Instant,

    /// Serialized timestamp for serde
    #[serde(rename = "completed_at_utc")]
    pub completed_at_utc: DateTime<Utc>,
}

/// Helper struct for deserializing TaskResult without the Instant field
#[derive(Deserialize)]
struct TaskResultHelper {
    task_id: Uuid,
    status: TaskStatus,
    result: Option<serde_json::Value>,
    error: Option<String>,
    execution_time: Duration,
    completed_at_utc: Option<DateTime<Utc>>,
}

impl<'de> Deserialize<'de> for TaskResult {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = TaskResultHelper::deserialize(deserializer)?;
        Ok(Self {
            task_id: helper.task_id,
            status: helper.status,
            result: helper.result,
            error: helper.error,
            execution_time: helper.execution_time,
            completed_at: Instant::now(),
            completed_at_utc: helper.completed_at_utc.unwrap_or_else(Utc::now),
        })
    }
}

impl Default for TaskResult {
    fn default() -> Self {
        Self {
            task_id: Uuid::new_v4(),
            status: TaskStatus::Pending,
            result: None,
            error: None,
            execution_time: Duration::from_secs(0),
            completed_at: Instant::now(),
            completed_at_utc: Utc::now(),
        }
    }
}

/// Task scheduler
pub struct Scheduler {
    #[allow(dead_code)]
    executor_handle: Option<ExecutorHandle>,
    tasks: Arc<RwLock<HashMap<Uuid, Task>>>,
    task_queue: Arc<RwLock<PriorityQueue<Uuid, TaskPriority>>>,
    completed_tasks: Arc<RwLock<HashSet<Uuid>>>,
    running_tasks: Arc<RwLock<HashSet<Uuid>>>,
    results: Arc<RwLock<HashMap<Uuid, TaskResult>>>,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        Self {
            executor_handle: None,
            tasks: Arc::new(RwLock::new(HashMap::new())),
            task_queue: Arc::new(RwLock::new(PriorityQueue::new())),
            completed_tasks: Arc::new(RwLock::new(HashSet::new())),
            running_tasks: Arc::new(RwLock::new(HashSet::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new scheduler with executor handle
    pub fn new_with_executor(executor_handle: ExecutorHandle) -> Self {
        Self {
            executor_handle: Some(executor_handle),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            task_queue: Arc::new(RwLock::new(PriorityQueue::new())),
            completed_tasks: Arc::new(RwLock::new(HashSet::new())),
            running_tasks: Arc::new(RwLock::new(HashSet::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Submit a task to the scheduler
    pub async fn submit_task(&self, task: Task) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        let mut queue = self.task_queue.write().await;

        tasks.insert(task.id, task.clone());
        queue.push(task.id, task.priority);

        info!("Task submitted to scheduler: {} ({})", task.name, task.id);
        Ok(())
    }

    /// Add a task to the scheduler
    pub async fn add_task(&self, task: Task) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        let mut queue = self.task_queue.write().await;

        tasks.insert(task.id, task.clone());
        queue.push(task.id, task.priority);

        info!("Task added to scheduler: {} ({})", task.name, task.id);
        Ok(())
    }

    /// Get next task to execute
    pub async fn get_next_task(&self) -> Option<Task> {
        let mut queue = self.task_queue.write().await;
        let tasks = self.tasks.read().await;
        let completed = self.completed_tasks.read().await;

        while let Some((task_id, _)) = queue.pop() {
            if !completed.contains(&task_id) {
                if let Some(task) = tasks.get(&task_id) {
                    return Some(task.clone());
                }
            }
        }

        None
    }

    /// Mark task as running
    pub async fn mark_task_running(&self, task_id: Uuid) -> Result<()> {
        let mut running = self.running_tasks.write().await;
        running.insert(task_id);
        Ok(())
    }

    /// Mark task as completed
    pub async fn mark_task_completed(&self, task_id: Uuid, result: TaskResult) -> Result<()> {
        let mut completed = self.completed_tasks.write().await;
        let mut running = self.running_tasks.write().await;
        let mut results = self.results.write().await;

        completed.insert(task_id);
        running.remove(&task_id);
        results.insert(task_id, result);

        info!("Task completed: {}", task_id);
        Ok(())
    }

    /// Get task result
    pub async fn get_task_result(&self, task_id: Uuid) -> Option<TaskResult> {
        let results = self.results.read().await;
        results.get(&task_id).cloned()
    }

    /// Get all tasks
    pub async fn get_all_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// Get pending tasks
    pub async fn get_pending_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        let completed = self.completed_tasks.read().await;

        tasks.values()
            .filter(|task| !completed.contains(&task.id))
            .cloned()
            .collect()
    }

    /// Get running tasks
    pub async fn get_running_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        let running = self.running_tasks.read().await;

        tasks.values()
            .filter(|task| running.contains(&task.id))
            .cloned()
            .collect()
    }

    /// Get completed tasks
    pub async fn get_completed_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        let completed = self.completed_tasks.read().await;

        tasks.values()
            .filter(|task| completed.contains(&task.id))
            .cloned()
            .collect()
    }

    /// Get task count by status
    pub async fn get_task_counts(&self) -> serde_json::Value {
        let tasks = self.tasks.read().await;
        let completed = self.completed_tasks.read().await;
        let running = self.running_tasks.read().await;

        let total = tasks.len();
        let completed_count = completed.len();
        let running_count = running.len();
        let pending_count = total - completed_count - running_count;

        serde_json::json!({
            "total": total,
            "pending": pending_count,
            "running": running_count,
            "completed": completed_count
        })
    }

    /// Clear completed tasks
    pub async fn clear_completed_tasks(&self) -> Result<()> {
        let mut completed = self.completed_tasks.write().await;
        let mut results = self.results.write().await;

        let count = completed.len();
        completed.clear();
        results.clear();

        info!("Cleared {} completed tasks", count);
        Ok(())
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: Uuid) -> Result<()> {
        let mut running = self.running_tasks.write().await;
        let mut queue = self.task_queue.write().await;

        if running.remove(&task_id) {
            // Add a cancelled result
            let result = TaskResult {
                task_id,
                status: TaskStatus::Cancelled,
                result: None,
                error: Some("Task cancelled".to_string()),
                execution_time: Duration::from_secs(0),
                completed_at: Instant::now(),
                completed_at_utc: Utc::now(),
            };

            self.mark_task_completed(task_id, result).await?;
        }

        // Remove from queue
        let mut new_queue = PriorityQueue::new();
        while let Some((id, priority)) = queue.pop() {
            if id != task_id {
                new_queue.push(id, priority);
            }
        }
        *queue = new_queue;

        Ok(())
    }
}
