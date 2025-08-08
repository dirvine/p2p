// Copyright 2024 Saorsa Labs Limited
//
// This software is dual-licensed under:
// - GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later)
// - Commercial License
//
// For AGPL-3.0 license, see LICENSE-AGPL-3.0
// For commercial licensing, contact: saorsalabs@gmail.com
//
// Unless required by applicable law or agreed to in writing, software
// distributed under these licenses is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.

//! Integration tests for the storage and retrieval system

#[cfg(test)]
mod tests {
    use crate::adaptive::*;
    use crate::network::ContentType;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::time::{sleep, Duration};
    
    /// Create a complete test storage system
    async fn create_test_storage_system() -> (
        Arc<ContentStore>,
        Arc<ReplicationManager>,
        Arc<RetrievalManager>,
        Arc<ChunkManager>,
        TempDir,
    ) {
        let temp_dir = TempDir::new().unwrap();
        let config = StorageConfig {
            db_path: temp_dir.path().to_str().unwrap().to_string(),
            chunk_size: 1024, // Small chunks for testing
            ..Default::default()
        };
        
        // Create components
        let content_store = Arc::new(ContentStore::new(config.clone()).await.unwrap());
        let trust_provider = Arc::new(trust::MockTrustProvider::new());
        let churn_predictor = Arc::new(ChurnPredictor::new());
        let hyperbolic = Arc::new(HyperbolicSpace::new());
        let som = Arc::new(SelfOrganizingMap::new(10, 10, 4));
        let router = Arc::new(AdaptiveRouter::new(
            trust_provider.clone(),
            hyperbolic,
            som,
        ));
        
        let replication_manager = Arc::new(ReplicationManager::new(
            config.replication_config,
            trust_provider,
            churn_predictor,
            router.clone(),
        ));
        
        let cache_manager = Arc::new(QLearnCacheManager::new(config.cache_size));
        let retrieval_manager = Arc::new(RetrievalManager::new(
            router,
            content_store.clone(),
            cache_manager,
        ));
        
        let chunk_manager = Arc::new(ChunkManager::new(config.chunk_size));
        
        (content_store, replication_manager, retrieval_manager, chunk_manager, temp_dir)
    }
    
    #[tokio::test]
    async fn test_end_to_end_storage_retrieval() {
        let (store, replication, retrieval, chunks, _temp) = create_test_storage_system().await;
        
        // Test content
        let content = b"This is a test content that will be stored and retrieved".to_vec();
        let hash = ContentStore::calculate_hash(&content);
        
        // Store content
        let metadata = storage::ContentMetadata {
            size: content.len(),
            content_type: ContentType::DataRetrieval,
            created_at: std::time::Instant::now(),
            chunk_count: None,
            replication_factor: 8,
        };
        
        store.store(content.clone(), metadata.clone()).await.unwrap();
        
        // Verify local retrieval
        let retrieved = store.retrieve(&hash).await.unwrap();
        assert_eq!(retrieved, Some(content.clone()));
        
        // Test retrieval manager
        let retrieved = retrieval.retrieve(&hash, RetrievalStrategy::Parallel).await.unwrap();
        assert_eq!(retrieved, content);
        
        // Check statistics
        let stats = retrieval.get_stats().await;
        assert_eq!(stats.successful_retrievals, 1);
    }
    
    #[tokio::test]
    async fn test_chunked_content_storage() {
        let (store, _replication, _retrieval, chunks, _temp) = create_test_storage_system().await;
        
        // Large content that will be chunked
        let large_content = vec![42u8; 5000]; // 5KB will create 5 chunks of 1KB each
        let parent_hash = ContentStore::calculate_hash(&large_content);
        
        // Create chunks
        let content_chunks = chunks.create_chunks(&large_content, parent_hash.clone());
        assert_eq!(content_chunks.len(), 5);
        
        // Store each chunk
        for chunk in &content_chunks {
            let chunk_metadata = storage::ContentMetadata {
                size: chunk.data.len(),
                content_type: ContentType::DataRetrieval,
                created_at: std::time::Instant::now(),
                chunk_count: None,
                replication_factor: 8,
            };
            
            store.store(chunk.data.clone(), chunk_metadata).await.unwrap();
        }
        
        // Retrieve and reassemble chunks
        let mut retrieved_chunks = Vec::new();
        for chunk in &content_chunks {
            let data = store.retrieve(&chunk.metadata.chunk_hash).await.unwrap().unwrap();
            retrieved_chunks.push(storage::Chunk {
                metadata: chunk.metadata.clone(),
                data,
            });
        }
        
        let reassembled = chunks.reassemble_chunks(retrieved_chunks).unwrap();
        assert_eq!(reassembled, large_content);
    }
    
