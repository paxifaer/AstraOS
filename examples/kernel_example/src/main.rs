use astraos_kernel::*;
use tokio::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting AstraOS Kernel Example");

    // Create kernel instance
    let _kernel: System = init()?;

    // Create nodes
    let node1 = Node::new(
        "node1".to_string(),
        "publisher".to_string(),
        Config::default(),
    );

    let node2 = Node::new(
        "node2".to_string(),
        "subscriber".to_string(),
        Config::default(),
    );

    let node3 = Node::new(
        "node3".to_string(),
        "service".to_string(),
        Config::default(),
    );

    info!("Created nodes successfully");

    // Start nodes
    node1.start().await?;
    node2.start().await?;
    node3.start().await?;

    info!("All nodes started successfully");

    // Simulate some operations
    for i in 0..5 {
        // Update node metadata
        let metadata = serde_json::json!({
            "batch_size": i,
            "iteration": i
        });
        node3.update_metadata(metadata.clone()).await?;

        info!("Updated node3 metadata: batch_size = {}", i);

        // Get node info
        let info = node3.get_info().await;
        info!("Node3 status: {:?}", info.status);

        // Sleep for a bit
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // Stop nodes
    node1.stop().await?;
    node2.stop().await?;
    node3.stop().await?;

    info!("All nodes stopped");

    Ok(())
}
