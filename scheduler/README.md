# AstraOS Scheduler

调度器模块负责任务调度、资源管理和实时性保证。

## 功能特性

- 多级优先级调度
- 实时任务支持
- 资源限制和监控
- 任务依赖管理
- 动态优先级调整
- 多核调度支持

## 快速开始

```rust
use astraos_scheduler::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建调度器
    let scheduler = Scheduler::new();
    
    // 创建任务
    let task = Task::new("task1", async {
        println!("执行任务");
        Ok(())
    })
    .with_priority(1)
    .with_deadline(chrono::Duration::seconds(1));
    
    // 提交任务
    let handle = scheduler.submit(task).await?;
    
    // 等待完成
    let result = handle.await?;
    println!("任务结果: {:?}", result);
    
    Ok(())
}
```

## API 文档

详细的API文档请查看 [docs.rs](https://docs.rs/astraos-scheduler)。

## 许可证

MIT/Apache 2.0