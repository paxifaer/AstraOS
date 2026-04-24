use crate::{error::Result, config::Config, node::NodeHandle};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

/// Edge in the system graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Source node
    pub source: NodeHandle,
    
    /// Target node
    pub target: NodeHandle,
    
    /// Edge type
    pub edge_type: String,
    
    /// Edge metadata
    pub metadata: serde_json::Value,
}

/// System graph representing the node topology
pub struct SystemGraph {
    nodes: RwLock<HashMap<Uuid, NodeHandle>>,
    edges: RwLock<Vec<Edge>>,
    config: Arc<Config>,
}

impl SystemGraph {
    /// Create a new system graph
    pub fn new(config: Config) -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            edges: RwLock::new(Vec::new()),
            config: Arc::new(config),
        }
    }
    
    /// Add a node to the graph
    pub async fn add_node(&self, node_handle: NodeHandle) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        nodes.insert(node_handle.id, node_handle.clone());
        info!("Node added to graph: {}", node_handle.name);
        Ok(())
    }
    
    /// Remove a node from the graph
    pub async fn remove_node(&self, node_id: Uuid) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        nodes.remove(&node_id);
        
        // Remove edges connected to this node
        let mut edges = self.edges.write().await;
        edges.retain(|edge| edge.source.id != node_id && edge.target.id != node_id);
        
        info!("Node removed from graph: {}", node_id);
        Ok(())
    }
    
    /// Add an edge to the graph
    pub async fn add_edge(&self, source: NodeHandle, target: NodeHandle, edge_type: String) -> Result<()> {
        let edge = Edge {
            source: source.clone(),
            target: target.clone(),
            edge_type,
            metadata: serde_json::json!({}),
        };
        
        let mut edges = self.edges.write().await;
        edges.push(edge);
        
        info!("Edge added from {} to {}", source.name, target.name);
        Ok(())
    }
    
    /// Remove an edge from the graph
    pub async fn remove_edge(&self, source: &NodeHandle, target: &NodeHandle, edge_type: &str) -> Result<()> {
        let mut edges = self.edges.write().await;
        edges.retain(|edge| 
            !(edge.source.id == source.id && 
              edge.target.id == target.id && 
              edge.edge_type == edge_type)
        );
        
        info!("Edge removed from {} to {}", source.name, target.name);
        Ok(())
    }
    
    /// Get all nodes
    pub async fn get_nodes(&self) -> Vec<NodeHandle> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }
    
    /// Get all edges
    pub async fn get_edges(&self) -> Vec<Edge> {
        let edges = self.edges.read().await;
        edges.clone()
    }
    
    /// Get node by ID
    pub async fn get_node(&self, node_id: Uuid) -> Option<NodeHandle> {
        let nodes = self.nodes.read().await;
        nodes.get(&node_id).cloned()
    }
    
    /// Get edges for a node
    pub async fn get_node_edges(&self, node_id: Uuid) -> Result<Vec<Edge>> {
        let edges = self.edges.read().await;
        let node_edges: Vec<Edge> = edges.iter()
            .filter(|edge| edge.source.id == node_id || edge.target.id == node_id)
            .cloned()
            .collect();
        
        Ok(node_edges)
    }
    
    /// Get graph topology
    pub async fn get_topology(&self) -> serde_json::Value {
        let nodes = self.nodes.read().await;
        let edges = self.edges.read().await;
        
        let node_list: Vec<serde_json::Value> = nodes.values()
            .map(|node| serde_json::json!({
                "id": node.id,
                "name": node.name
            }))
            .collect();
        
        let edge_list: Vec<serde_json::Value> = edges.iter()
            .map(|edge| serde_json::json!({
                "source": edge.source.name,
                "target": edge.target.name,
                "type": edge.edge_type
            }))
            .collect();
        
        serde_json::json!({
            "nodes": node_list,
            "edges": edge_list
        })
    }
    
    /// Find path between nodes
    pub async fn find_path(&self, source_id: Uuid, target_id: Uuid) -> Result<Vec<NodeHandle>> {
        // Simple BFS implementation
        let nodes = self.nodes.read().await;
        let edges = self.edges.read().await;
        
        if !nodes.contains_key(&source_id) || !nodes.contains_key(&target_id) {
            return Err(crate::error::KernelError::Graph(
                "Source or target node not found".to_string()
            ));
        }
        
        let mut queue = vec![source_id];
        let mut visited = std::collections::HashSet::new();
        let mut parent = std::collections::HashMap::new();
        visited.insert(source_id);
        
        while let Some(current_id) = queue.pop() {
            if current_id == target_id {
                // Reconstruct path
                let mut path = Vec::new();
                let mut current = target_id;
                
                while current != source_id {
                    if let Some(node) = nodes.get(&current) {
                        path.push(node.clone());
                    }
                    current = parent[&current];
                }
                
                if let Some(source_node) = nodes.get(&source_id) {
                    path.push(source_node.clone());
                }
                
                path.reverse();
                return Ok(path);
            }
            
            // Find neighbors
            for edge in edges.iter() {
                let neighbor = if edge.source.id == current_id {
                    edge.target.id
                } else if edge.target.id == current_id {
                    edge.source.id
                } else {
                    continue;
                };
                
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    parent.insert(neighbor, current_id);
                    queue.push(neighbor);
                }
            }
        }
        
        Err(crate::error::KernelError::Graph(
            "No path found between nodes".to_string()
        ))
    }
    
    /// Get connected components
    pub async fn get_connected_components(&self) -> Result<Vec<Vec<NodeHandle>>> {
        let nodes = self.nodes.read().await;
        let edges = self.edges.read().await;
        
        let mut visited = std::collections::HashSet::new();
        let mut components = Vec::new();
        
        for node_id in nodes.keys() {
            if !visited.contains(node_id) {
                let mut component = Vec::new();
                let mut queue = vec![*node_id];
                visited.insert(*node_id);
                
                while let Some(current_id) = queue.pop() {
                    if let Some(node) = nodes.get(&current_id) {
                        component.push(node.clone());
                    }
                    
                    // Find neighbors
                    for edge in edges.iter() {
                        let neighbor = if edge.source.id == current_id {
                            edge.target.id
                        } else if edge.target.id == current_id {
                            edge.source.id
                        } else {
                            continue;
                        };
                        
                        if !visited.contains(&neighbor) {
                            visited.insert(neighbor);
                            queue.push(neighbor);
                        }
                    }
                }
                
                components.push(component);
            }
        }
        
        Ok(components)
    }
}