    #[tokio::test]
    async fn test_adaptive_replication() {
        let (store, replication, _retrieval, _chunks, _temp) = create_test_storage_system().await;
        
        let content = b"Content to replicate".to_vec();
        let hash = ContentStore::calculate_hash(&content);
        let metadata = storage::ContentMetadata {
            size: content.len(),
            content_type: ContentType::DataRetrieval,
            created_at: std::time::Instant::now(),
            chunk_count: None,
            replication_factor: 8,
        };
        
        // Store and replicate
        store.store(content.clone(), metadata.clone()).await.unwrap();
        let replica_info = replication.replicate_content(&hash, &content, metadata).await.unwrap();
        
        // Check replication factor is within bounds
        assert!(replica_info.replication_factor >= 5);
        assert!(replica_info.replication_factor <= 20);
        
        // Verify replication stats
        let stats = replication.get_stats().await;
        assert!(stats.total_replications > 0);
    }
    
    #[tokio::test]
    async fn test_parallel_retrieval_strategies() {
        let (_store, _replication, retrieval, _chunks, _temp) = create_test_storage_system().await;
        
        // Test content that doesn't exist locally
        let hash = ContentHash([99u8; 32]);
        
        // Try different strategies
        let strategies = vec![
            RetrievalStrategy::Kademlia,
            RetrievalStrategy::Hyperbolic,
            RetrievalStrategy::SOMBroadcast,
            RetrievalStrategy::Sequential,
            RetrievalStrategy::Parallel,
        ];
        
        for strategy in strategies {
            let _ = retrieval.retrieve(&hash, strategy.clone()).await;
        }
        
        // Check that all strategies were attempted
        let stats = retrieval.get_stats().await;
        assert_eq!(stats.total_retrievals, 5);
    }
    
    #[tokio::test]
    async fn test_cache_integration() {
        let (store, _replication, retrieval, _chunks, _temp) = create_test_storage_system().await;
        
        // Store content
        let content = b"Cacheable content".to_vec();
        let hash = ContentStore::calculate_hash(&content);
        let metadata = storage::ContentMetadata {
            size: content.len(),
            content_type: ContentType::DataRetrieval,
            created_at: std::time::Instant::now(),
            chunk_count: None,
            replication_factor: 8,
        };
        
        store.store(content.clone(), metadata).await.unwrap();
        
        // First retrieval (from local store)
        let _ = retrieval.retrieve(&hash, RetrievalStrategy::Parallel).await.unwrap();
        
        // Remove from store to force cache hit
        store.delete(&hash).await.unwrap();
        
        // Second retrieval should hit cache
        let retrieved = retrieval.retrieve(&hash, RetrievalStrategy::Parallel).await.unwrap();
        assert_eq!(retrieved, content);
        
        let stats = retrieval.get_stats().await;
        assert_eq!(stats.cache_hits, 1);
    }
    
    #[tokio::test]
    async fn test_churn_resilience() {
        let (_store, replication, _retrieval, _chunks, _temp) = create_test_storage_system().await;
        
        // Simulate content with replicas
        let content = b"Content at risk".to_vec();
        let hash = ContentStore::calculate_hash(&content);
        let departed_node = NodeId { hash: [1u8; 32] };
        
        // Manually add replica info
        let mut storing_nodes = std::collections::HashSet::new();
        storing_nodes.insert(departed_node.clone());
        storing_nodes.insert(NodeId { hash: [2u8; 32] });
        storing_nodes.insert(NodeId { hash: [3u8; 32] });
        
        let replica_info = ReplicaInfo {
            storing_nodes,
            replication_factor: 3,
            target_factor: 5,
            last_check: std::time::Instant::now(),
            metadata: storage::ContentMetadata {
                size: content.len(),
                content_type: ContentType::DataRetrieval,
                created_at: std::time::Instant::now(),
                chunk_count: None,
                replication_factor: 5,
            },
        };
        
        // Insert replica info
        {
            let mut map = replication.replica_map.write().await;
            map.insert(hash, replica_info);
        }
        
        // Handle node departure
        replication.handle_node_departure(&departed_node).await.unwrap();
        
        // Check that node was removed and replication maintained
        let map = replication.replica_map.read().await;
        let updated = map.get(&hash).unwrap();
        assert!(!updated.storing_nodes.contains(&departed_node));
    }
    
    #[tokio::test]
    async fn test_storage_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().to_str().unwrap().to_string();
        
        let content = b"Persistent content".to_vec();
        let hash = ContentStore::calculate_hash(&content);
        
        // Store content
        {
            let config = StorageConfig {
                db_path: db_path.clone(),
                ..Default::default()
            };
            let store = ContentStore::new(config).await.unwrap();
            
            let metadata = storage::ContentMetadata {
                size: content.len(),
                content_type: ContentType::DataRetrieval,
                created_at: std::time::Instant::now(),
                chunk_count: None,
                replication_factor: 8,
            };
            
            store.store(content.clone(), metadata).await.unwrap();
        }
        
        // Create new store instance and verify content persists
        {
            let config = StorageConfig {
                db_path,
                ..Default::default()
            };
            let store = ContentStore::new(config).await.unwrap();
            
            let retrieved = store.retrieve(&hash).await.unwrap();
            assert_eq!(retrieved, Some(content));
        }
    }
}