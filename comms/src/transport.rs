use async_trait::async_trait;
use bincode;
use bytes::{Buf, BytesMut};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_util::codec::{Decoder, Encoder};
use tracing::error;

use crate::{Message, CommsError, CommsResult};

pub struct TransportManager {
    transports: Arc<RwLock<HashMap<String, Box<dyn Transport>>>>,
}

impl TransportManager {
    pub fn new() -> Self {
        Self {
            transports: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_transport(&self, name: String, transport: Box<dyn Transport>) -> CommsResult<()> {
        let name_clone = name.clone();
        self.transports.write().await.insert(name, transport);
        tracing::info!("Registered transport: {}", name_clone);
        Ok(())
    }

    pub async fn get_transport(&self, name: &str) -> Option<String> {
        if self.transports.read().await.contains_key(name) {
            Some(name.to_string())
        } else {
            None
        }
    }

    pub async fn list_transports(&self) -> Vec<String> {
        self.transports.read().await.keys().cloned().collect()
    }
}

#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, message: Message) -> CommsResult<()>;
    async fn receive(&mut self) -> CommsResult<Message>;
    fn get_info(&self) -> TransportInfo;
}

#[derive(Debug, Clone)]
pub struct TransportInfo {
    pub name: String,
    pub protocol: String,
    pub address: String,
    pub connected: bool,
}

pub struct TcpTransport {
    address: String,
    connection: Option<Arc<Mutex<TcpStream>>>,
}

impl TcpTransport {
    pub fn new(address: String) -> Self {
        Self {
            address,
            connection: None,
        }
    }

    pub async fn connect(&mut self) -> CommsResult<()> {
        let stream = TcpStream::connect(&self.address).await
            .map_err(|e| CommsError::ConnectionError(e.to_string()))?;
        self.connection = Some(Arc::new(Mutex::new(stream)));
        Ok(())
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn send(&self, message: Message) -> CommsResult<()> {
        if let Some(ref stream) = self.connection {
            use tokio_util::codec::Encoder;
            use tokio::io::AsyncWriteExt;
            let mut codec = TcpCodec;
            let mut bytes = BytesMut::new();
            codec.encode(message, &mut bytes)?;

            let stream_mutex = Arc::clone(stream);
            let mut stream = stream_mutex.lock().await;
            stream.write_all(&bytes).await
                .map_err(|e| CommsError::TransportError(e.to_string()))?;
        } else {
            return Err(CommsError::ConnectionError("Not connected".to_string()));
        }
        Ok(())
    }

    async fn receive(&mut self) -> CommsResult<Message> {
        if let Some(ref stream) = self.connection {
            use tokio::io::AsyncReadExt;
            let mut size_buf = [0u8; 4];
            let stream_mutex = Arc::clone(stream);
            let mut stream = stream_mutex.lock().await;
            stream.read_exact(&mut size_buf).await
                .map_err(|e: std::io::Error| CommsError::IoError(e))?;
            let size = u32::from_be_bytes(size_buf) as usize;

            let mut data_buf = vec![0u8; size];
            stream.read_exact(&mut data_buf).await
                .map_err(|e: std::io::Error| CommsError::IoError(e))?;

            let message = bincode::deserialize(&data_buf)
                .map_err(|e| CommsError::SerializationError(e.to_string()))?;
            Ok(message)
        } else {
            return Err(CommsError::ConnectionError("Not connected".to_string()));
        }
    }

    fn get_info(&self) -> TransportInfo {
        TransportInfo {
            name: "tcp".to_string(),
            protocol: "TCP".to_string(),
            address: self.address.clone(),
            connected: self.connection.is_some(),
        }
    }
}

pub struct UdpTransport {
    address: String,
    socket: Option<Arc<tokio::net::UdpSocket>>,
}

impl UdpTransport {
    pub fn new(address: String) -> Self {
        Self {
            address,
            socket: None,
        }
    }

