use astraos_runtime::*;
use tokio::time::{sleep, Duration};
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("Starting AstraOS Runtime Example");
    
    // Create resource manager
    let resource_manager = ResourceManager::new();
    
    // Add some resources
    let cpu_resource = Resource::new(ResourceType::CPU, 100.0, "%");
    let memory_resource = Resource::new(ResourceType::Memory, 8192.0, "MB");
    
    resource_manager.add_resource(cpu_resource).await?;
    resource_manager.add_resource(memory_resource).await?;
    
    // Create executor
    let (mut executor, executor_handle) = Executor::new();
    
    // Register task handlers
    executor.register_handler(Box::new(MathTaskHandler)).await?;
    
    // Create scheduler
    let scheduler = Scheduler::new();
    
    // Create actor manager
    let actor_manager = ActorManager::new();
    
    // Create monitor
    let monitor = Monitor::new(MonitorConfig::default());
    
    // Register health checks
    monitor.register_health_check("system".to_string(), HealthStatus::Healthy).await?;
    monitor.register_health_check("executor".to_string(), HealthStatus::Healthy).await?;
    monitor.register_health_check("scheduler".to_string(), HealthStatus::Healthy).await?;
    
    // Register recovery handler
    monitor.register_recovery_handler(Box::new(RestartRecoveryHandler::new(5))).await?;
    
    // Spawn tasks
    let task1 = Task::new(
        "math_task".to_string(),
        TaskPriority::Normal,
        serde_json::json!({
            "type": "math",
            "message": "Hello from AstraOS!"
        })
    );
    
    let task2 = Task::new(
        "math_task_2".to_string(),
        TaskPriority::High,
        serde_json::json!({
            "type": "math",
            "operation": "add",
            "a": 10,
            "b": 20
        })
    );
    
    // Submit tasks
    scheduler.submit_task(task1).await?;
    scheduler.submit_task(task2).await?;
    
    // Spawn executor
    let executor_handle_clone = executor_handle.clone();
    tokio::spawn(async move {
        if let Err(e) = executor.run().await {
            error!("Executor error: {}", e);
        }
    });
    
    // Spawn monitor
    let monitor_handle = monitor.clone();
    tokio::spawn(async move {
        if let Err(e) = monitor_handle.start_monitoring().await {
            error!("Monitor error: {}", e);
        }
    });
    
    // Simulate system monitoring
    for i in 0..10 {
        // Update health status
        let status = if i % 3 == 0 {
            HealthStatus::Warning
        } else if i % 5 == 0 {
            HealthStatus::Critical
        } else {
            HealthStatus::Healthy
        };
        
        monitor.update_health_check(
            "system",
            status,
            Some(format!("System check {}", i)),
            Some(serde_json::json!({"cpu_usage": i * 10}))
        ).await?;
        
        // Check resource usage
        let usage = resource_manager.get_resource_usage().await;
        info!("Resource usage: {}", usage);
        
        // Check task status
        let tasks = scheduler.get_all_tasks().await;
        info!("Tasks in queue: {}", tasks.len());
        
        sleep(Duration::from_secs(2)).await;
    }
    
    // Shutdown
    info!("Shutting down runtime example");
    executor_handle.shutdown().await?;
    
    Ok(())
}

// Math task handler
#[derive(Clone)]
pub struct MathTaskHandler;

#[async_trait::async_trait]
impl TaskHandler for MathTaskHandler {
    fn name(&self) -> &'static str {
        "math"
    }
    
    async fn execute(&self, task_id: uuid::Uuid, payload: serde_json::Value) -> Result<serde_json::Value> {
        let operation = payload.get("operation")
            .and_then(|o| o.as_str())
            .unwrap_or("unknown");
        
        let a = payload.get("a")
            .and_then(|a| a.as_u64())
            .unwrap_or(0);
        
        let b = payload.get("b")
            .and_then(|b| b.as_u64())
            .unwrap_or(0);
        
        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => if b != 0 { a / b } else { 0 },
            _ => 0,
        };
        
        Ok(serde_json::json!({
            "task_id": task_id,
            "operation": operation,
            "result": result,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
    
    fn can_handle(&self, task_type: &str) -> bool {
        task_type == "math"
    }
}
