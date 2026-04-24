use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub timestamp: SystemTime,
    pub source: String,
    pub destination: Option<String>,
    pub topic: String,
    pub payload: MessagePayload,
    pub metadata: MessageMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    Data(Vec<u8>),
    Text(String),
    Json(serde_json::Value),
    Binary(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub qos: QoS,
    pub persistent: bool,
    pub ttl: Option<std::time::Duration>,
    pub sequence: u64,
    pub headers: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

impl Default for QoS {
    fn default() -> Self {
        QoS::AtMostOnce
    }
}

impl Message {
    pub fn new(topic: String, payload: MessagePayload) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            source: String::new(),
            destination: None,
            topic,
            payload,
            metadata: MessageMetadata::default(),
        }
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = source;
        self
    }

    pub fn with_destination(mut self, destination: String) -> Self {
        self.destination = Some(destination);
        self
    }

    pub fn with_qos(mut self, qos: QoS) -> Self {
        self.metadata.qos = qos;
        self
    }

    pub fn with_persistent(mut self, persistent: bool) -> Self {
        self.metadata.persistent = persistent;
        self
    }

    pub fn with_ttl(mut self, ttl: std::time::Duration) -> Self {
        self.metadata.ttl = Some(ttl);
        self
    }

    pub fn get_text(&self) -> Option<&str> {
        match &self.payload {
            MessagePayload::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn get_json(&self) -> Option<&serde_json::Value> {
        match &self.payload {
            MessagePayload::Json(json) => Some(json),
            _ => None,
        }
    }

    pub fn get_data(&self) -> Vec<u8> {
        match &self.payload {
            MessagePayload::Data(data) | MessagePayload::Binary(data) => data.clone(),
            MessagePayload::Text(text) => text.as_bytes().to_vec(),
            MessagePayload::Json(json) => json.to_string().into_bytes(),
        }
    }
}

impl Default for MessageMetadata {
    fn default() -> Self {
        Self {
            qos: QoS::default(),
            persistent: false,
            ttl: None,
            sequence: 0,
            headers: std::collections::HashMap::new(),
        }
    }
}