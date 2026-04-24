# AstraOS Comms

AstraOS Comms 是 AstraOS 的通信模块，负责实现节点间的消息传递，包括话题、服务和动作等通信机制。

## 功能特性

- **话题通信**：支持发布/订阅模式，支持多种QoS等级
- **服务通信**：支持请求/响应模式，支持超时控制
- **动作通信**：支持长期运行的任务，支持目标状态跟踪
- **传输层**：支持TCP、UDP、内存等多种传输方式
- **服务发现**：自动发现网络中的服务和节点
- **消息序列化**：支持多种序列化格式

## 快速开始

### 基本使用

```rust
use astraos_comms::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化话题管理器
    let topic_manager = TopicManager::new();
    
    // 创建消息
    let message = Message::new("test_topic".to_string(), MessagePayload::Text("Hello, AstraOS!".to_string()));
    
    // 发布消息
    topic_manager.publish(message).await?;
    
    // 订阅话题
    let (tx, mut rx) = mpsc::unbounded_channel();
    let subscriber = Subscriber::new("test_subscriber".to_string(), tx);
    
    topic_manager.subscribe("test_topic", subscriber).await?;
    
    // 接收消息
    while let Some(message) = rx.recv().await {
        println!("Received: {}", message.get_text().unwrap());
    }
    
    Ok(())
}
```

### 服务调用

```rust
use astraos_comms::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建服务客户端
    let client = ServiceClient::new();
    
    // 创建请求
    let request = Request::new("echo_service".to_string(), json!({
        "message": "Hello, World!"
    }));
    
    // 调用服务
    let response = client.call("echo_service", request).await?;
    
    if response.success {
        println!("Response: {}", response.data);
    } else {
        println!("Error: {}", response.error.unwrap());
    }
    
    Ok(())
}
```

### 动作执行

```rust
use astraos_comms::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建动作客户端
    let client = ActionClient::new();
    
    // 创建目标
    let goal = Goal::new("move_action".to_string(), json!({
        "x": 1.0,
        "y": 2.0,
        "z": 0.0
    }));
    
    // 发送目标
    let mut handle = client.send_goal("move_action", goal).await?;
    
    // 等待结果
    let status = handle.get_result().await?;
    
    match status.status {
        GoalState::Succeeded => {
            println!("Action succeeded: {:?}", status.result);
        }
        GoalState::Aborted => {
            println!("Action aborted: {:?}", status.error);
        }
        _ => {
            println!("Action status: {:?}", status.status);
        }
    }
    
    Ok(())
}
```

## API 参考

### TopicManager

话题管理器，负责话题的创建、发布和订阅。

```rust
pub struct TopicManager {
    topics: DashMap<String, Topic>,
    subscribers: DashMap<String, Vec<Subscriber>>,
}
```

**主要方法：**
- `publish(message: Message)` - 发布消息到话题
- `subscribe(topic_name: &str, subscriber: Subscriber)` - 订阅话题
- `unsubscribe(topic_name: &str, subscriber_id: Uuid)` - 取消订阅
- `list_topics()` - 列出所有话题
- `get_topic_info(topic_name: &str)` - 获取话题信息

### ServiceClient

服务客户端，用于调用服务。

```rust
pub struct ServiceClient {
    service_manager: Arc<ServiceManager>,
}
```

**主要方法：**
- `call(service_name: &str, request: Request)` - 调用服务
- `call_with_timeout(service_name: &str, request: Request, timeout: Duration)` - 带超时的服务调用

### ActionClient

动作客户端，用于执行长期运行的任务。

```rust
pub struct ActionClient {
    action_server: Arc<ActionServer>,
}
```

**主要方法：**
- `send_goal(action_name: &str, goal: Goal)` - 发送目标
- `cancel_goal(action_name: &str, goal_id: Uuid)` - 取消目标

### Transport

传输层接口，支持不同的传输方式。

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, message: Message) -> CommsResult<()>;
    async fn receive(&mut self) -> CommsResult<Message>;
    fn get_info(&self) -> TransportInfo;
}
```

**内置传输类型：**
- `TcpTransport` - TCP传输
- `UdpTransport` - UDP传输
- `MemoryTransport` - 内存传输

### DiscoveryManager

服务发现管理器，用于发现网络中的服务和节点。

```rust
pub struct DiscoveryManager {
    services: Arc<TokioRwLock<HashMap<String, ServiceInfo>>>,
    nodes: Arc<TokioRwLock<HashMap<String, NodeInfo>>>,
    topics: Arc<TokioRwLock<HashMap<String, TopicInfo>>>,
}
```

**主要方法：**
- `register_service(service_info: ServiceInfo)` - 注册服务
- `find_service(service_name: &str)` - 查找服务
- `register_node(node_info: NodeInfo)` - 注册节点
- `heartbeat(node_name: &str)` - 发送心跳
- `cleanup_stale(timeout: Duration)` - 清理过期节点

## 配置

### Cargo.toml

```toml
[dependencies]
astraos-comms = "0.1.0"
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

### 特性开关

```toml
[features]
default = ["tcp", "udp"]
tcp = []
udp = []
websocket = []
memory = []
```

## 示例

完整的示例代码请参考 `examples/comms_example` 目录。

## 许可证

MIT 或 Apache 2.0