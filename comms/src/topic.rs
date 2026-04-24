use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info};
use uuid::Uuid;

use crate::{Message, CommsError, CommsResult};

pub struct TopicManager {
    topics: Arc<RwLock<HashMap<String, Topic>>>,
    subscribers: Arc<RwLock<HashMap<String, Subscriber>>>,
}

impl TopicManager {
    pub fn new() -> Self {
        Self {
            topics: Arc::new(RwLock::new(HashMap::new())),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn subscribe(&self, topic_name: &str, subscriber: Subscriber) -> CommsResult<()> {
        let mut topics = self.topics.write().await;
        let topic = topics.entry(topic_name.to_string())
            .or_insert_with(|| Topic::new(topic_name.to_string()));
        
        let subscriber_id = subscriber.id.clone();
        topic.subscribe(subscriber_id.clone()).await?;
        
        let mut subscribers = self.subscribers.write().await;
        subscribers.insert(subscriber_id, subscriber);
        
        Ok(())
    }

    pub async fn publish(&self, message: Message) -> CommsResult<()> {
        let topics = self.topics.read().await;
        if let Some(topic) = topics.get(&message.topic) {
            let subscribers = topic.subscribers.read().await;
            let all_subscribers = self.subscribers.read().await;
            
            for subscriber_id in subscribers.iter() {
                if let Some(subscriber) = all_subscribers.get(subscriber_id) {
                    let _ = subscriber.sender.send(message.clone());
                }
            }
        }
        Ok(())
    }

    pub async fn get_topic_info(&self, topic_name: &str) -> Option<TopicInfo> {
        let topics = self.topics.read().await;
        if let Some(topic) = topics.get(topic_name) {
            let subscribers = topic.subscribers.read().await;
            Some(TopicInfo {
                name: topic.name.clone(),
                subscriber_count: subscribers.len(),
            })
        } else {
            None
        }
    }

    pub fn list_topics(&self) -> Vec<TopicInfo> {
        // This is a synchronous version for the example
        // In a real implementation, this should be async
        Vec::new()
    }
}

#[derive(Clone)]
pub struct Topic {
    pub name: String,
    pub subscribers: Arc<RwLock<Vec<String>>>,
    pub metadata: HashMap<String, String>,
}

impl Topic {
    pub fn new(name: String) -> Self {
        Self {
            name,
            subscribers: Arc::new(RwLock::new(Vec::new())),
            metadata: HashMap::new(),
        }
    }

    pub async fn subscribe(&self, subscriber_id: String) -> CommsResult<()> {
        let mut subscribers = self.subscribers.write().await;
        if !subscribers.contains(&subscriber_id) {
            subscribers.push(subscriber_id);
            debug!("Subscribed to topic: {}", self.name);
        }
        Ok(())
    }

    pub async fn unsubscribe(&self, subscriber_id: &str) -> CommsResult<()> {
        let mut subscribers = self.subscribers.write().await;
        subscribers.retain(|id| id != subscriber_id);
        debug!("Unsubscribed from topic: {}", self.name);
        Ok(())
    }

    pub async fn publish(&self, _message: Message) -> CommsResult<()> {
        let subscribers = self.subscribers.read().await;
        debug!("Publishing to topic {} with {} subscribers", self.name, subscribers.len());
        // In a real implementation, you would send the message to all subscribers
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TopicInfo {
    pub name: String,
    pub subscriber_count: usize,
}

/// A subscriber that receives messages via a channel
#[derive(Clone)]
pub struct Subscriber {
    pub id: String,
    pub name: String,
    pub sender: mpsc::UnboundedSender<Message>,
}

impl Subscriber {
    pub fn new(name: String, sender: mpsc::UnboundedSender<Message>) -> Self {
        let id = Uuid::new_v4().to_string();
        Self { id, name, sender }
    }
}

/// Trait for implementing custom message subscribers
#[async_trait::async_trait]
pub trait TopicSubscriber: Send + Sync {
    /// Called when a message is received
    async fn on_message(&self, message: Message) -> CommsResult<()>;
}
