use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use smallvec::SmallVec;
use priority_queue::PriorityQueue;
use tracing::{info, warn, error};

use astraos_runtime::{Actor, Message};

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskState {
    Pending,      // 等待执行
    Running,      // 正在执行
    Completed,    // 已完成
    Failed,       // 执行失败
    Cancelled,    // 已取消
    Timeout,      // 超时
}

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// 任务结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskResult<T = ()> {
    Success(T),
    Failed(String),
    Cancelled,
}

/// 任务句柄
pub struct TaskHandle<T = ()> {
    id: Uuid,
    sender: mpsc::UnboundedSender<TaskCommand<T>>,
}

impl<T> TaskHandle<T> {
    pub fn new(id: Uuid, sender: mpsc::UnboundedSender<TaskCommand<T>>) -> Self {
        Self { id, sender }
    }

    pub async fn cancel(&self) -> Result<(), TaskError> {
        self.sender.send(TaskCommand::Cancel(self.id))
            .map_err(|_| TaskError::ChannelClosed)?;
        Ok(())
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}

/// 任务命令
enum TaskCommand<T> {
    Cancel(Uuid),
}

/// 任务定义
pub struct Task<T = ()> {
    id: Uuid,
    name: String,
    priority: Priority,
    deadline: Option<DateTime<Utc>>,
    timeout: Option<Duration>,
    dependencies: SmallVec<[Uuid; 8]>,
    executor: Box<dyn TaskExecutor<T>>,
}

impl<T> Task<T> {
    pub fn new<F, Fut, R>(name: impl Into<String>, future: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<R, TaskError>> + Send + 'static,
        R: Send + 'static,
    {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            priority: Priority::Normal,
            deadline: None,
            timeout: None,
            dependencies: SmallVec::new(),
            executor: Box::new(FutureTaskExecutor {
                future: Some(Box::pin(future())),
            }),
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_dependencies(mut self, deps: impl IntoIterator<Item = Uuid>) -> Self {
        self.dependencies.extend(deps);
        self
    }
}

/// 任务执行器trait
#[async_trait::async_trait]
pub trait TaskExecutor<T>: Send + Sync {
    async fn execute(&mut self) -> TaskResult<T>;
    fn cancel(&mut self) -> bool;
}

/// 适配器，将Future转换为TaskExecutor
struct FutureTaskExecutor<F> {
    future: Option<F>,
}

#[async_trait::async_trait]
impl<F, T> TaskExecutor<T> for FutureTaskExecutor<F>
where
    F: std::future::Future<Output = Result<T, TaskError>> + Send + 'static,
{
    async fn execute(&mut self) -> TaskResult<T> {
        match self.future.take() {
            Some(fut) => match fut.await {
                Ok(value) => TaskResult::Success(value),
                Err(e) => TaskResult::Failed(e.to_string()),
            },
            None => TaskResult::Failed("Task already executed".to_string()),
        }
    }

    fn cancel(&mut self) -> bool {
        self.future = None;
        true
    }
}

/// 任务调度器
pub struct Scheduler {
    tasks: DashMap<Uuid, Task>,
    running: DashMap<Uuid, TaskHandle>,
    completed: DashMap<Uuid, TaskResult>,
    failed: DashMap<Uuid, TaskResult>,
    cancelled: DashMap<Uuid, TaskResult>,
    queue: PriorityQueue<Uuid, Priority>,
    task_map: Arc<DashMap<Uuid, TaskHandle>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: DashMap::new(),
            running: DashMap::new(),
            completed: DashMap::new(),
            failed: DashMap::new(),
            cancelled: DashMap::new(),
            queue: PriorityQueue::new(),
            task_map: Arc::new(DashMap::new()),
        }
    }

