#[cfg(test)]
mod tests {
    use super::super::core_engine::*;
    use super::super::content_addressing::*;
    use blake3;

    #[tokio::test]
    async fn test_dht_engine_creation() {
        let engine = DhtEngine::new(8); // K=8 replication factor
        assert_eq!(engine.replication_factor(), 8);
        assert_eq!(engine.node_count(), 0);
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let mut engine = DhtEngine::new(8);
        let data = b"Test DHT data";
        let hash = ContentHash::new(data);
        
        // Store data
        engine.store(hash.clone(), data.to_vec()).await.unwrap();
        
        // Retrieve data
        let retrieved = engine.retrieve(&hash).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), data);
    }

    #[tokio::test]
    async fn test_node_management() {
        let mut engine = DhtEngine::new(8);
        
        // Add nodes
        for i in 0..10 {
            let node_id = NodeId::from_bytes(&[i; 32]);
            engine.add_node(node_id).await.unwrap();
        }
        
        assert_eq!(engine.node_count(), 10);
        
        // Remove a node
        let node_to_remove = NodeId::from_bytes(&[5; 32]);
        engine.remove_node(&node_to_remove).await.unwrap();
        assert_eq!(engine.node_count(), 9);
    }

    #[tokio::test]
    async fn test_find_closest_nodes() {
        let mut engine = DhtEngine::new(3);
        
        // Add nodes with different distances
        for i in 0..20 {
            let node_id = NodeId::from_bytes(&[i; 32]);
            engine.add_node(node_id).await.unwrap();
        }
        
        let target = ContentHash::new(b"target");
        let closest = engine.find_closest_nodes(&target, 3).await.unwrap();
        
        assert_eq!(closest.len(), 3);
        
        // Verify nodes are sorted by distance
        let distances: Vec<_> = closest.iter()
            .map(|node| engine.calculate_distance(&target, node))
            .collect();
        
        for i in 1..distances.len() {
            assert!(distances[i-1] <= distances[i]);
        }
    }

    #[tokio::test]
    async fn test_replication() {
        let mut engine = DhtEngine::new(3);
        
        // Add nodes
        for i in 0..5 {
            let node_id = NodeId::from_bytes(&[i; 32]);
            engine.add_node(node_id).await.unwrap();
        }
        
        let data = b"Replicated data";
        let hash = ContentHash::new(data);
        
        // Store with replication
        let replicas = engine.store_with_replication(hash.clone(), data.to_vec()).await.unwrap();
        
        // Should replicate to K nodes
        assert_eq!(replicas.len(), 3);
    }

    #[tokio::test]
    async fn test_data_migration_on_node_join() {
        let mut engine = DhtEngine::new(3);
        
        // Add initial nodes and data
        for i in 0..3 {
            let node_id = NodeId::from_bytes(&[i; 32]);
            engine.add_node(node_id).await.unwrap();
        }
        
        let data = b"Migrating data";
        let hash = ContentHash::new(data);
        engine.store(hash.clone(), data.to_vec()).await.unwrap();
        
        // Add new node
        let new_node = NodeId::from_bytes(&[10; 32]);
        engine.add_node_with_migration(new_node).await.unwrap();
        
        // Data should still be retrievable
        let retrieved = engine.retrieve(&hash).await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        use tokio::task;
        
        let engine = DhtEngine::new(8);
        let engine = std::sync::Arc::new(tokio::sync::RwLock::new(engine));
        
        let mut handles = vec![];
        
        // Spawn concurrent store operations
        for i in 0..10 {
            let engine_clone = engine.clone();
            let handle = task::spawn(async move {
                let data = format!("Data {}", i).into_bytes();
                let hash = ContentHash::new(&data);
                let mut engine = engine_clone.write().await;
                engine.store(hash, data).await.unwrap();
            });
            handles.push(handle);
        }
        
        // Wait for all operations
        for handle in handles {
            handle.await.unwrap();
        }
        
        // Verify all data was stored
        let engine = engine.read().await;
        assert!(engine.data_count() >= 10);
    }

    #[tokio::test]
    async fn test_bucket_organization() {
        let mut engine = DhtEngine::new(8);
        
        // Add nodes to different buckets
        for i in 0..160 {
            let mut bytes = [0u8; 32];
            bytes[i / 8] = 1 << (i % 8);
            let node_id = NodeId::from_bytes(&bytes);
            engine.add_node(node_id).await.unwrap();
        }
        
        // Verify bucket distribution
        let buckets = engine.get_bucket_info();
        assert!(buckets.len() > 0);
        
        // Each bucket should have at most K nodes
        for bucket in buckets {
            assert!(bucket.node_count <= 8);
        }
    }

    #[tokio::test]
    async fn test_data_expiration() {
        use tokio::time::{sleep, Duration};
        
        let mut engine = DhtEngine::with_ttl(8, Duration::from_millis(100));
        
        let data = b"Expiring data";
        let hash = ContentHash::new(data);
        
        // Store data
        engine.store(hash.clone(), data.to_vec()).await.unwrap();
        
        // Data should be retrievable immediately
        assert!(engine.retrieve(&hash).await.unwrap().is_some());
        
        // Wait for expiration
        sleep(Duration::from_millis(150)).await;
        
        // Data should be expired
        assert!(engine.retrieve(&hash).await.unwrap().is_none());
    }
}