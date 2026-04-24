# AstraOS Kernel

AstraOS Kernel 是 AstraOS 的核心模块，负责节点管理、服务发现、配置管理和生命周期控制。

## 功能特性

- **节点管理**：创建、启动、停止、监控节点
- **服务发现**：自动发现和注册服务
- **拓扑管理**：基于图结构的节点关系管理
- **配置管理**：动态配置更新和热重载
- **生命周期控制**：完整的节点生命周期管理
- **状态监控**：实时节点状态和性能监控

## 核心组件

### 1. 核心结构

```rust
use astraos_kernel::*;

// 创建内核实例
let kernel = Kernel::new(KernelConfig {
    name: "my_robot".to_string(),
    node_id: uuid::Uuid::new_v4(),
    max_nodes: 100,
    enable_discovery: true,
    discovery_interval: Duration::from_secs(5),
}).await?;
```

### 2. 节点类型

系统支持三种节点类型：

- `Publisher` - 发布节点，发布数据到话题
- `Subscriber` - 订阅节点，订阅话题数据
- `Service` - 服务节点，提供RPC服务

```rust
let node = Node::new(
    "sensor_node",
    NodeType::Publisher,
    vec!["lidar_data".to_string()],
    vec!["control_cmd".to_string()],
    serde_json::json!({
        "sensor_type": "lidar",
        "rate": 10
    })
);
```

### 3. 节点操作

```rust
// 注册节点
kernel.register_node(node).await?;

// 启动节点
kernel.start_node("node_name").await?;

// 停止节点
kernel.stop_node("node_name").await?;

// 更新配置
let new_config = serde_json::json!({"rate": 20});
kernel.update_node_config("node_name", new_config).await?;

// 获取状态
let status = kernel.get_node_status("node_name").await?;
```

## 使用示例

### 运行示例程序

```bash
cd os/examples/kernel_example
cargo run
```

### 自定义节点

```rust
use astraos_kernel::*;
use serde_json::Value;

// 创建自定义节点
let custom_node = Node::new(
    "custom_node",
    NodeType::Publisher,
    vec!["custom_topic".to_string()],
    vec![],
    serde_json::json!({
        "custom_param": "value"
    })
);

// 注册并启动
kernel.register_node(custom_node).await?;
kernel.start_node("custom_node").await?;
```

## 节点状态

节点状态包括：

- `Initializing` - 初始化中
- `Running` - 运行中
- `Paused` - 暂停
- `Stopping` - 停止中
- `Stopped` - 已停止
- `Error` - 错误状态

## 配置管理

支持动态配置更新：

```rust
// 更新节点配置
let mut config = node.config.clone();
config.data["new_param"] = serde_json::Value::String("new_value");
kernel.update_node_config("node_name", config).await?;
```

## 服务发现

自动发现和注册服务：

```rust
// 启用服务发现
let mut kernel = Kernel::new(KernelConfig {
    enable_discovery: true,
    discovery_interval: Duration::from_secs(5),
    // ... 其他配置
}).await?;
```

## 依赖关系

- 使用 Tokio 异步运行时
- 使用 tracing 进行日志记录
- 使用 serde 进行序列化
- 使用 uuid 生成唯一标识

## 开发状态

- ✅ 基础框架完成
- ✅ 节点管理
- ✅ 服务发现
- ✅ 配置管理
- ✅ 生命周期控制
- ✅ 状态监控
- ✅ 示例程序

## 下一步计划

1. 实现通信模块
2. 添加实时调度器
3. 实现硬件驱动抽象
4. 添加机器人专用模块
5. 集成 AI/ML 功能