    pub async fn submit<T>(&mut self, task: Task<T>) -> Result<TaskHandle<T>, TaskError>
    where
        T: Send + 'static,
    {
        let id = task.id;
        let (tx, rx) = mpsc::unbounded_channel();
        
        let handle = TaskHandle::new(id, tx);
        self.task_map.insert(id, handle.clone());
        self.tasks.insert(id, task);
        self.queue.push(id, task.priority);
        
        // 启动任务执行
        tokio::spawn(async move {
            let mut task = self.tasks.get_mut(&id).unwrap();
            let result = tokio::time::timeout(
                task.timeout.unwrap_or(Duration::from_secs(300)),
                task.executor.execute()
            ).await;

            match result {
                Ok(TaskResult::Success(_)) => {
                    self.completed.insert(id, TaskResult::Success(()));
                }
                Ok(TaskResult::Failed(err)) => {
                    self.failed.insert(id, TaskResult::Failed(err));
                }
                Ok(TaskResult::Cancelled) => {
                    self.cancelled.insert(id, TaskResult::Cancelled);
                }
                Err(_) => {
                    self.failed.insert(id, TaskResult::Failed("Task timeout".to_string()));
                }
            }

            self.running.remove(&id);
        });

        Ok(handle)
    }

    pub fn get_task_state(&self, id: Uuid) -> Option<TaskState> {
        if self.completed.contains_key(&id) {
            Some(TaskState::Completed)
        } else if self.failed.contains_key(&id) {
            Some(TaskState::Failed)
        } else if self.cancelled.contains_key(&id) {
            Some(TaskState::Cancelled)
        } else if self.running.contains_key(&id) {
            Some(TaskState::Running)
        } else {
            Some(TaskState::Pending)
        }
    }

    pub fn get_task_result<T>(&self, id: Uuid) -> Option<TaskResult<T>> {
        self.completed.get(&id).map(|r| r.clone())
            .or_else(|| self.failed.get(&id).map(|r| r.clone()))
            .or_else(|| self.cancelled.get(&id).map(|r| r.clone()))
    }

    pub fn cancel(&mut self, id: Uuid) -> Result<(), TaskError> {
        if let Some(mut task) = self.tasks.get_mut(&id) {
            if task.executor.cancel() {
                self.cancelled.insert(id, TaskResult::Cancelled);
                Ok(())
            } else {
                Err(TaskError::TaskNotFound)
            }
        } else {
            Err(TaskError::TaskNotFound)
        }
    }

    pub fn get_stats(&self) -> SchedulerStats {
        SchedulerStats {
            total: self.tasks.len(),
            pending: self.tasks.len() - self.running.len() - self.completed.len() - self.failed.len() - self.cancelled.len(),
            running: self.running.len(),
            completed: self.completed.len(),
            failed: self.failed.len(),
            cancelled: self.cancelled.len(),
        }
    }
}

/// 调度器统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStats {
    pub total: usize,
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
}

/// 任务错误
#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("Task not found")]
    TaskNotFound,
    #[error("Channel closed")]
    ChannelClosed,
    #[error("Timeout")]
    Timeout,
    #[error("Cancelled")]
    Cancelled,
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskError::TaskNotFound => write!(f, "任务未找到"),
            TaskError::ChannelClosed => write!(f, "通道已关闭"),
            TaskError::Timeout => write!(f, "任务超时"),
            TaskError::Cancelled => write!(f, "任务已取消"),
        }
    }
}

/// 任务 Actor
pub struct TaskActor<T> {
    id: Uuid,
    name: String,
    task: Task<T>,
}

impl<T> TaskActor<T> {
    pub fn new(name: impl Into<String>, task: Task<T>) -> Self {
        Self {
            id: task.id,
            name: name.into(),
            task,
        }
    }
}

#[async_trait::async_trait]
impl<T: Send + 'static> Actor for TaskActor<T> {
    type Message = TaskMessage;

    async fn handle(&mut self, msg: Self::Message, ctx: &mut ActorContext<Self>) {
        match msg {
            TaskMessage::Execute => {
                info!("执行任务: {}", self.name);
                let result = self.task.executor.execute().await;
                ctx.send(TaskMessage::Completed(result));
            }
            TaskMessage::Cancel => {
                if self.task.executor.cancel() {
                    info!("取消任务: {}", self.name);
                    ctx.send(TaskMessage::Cancelled);
                }
            }
            TaskMessage::Completed(result) => {
                info!("任务完成: {}, 结果: {:?}", self.name, result);
            }
            TaskMessage::Cancelled => {
                info!("任务已取消: {}", self.name);
            }
        }
    }
}

/// 任务消息
pub enum TaskMessage {
    Execute,
    Cancel,
    Completed(TaskResult),
    Cancelled,
}