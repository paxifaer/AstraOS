# AstraOS Runtime

AstraOS Runtime 是 AstraOS 的执行环境模块，负责管理节点的生命周期、资源分配和任务执行。

## 功能特性

- **任务调度**：支持优先级任务调度，支持任务依赖
- **资源管理**：动态资源分配和释放，支持多种资源类型
- **执行器**：可扩展的任务处理器，支持自定义任务类型
- **Actor 系统**：轻量级并发模型，支持消息传递
- **健康监控**：系统健康检查，自动故障恢复
- **实时监控**：资源使用情况、任务执行状态监控

## 核心组件

### 1. 任务调度器 (Scheduler)

负责任务的优先级调度和依赖管理：

```rust
use astraos_runtime::*;

let (executor, executor_handle) = Executor::new();
let scheduler = Scheduler::new(executor_handle);

// 创建任务
let task = Task::new(
    "process_data",
    TaskPriority::High,
    serde_json::json!({"data": [1, 2, 3]})
);

// 提交任务
scheduler.submit_task(task).await?;
```

### 2. 资源管理器 (ResourceManager)

管理系统资源分配：

```rust
let resource_manager = ResourceManager::new();

// 添加资源
let cpu = Resource::new(ResourceType::CPU, 100.0, "%");
resource_manager.add_resource(cpu).await?;

// 分配资源
let resources = resource_manager.allocate_resources(
    actor_id,
    vec![cpu_id]
).await?;
```

### 3. 执行器 (Executor)

执行任务并处理结果：

```rust
let (mut executor, executor_handle) = Executor::new();

// 注册任务处理器
executor.register_handler(Box::new(MyTaskHandler)).await?;

// 运行执行器
executor.run().await?;
```

### 4. Actor 系统

轻量级并发模型：

```rust
let actor_manager = ActorManager::new();

// 创建 Actor
let actor_handle = ActorHandle::new(actor_id, "my_actor", sender);
actor_manager.register_actor(actor_handle).await?;
```

### 5. 健康监控 (Monitor)

系统健康检查和自动恢复：

```rust
let monitor = Monitor::new(MonitorConfig::default());

// 注册健康检查
monitor.register_health_check("system".to_string(), HealthStatus::Healthy).await?;

// 更新健康状态
monitor.update_health_check(
    "system",
    HealthStatus::Warning,
    "High CPU usage",
    serde_json::json!({"cpu": 90})
).await?;
```

## 使用示例

### 运行示例程序

```bash
cd examples/runtime_example
cargo run
```

### 自定义任务处理器

```rust
use astraos_runtime::*;
use async_trait::async_trait;

#[derive(Clone)]
pub struct MyTaskHandler;

#[async_trait::async_trait]
impl TaskHandler for MyTaskHandler {
    fn name(&self) -> &'static str {
        "my_task"
    }
    
    async fn execute(&self, task_id: uuid::Uuid, payload: serde_json::Value) -> Result<serde_json::Value> {
        // 处理任务逻辑
        let result = /* ... */;
        
        Ok(serde_json::json!({
            "task_id": task_id,
            "result": result
        }))
    }
    
    fn can_handle(&self, task_type: &str) -> bool {
        task_type == "my_task"
    }
}
```

## 任务优先级

系统支持5个优先级级别：

- `Critical` (0) - 系统关键任务
- `High` (1) - 高优先级任务
- `Normal` (2) - 普通任务
- `Low` (3) - 后台任务
- `Idle` (4) - 最低优先级

## 资源类型

支持多种资源类型：

- `CPU` - CPU 资源
- `Memory` - 内存资源
- `GPU` - GPU 资源
- `Network` - 网络资源
- `Storage` - 存储资源
- `Custom` - 自定义资源类型

## 健康状态

系统健康状态分为4个级别：

- `Healthy` - 系统健康
- `Warning` - 系统有警告但仍可运行
- `Critical` - 系统处于关键状态
- `Failed` - 系统已失败

## 依赖关系

- 依赖 `astraos-kernel` 模块
- 使用 Tokio 异步运行时
- 使用 tracing 进行日志记录

## 开发状态

- ✅ 基础框架完成
- ✅ 任务调度器
- ✅ 资源管理器
- ✅ 执行器
- ✅ Actor 系统
- ✅ 健康监控
- ✅ 示例程序

## 下一步计划

1. 实现通信模块
2. 添加实时调度器
3. 实现硬件驱动抽象
4. 添加机器人专用模块
5. 集成 AI/ML 功能