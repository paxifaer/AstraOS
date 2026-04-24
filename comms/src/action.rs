use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::info;
use uuid::Uuid;

use crate::{CommsError, CommsResult};

pub struct ActionServer {
    actions: DashMap<String, Action>,
}

impl ActionServer {
    pub fn new() -> Self {
        Self {
            actions: DashMap::new(),
        }
    }

    pub async fn register_action(&self, action: Action) -> CommsResult<()> {
        let action_name = action.name.clone();
        self.actions.insert(action_name, action.clone());
        info!("Registered action: {}", action.name);
        Ok(())
    }

    pub async fn unregister_action(&self, action_name: &str) -> CommsResult<()> {
        if self.actions.remove(action_name).is_some() {
            info!("Unregistered action: {}", action_name);
            Ok(())
        } else {
            Err(CommsError::ServiceNotFound(action_name.to_string()))
        }
    }

    pub async fn execute_action(&self, action_name: &str, goal: Goal) -> CommsResult<GoalStatus> {
        if let Some(action) = self.actions.get(action_name) {
            action.execute(goal).await
        } else {
            Err(CommsError::ServiceNotFound(action_name.to_string()))
        }
    }

    pub fn list_actions(&self) -> Vec<String> {
        self.actions.iter().map(|a| a.key().clone()).collect()
    }
}

#[derive(Clone)]
pub struct Action {
    pub name: String,
    pub executor: Arc<dyn ActionExecutor>,
    pub goal_count: Arc<std::sync::atomic::AtomicU64>,
}

impl Action {
    pub fn new<T>(name: String, executor: T) -> Self
    where
        T: ActionExecutor + Send + Sync + 'static,
    {
        Self {
            name,
            executor: Arc::new(executor),
            goal_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub async fn execute(&self, goal: Goal) -> CommsResult<GoalStatus> {
        self.goal_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.executor.execute(goal).await
    }
}

#[async_trait]
pub trait ActionExecutor: Send + Sync {
    async fn execute(&self, goal: Goal) -> CommsResult<GoalStatus>;
    async fn cancel(&self, goal_id: Uuid) -> CommsResult<()>;
}

pub struct ActionClient {
    action_server: Arc<ActionServer>,
}

impl ActionClient {
    pub fn new() -> Self {
        Self {
            action_server: Arc::new(ActionServer::new()),
        }
    }

    pub async fn send_goal(&self, action_name: &str, goal: Goal) -> CommsResult<GoalHandle> {
        let (tx, rx) = oneshot::channel();
        let goal_handle = GoalHandle::new(goal.id, rx);
        
        let goal_status = self.action_server.execute_action(action_name, goal).await?;
        tx.send(goal_status).map_err(|_| {
            CommsError::ConnectionError("Failed to send goal status".to_string())
        })?;
        
        Ok(goal_handle)
    }

    pub async fn cancel_goal(&self, action_name: &str, goal_id: Uuid) -> CommsResult<()> {
        if let Some(action) = self.action_server.actions.get(action_name) {
            action.executor.cancel(goal_id).await
        } else {
            Err(CommsError::ServiceNotFound(action_name.to_string()))
        }
    }
}

impl Default for ActionClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Goal {
    pub id: Uuid,
    pub action: String,
    pub parameters: serde_json::Value,
}

impl Goal {
    pub fn new(action: String, parameters: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            action,
            parameters,
        }
    }

    pub fn get_param<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.parameters.get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

#[derive(Debug)]
pub struct GoalHandle {
    pub goal_id: Uuid,
    receiver: Option<oneshot::Receiver<GoalStatus>>,
}

impl GoalHandle {
    pub fn new(goal_id: Uuid, receiver: oneshot::Receiver<GoalStatus>) -> Self {
        Self {
            goal_id,
            receiver: Some(receiver),
        }
    }

    pub async fn get_result(&mut self) -> CommsResult<GoalStatus> {
        if let Some(receiver) = self.receiver.take() {
            receiver.await
                .map_err(|_| CommsError::ConnectionError("Goal handle dropped".to_string()))
        } else {
            Err(CommsError::ConnectionError("Goal already consumed".to_string()))
        }
    }

    pub fn cancel(&self) -> CommsResult<()> {
        Err(CommsError::ConnectionError("Cancellation not implemented".to_string()))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoalStatus {
    pub goal_id: Uuid,
    pub status: GoalState,
    pub result: Option<serde_json::Value>,
    pub feedback: Option<serde_json::Value>,
}

impl GoalStatus {
    pub fn new(goal_id: Uuid, status: GoalState) -> Self {
        Self {
            goal_id,
            status,
            result: None,
            feedback: None,
        }
    }

    pub fn with_result(mut self, result: serde_json::Value) -> Self {
        self.result = Some(result);
        self
    }

    pub fn with_feedback(mut self, feedback: serde_json::Value) -> Self {
        self.feedback = Some(feedback);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GoalState {
    Pending = 0,
    Accepted = 1,
    Executing = 2,
    Succeeded = 3,
    Aborted = 4,
    Rejected = 5,
    Preempted = 6,
}

impl Default for GoalState {
    fn default() -> Self {
        GoalState::Pending
    }
}
