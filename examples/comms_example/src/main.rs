//! A simple example demonstrating the communication system
//! 
//! This example shows:
//! 1. Publishing messages to topics
//! 2. Subscribing to topics
//! 3. Handling incoming messages
//! 4. Basic error handling

use astraos_comms::{
    message::{Message, MessagePayload, QoS},
    topic::{TopicManager, TopicSubscriber, Subscriber},
    CommsResult,
};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::info;

#[tokio::main]
async fn main() -> CommsResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Create a topic manager
    let topic_manager = TopicManager::new();
    
    info!("Starting communication example...");
    
    // Subscribe to a topic
    let topic_name = "example/messages".to_string();
    
    // Create a simple subscriber with a message handler task
    let subscriber_id = create_subscriber(&topic_manager, &topic_name, "subscriber1").await?;
    
    // Publish some messages
    publish_messages(&topic_manager, &topic_name).await?;
    
    // Wait for messages to be processed
    sleep(Duration::from_secs(1)).await;
    
    // List all topics
    let topics = topic_manager.list_topics();
    info!("Available topics: {:?}", topics);
    
    // Get topic info
    if let Some(info) = topic_manager.get_topic_info(&topic_name).await {
        info!("Topic info: {} ({} subscribers)", info.name, info.subscriber_count);
    }
    
    info!("Example completed successfully!");
    
    // Clean up: stop the subscriber task
    let mut tasks = SUBSCRIBER_TASKS.write().await;
    if let Some(handle) = tasks.remove(&subscriber_id) {
        handle.abort();
    }
    
    Ok(())
}

/// Create a subscriber and start a task to receive and handle messages
async fn create_subscriber(
    topic_manager: &TopicManager,
    topic_name: &str,
    name: &str,
) -> CommsResult<String> {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let subscriber = Subscriber::new(name.to_string(), tx);
    
    topic_manager.subscribe(topic_name, subscriber).await?;
    
    let subscriber_id = name.to_string();
    let topic_name = topic_name.to_string();
    
    // Clone for use in the closure
    let subscriber_id_for_closure = subscriber_id.clone();
    
    // Spawn a task to receive and process messages
    let handle = tokio::spawn(async move {
        info!("[{}] Waiting for messages on topic '{}'...", subscriber_id_for_closure, topic_name);
        
        while let Some(message) = rx.recv().await {
            info!("[{}] Received message on topic '{}':", subscriber_id_for_closure, message.topic);
            
            // Handle different payload types
            match &message.payload {
                MessagePayload::Text(text) => {
                    info!("[{}]   Text: {}", subscriber_id_for_closure, text);
                }
                MessagePayload::Json(json) => {
                    info!("[{}]   JSON: {}", subscriber_id_for_closure, json);
                }
                MessagePayload::Binary(data) => {
                    info!("[{}]   Binary: {:?} bytes", subscriber_id_for_closure, data);
                }
                MessagePayload::Data(data) => {
                    info!("[{}]   Data: {:?} bytes", subscriber_id_for_closure, data);
                }
            }
            
            info!("[{}]   Timestamp: {:?}", subscriber_id_for_closure, message.timestamp);
            info!("[{}]   Source: {}", subscriber_id_for_closure, message.source);
        }
        
        info!("[{}] Subscriber stopped", subscriber_id_for_closure);
    });
    
    // Store the task handle for cleanup
    let mut tasks = SUBSCRIBER_TASKS.write().await;
    tasks.insert(subscriber_id.clone(), handle);
    
    Ok(subscriber_id)
}

async fn publish_messages(topic_manager: &TopicManager, topic_name: &str) -> CommsResult<()> {
    info!("Publishing messages to topic: {}", topic_name);
    
    // Message 1: Text payload
    let text_message = Message::new(
        topic_name.to_string(),
        MessagePayload::Text("Hello from text message!".to_string())
    ).with_qos(QoS::AtLeastOnce);
    
    topic_manager.publish(text_message).await?;
    info!("Published text message");
    
    // Message 2: JSON payload
    let json_message = Message::new(
        topic_name.to_string(),
        MessagePayload::Json(serde_json::json!({
            "type": "user_update",
            "user_id": 123,
            "name": "Alice",
            "active": true
        }))
    ).with_qos(QoS::ExactlyOnce);
    
    topic_manager.publish(json_message).await?;
    info!("Published JSON message");
    
    // Message 3: Binary payload
    let binary_data = vec![1, 2, 3, 4, 5];
    let binary_message = Message::new(
        topic_name.to_string(),
        MessagePayload::Binary(binary_data)
    ).with_qos(QoS::AtMostOnce);
    
    topic_manager.publish(binary_message).await?;
    info!("Published binary message");
    
    info!("All messages published successfully");
    Ok(())
}

// Global storage for subscriber task handles
use tokio::sync::RwLock;

static SUBSCRIBER_TASKS: LazyLock<RwLock<HashMap<String, JoinHandle<()>>>> = LazyLock::new(|| {
    RwLock::new(HashMap::new())
});