    pub async fn bind(&mut self) -> CommsResult<()> {
        let socket = tokio::net::UdpSocket::bind(&self.address).await
            .map_err(|e| CommsError::TransportError(e.to_string()))?;
        self.socket = Some(Arc::new(socket));
        Ok(())
    }
}

#[async_trait]
impl Transport for UdpTransport {
    async fn send(&self, message: Message) -> CommsResult<()> {
        if let Some(ref socket) = self.socket {
            let data = bincode::serialize(&message)
                .map_err(|e| CommsError::SerializationError(e.to_string()))?;
            socket.as_ref().send_to(&data, &self.address).await
                .map_err(|e| CommsError::TransportError(e.to_string()))?;
        } else {
            return Err(CommsError::ConnectionError("Not bound".to_string()));
        }
        Ok(())
    }

    async fn receive(&mut self) -> CommsResult<Message> {
        if let Some(ref socket) = self.socket {
            let mut buffer = vec![0; 65535];
            let (size, _) = socket.as_ref().recv_from(&mut buffer).await
                .map_err(|e| CommsError::TransportError(e.to_string()))?;
            let data = &buffer[..size];
            let message = bincode::deserialize(data)
                .map_err(|e| CommsError::SerializationError(e.to_string()))?;
            Ok(message)
        } else {
            return Err(CommsError::ConnectionError("Not bound".to_string()));
        }
    }

    fn get_info(&self) -> TransportInfo {
        TransportInfo {
            name: "udp".to_string(),
            protocol: "UDP".to_string(),
            address: self.address.clone(),
            connected: self.socket.is_some(),
        }
    }
}

pub struct MemoryTransport {
    name: String,
    subscribers: Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<Message>>>>>,
}

impl MemoryTransport {
    pub fn new(name: String) -> Self {
        Self {
            name,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn subscribe(&self, topic: String, sender: mpsc::UnboundedSender<Message>) -> CommsResult<()> {
        let mut subscribers = self.subscribers.write().await;
        let list = subscribers.entry(topic).or_insert_with(Vec::new);
        list.push(sender);
        Ok(())
    }

    pub async fn publish(&self, message: Message) -> CommsResult<()> {
        let topic = message.topic.clone();
        let subscribers = self.subscribers.read().await;

        if let Some(list) = subscribers.get(&topic) {
            for sender in list {
                if let Err(e) = sender.send(message.clone()) {
                    error!("Failed to send message: {}", e);
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Transport for MemoryTransport {
    async fn send(&self, message: Message) -> CommsResult<()> {
        self.publish(message).await
    }

    async fn receive(&mut self) -> CommsResult<Message> {
        Err(CommsError::TransportError("Memory transport doesn't support direct receive".to_string()))
    }

    fn get_info(&self) -> TransportInfo {
        TransportInfo {
            name: self.name.clone(),
            protocol: "Memory".to_string(),
            address: "local".to_string(),
            connected: true,
        }
    }
}

pub struct TcpCodec;

impl Encoder<Message> for TcpCodec {
    type Error = CommsError;

    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let data = bincode::serialize(&item)
            .map_err(|e| CommsError::SerializationError(e.to_string()))?;
        dst.extend_from_slice(&(data.len() as u32).to_be_bytes());
        dst.extend_from_slice(&data);
        Ok(())
    }
}

impl Decoder for TcpCodec {
    type Item = Message;
    type Error = CommsError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }

        let mut size_bytes = [0; 4];
        size_bytes.copy_from_slice(&src[0..4]);
        let size = u32::from_be_bytes(size_bytes) as usize;

        if src.len() < size + 4 {
            return Ok(None);
        }

        let data = src[4..size + 4].to_vec();
        src.advance(size + 4);

        let message = bincode::deserialize(&data)
            .map_err(|e| CommsError::SerializationError(e.to_string()))?;
        Ok(Some(message))
    }
}
