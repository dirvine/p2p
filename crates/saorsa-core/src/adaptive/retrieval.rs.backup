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

//! Parallel content retrieval system
//!
//! This module implements parallel retrieval strategies as specified:
//! - Kademlia lookup (α=3 parallel)
//! - Hyperbolic greedy routing
//! - SOM region broadcast
//! - First successful response wins

use super::*;
use crate::adaptive::{
    routing::AdaptiveRouter,
    RoutingStrategy,
    learning::QLearnCacheManager,
    storage::ContentStore,
    ContentType,
};
use anyhow::Result;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::{RwLock, mpsc},
    time::timeout,
};
use futures::future::select_all;

/// Retrieval manager for parallel content fetching
pub struct RetrievalManager {
    /// Routing system for finding content
    router: Arc<AdaptiveRouter>,
    
    /// Local content store
    content_store: Arc<ContentStore>,
    
    /// Cache manager for Q-learning decisions
    cache_manager: Arc<QLearnCacheManager>,
    
    /// Retrieval statistics
    stats: Arc<RwLock<RetrievalStats>>,
    
    /// Retrieval timeout
    timeout: Duration,
}

/// Strategy for content retrieval
#[derive(Debug, Clone, PartialEq)]
pub enum RetrievalStrategy {
    /// Use all available strategies in parallel
    Parallel,
    
    /// Kademlia DHT lookup
    Kademlia,
    
    /// Hyperbolic greedy routing
    Hyperbolic,
    
    /// SOM region broadcast
    SOMBroadcast,
    
    /// Sequential fallback (try strategies in order)
    Sequential,
}

/// Retrieval statistics
#[derive(Debug, Default, Clone)]
pub struct RetrievalStats {
    /// Total retrieval attempts
    pub total_retrievals: u64,
    
    /// Successful retrievals
    pub successful_retrievals: u64,
    
    /// Failed retrievals
    pub failed_retrievals: u64,
    
    /// Retrievals by strategy
    pub retrievals_by_strategy: HashMap<String, u64>,
    
    /// Success by strategy
    pub success_by_strategy: HashMap<String, u64>,
    
    /// Average retrieval time
    pub avg_retrieval_time_ms: f64,
    
    /// Cache hits during retrieval
    pub cache_hits: u64,
}

/// Result of a retrieval attempt
#[derive(Debug)]
struct RetrievalResult {
    /// Retrieved content
    content: Vec<u8>,
    
    /// Strategy that succeeded
    strategy: String,
    
    /// Time taken
    duration: Duration,
    
    /// Node that provided content
    source_node: NodeId,
}

impl RetrievalManager {
    /// Create a new retrieval manager
    pub fn new(
        router: Arc<AdaptiveRouter>,
        content_store: Arc<ContentStore>,
        cache_manager: Arc<QLearnCacheManager>,
    ) -> Self {
        Self {
            router,
            content_store,
            cache_manager,
            stats: Arc::new(RwLock::new(RetrievalStats::default())),
            timeout: Duration::from_secs(5),
        }
    }
    
    /// Retrieve content using specified strategy
    pub async fn retrieve(
        &self,
        content_hash: &ContentHash,
        strategy: RetrievalStrategy,
    ) -> Result<Vec<u8>> {
        let start_time = Instant::now();
        
        // Check local store first
        if let Some(content) = self.content_store.retrieve(content_hash).await? {
            self.update_stats_success("local", start_time.elapsed()).await;
            return Ok(content);
        }
        
        // Check cache
        if let Some(content) = self.cache_manager.get(content_hash).await {
            let mut stats = self.stats.write().await;
            stats.cache_hits += 1;
            drop(stats);
            self.update_stats_success("cache", start_time.elapsed()).await;
            return Ok(content);
        }
        
        // Perform retrieval based on strategy
        let result = match strategy {
            RetrievalStrategy::Parallel => {
                self.parallel_retrieve(content_hash).await
            }
            RetrievalStrategy::Kademlia => {
                self.kademlia_retrieve(content_hash).await
            }
            RetrievalStrategy::Hyperbolic => {
                self.hyperbolic_retrieve(content_hash).await
            }
            RetrievalStrategy::SOMBroadcast => {
                self.som_broadcast_retrieve(content_hash).await
            }
            RetrievalStrategy::Sequential => {
                self.sequential_retrieve(content_hash).await
            }
        };
        
        match result {
            Ok(retrieval_result) => {
                // Update statistics
                self.update_stats_success(&retrieval_result.strategy, retrieval_result.duration).await;
                
                // Cache the content based on Q-learning decision
                self.cache_manager.decide_caching(
                    content_hash.clone(),
                    retrieval_result.content.clone(),
                    ContentType::DataRetrieval,
                ).await?;
                
                // Update routing statistics
                self.router.update_statistics(
                    &retrieval_result.source_node,
                    true,
                    retrieval_result.duration.as_millis() as u64,
                ).await;
                
                Ok(retrieval_result.content)
            }
            Err(e) => {
                self.update_stats_failure().await;
                Err(e)
            }
        }
    }
    
    /// Parallel retrieval using all strategies
    async fn parallel_retrieve(&self, content_hash: &ContentHash) -> Result<RetrievalResult> {
        let (tx, mut rx) = mpsc::channel(3);
        
        // Launch all strategies in parallel
        let kademlia_handle = {
            let hash = content_hash.clone();
            let tx = tx.clone();
            let manager = self.clone_for_task();
            tokio::spawn(async move {
                if let Ok(result) = manager.kademlia_retrieve(&hash).await {
                    let _ = tx.send(result).await;
                }
            })
        };
        
        let hyperbolic_handle = {
            let hash = content_hash.clone();
            let tx = tx.clone();
            let manager = self.clone_for_task();
            tokio::spawn(async move {
                if let Ok(result) = manager.hyperbolic_retrieve(&hash).await {
                    let _ = tx.send(result).await;
                }
            })
        };
        
        let som_handle = {
            let hash = content_hash.clone();
            let tx = tx.clone();
            let manager = self.clone_for_task();
            tokio::spawn(async move {
                if let Ok(result) = manager.som_broadcast_retrieve(&hash).await {
                    let _ = tx.send(result).await;
                }
            })
        };
        
        // Drop original sender so channel closes when all tasks complete
        drop(tx);
        
        // Wait for first successful result
        match timeout(self.timeout, rx.recv()).await {
            Ok(Some(result)) => {
                // Cancel other tasks
                kademlia_handle.abort();
                hyperbolic_handle.abort();
                som_handle.abort();
                
                Ok(result)
            }
            Ok(None) => Err(anyhow::anyhow!("All retrieval strategies failed")),
            Err(_) => Err(anyhow::anyhow!("Retrieval timeout")),
        }
    }
    
    /// Kademlia-based retrieval
    async fn kademlia_retrieve(&self, content_hash: &ContentHash) -> Result<RetrievalResult> {
        let start_time = Instant::now();
        
        // Get Kademlia routing strategy
        let strategies = self.router.get_all_strategies();
        let kademlia_strategy = strategies.get("Kademlia")
            .ok_or_else(|| anyhow::anyhow!("Kademlia strategy not available"))?;
        
        // Find nodes storing this content (α=3 parallel as per spec)
        let nodes = kademlia_strategy.find_closest_nodes(content_hash, 3).await?;
        
        // Query nodes in parallel
        let mut futures = Vec::new();
        for node in nodes {
            let future = Box::pin(self.query_node_for_content(node, content_hash.clone()));
            futures.push(future);
        }
        
        // Wait for first successful response
        if !futures.is_empty() {
            let (result, _index, _remaining) = select_all(futures).await;
            if let Ok((content, source_node)) = result {
                return Ok(RetrievalResult {
                    content,
                    strategy: "Kademlia".to_string(),
                    duration: start_time.elapsed(),
                    source_node,
                });
            }
        }
        
        Err(anyhow::anyhow!("Kademlia retrieval failed"))
    }
    
    /// Hyperbolic greedy routing retrieval
    async fn hyperbolic_retrieve(&self, content_hash: &ContentHash) -> Result<RetrievalResult> {
        let start_time = Instant::now();
        
        // Get hyperbolic routing strategy
        let strategies = self.router.get_all_strategies();
        let hyperbolic_strategy = strategies.get("Hyperbolic")
            .ok_or_else(|| anyhow::anyhow!("Hyperbolic strategy not available"))?;
        
        // Find path to content using greedy routing
        let path = hyperbolic_strategy.find_path(&NodeId { hash: content_hash.0 }).await?;
        
        // Query nodes along the path
        for node in path {
            if let Ok((content, source_node)) = self.query_node_for_content(node.clone(), content_hash.clone()).await {
                return Ok(RetrievalResult {
                    content,
                    strategy: "Hyperbolic".to_string(),
                    duration: start_time.elapsed(),
                    source_node,
                });
            }
        }
        
        Err(anyhow::anyhow!("Hyperbolic retrieval failed"))
    }
    
    /// SOM broadcast retrieval
    async fn som_broadcast_retrieve(&self, content_hash: &ContentHash) -> Result<RetrievalResult> {
        let start_time = Instant::now();
        
        // Get SOM routing strategy
        let strategies = self.router.get_all_strategies();
        let som_strategy = strategies.get("SOM")
            .ok_or_else(|| anyhow::anyhow!("SOM strategy not available"))?;
        
        // Find nodes in the content's SOM region
        let nodes = som_strategy.find_closest_nodes(content_hash, 10).await?;
        
        // Broadcast to all nodes in parallel
        let mut futures = Vec::new();
        for node in nodes {
            let future = Box::pin(self.query_node_for_content(node, content_hash.clone()));
            futures.push(future);
        }
        
        // Wait for first successful response
        if !futures.is_empty() {
            let (result, _index, _remaining) = select_all(futures).await;
            if let Ok((content, source_node)) = result {
                return Ok(RetrievalResult {
                    content,
                    strategy: "SOM".to_string(),
                    duration: start_time.elapsed(),
                    source_node,
                });
            }
        }
        
        Err(anyhow::anyhow!("SOM broadcast retrieval failed"))
    }
    
    /// Sequential retrieval (fallback mode)
    async fn sequential_retrieve(&self, content_hash: &ContentHash) -> Result<RetrievalResult> {
        // Try strategies in order
        if let Ok(result) = self.kademlia_retrieve(content_hash).await {
            return Ok(result);
        }
        
        if let Ok(result) = self.hyperbolic_retrieve(content_hash).await {
            return Ok(result);
        }
        
        if let Ok(result) = self.som_broadcast_retrieve(content_hash).await {
            return Ok(result);
        }
        
        Err(anyhow::anyhow!("All retrieval strategies failed"))
    }
    
    /// Query a specific node for content
    async fn query_node_for_content(
        &self,
        node: NodeId,
        content_hash: ContentHash,
    ) -> Result<(Vec<u8>, NodeId)> {
        // In real implementation, this would:
        // 1. Send GET_CONTENT message to node
        // 2. Wait for response
        // 3. Verify content hash matches
        // 4. Return content
        
        // For now, simulate with random success
        if rand::random::<f64>() > 0.7 {
            // Simulate content retrieval
            let content = format!("Content for hash {content_hash:?}").into_bytes();
            Ok((content, node))
        } else {
            Err(anyhow::anyhow!("Node does not have content"))
        }
    }
    
    /// Update statistics for successful retrieval
    async fn update_stats_success(&self, strategy: &str, duration: Duration) {
        let mut stats = self.stats.write().await;
        stats.total_retrievals += 1;
        stats.successful_retrievals += 1;
        
        *stats.retrievals_by_strategy.entry(strategy.to_string()).or_insert(0) += 1;
        *stats.success_by_strategy.entry(strategy.to_string()).or_insert(0) += 1;
        
        // Update average retrieval time
        let current_avg = stats.avg_retrieval_time_ms;
        let current_count = stats.successful_retrievals as f64;
        stats.avg_retrieval_time_ms = 
            (current_avg * (current_count - 1.0) + duration.as_millis() as f64) / current_count;
    }
    
    /// Update statistics for failed retrieval
    async fn update_stats_failure(&self) {
        let mut stats = self.stats.write().await;
        stats.total_retrievals += 1;
        stats.failed_retrievals += 1;
    }
    
    /// Clone manager for spawning tasks
    fn clone_for_task(&self) -> Self {
        Self {
            router: self.router.clone(),
            content_store: self.content_store.clone(),
            cache_manager: self.cache_manager.clone(),
            stats: self.stats.clone(),
            timeout: self.timeout,
        }
    }
    
    /// Get retrieval statistics
    pub async fn get_stats(&self) -> RetrievalStats {
        self.stats.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adaptive::{
        trust::MockTrustProvider,
        hyperbolic::HyperbolicSpace,
        som::SelfOrganizingMap,
    };
    use tempfile::TempDir;
    
    async fn create_test_retrieval_manager() -> RetrievalManager {
        let trust_provider = Arc::new(MockTrustProvider::new());
        let hyperbolic = Arc::new(HyperbolicSpace::new());
        let som = Arc::new(SelfOrganizingMap::new(10, 10, 4));
        let router = Arc::new(AdaptiveRouter::new(trust_provider, hyperbolic, som));
        
        let temp_dir = TempDir::new().unwrap();
        let storage_config = StorageConfig {
            db_path: temp_dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };
        let content_store = Arc::new(ContentStore::new(storage_config).await.unwrap());
        let cache_manager = Arc::new(QLearnCacheManager::new(1024 * 1024));
        
        RetrievalManager::new(router, content_store, cache_manager)
    }
    
    #[tokio::test]
    async fn test_local_retrieval() {
        let manager = create_test_retrieval_manager().await;
        let content = b"Test content".to_vec();
        let hash = ContentStore::calculate_hash(&content);
        
        // Store content locally
        let metadata = crate::adaptive::storage::ContentMetadata {
            size: content.len(),
            content_type: ContentType::DataRetrieval,
            created_at: Instant::now(),
            chunk_count: None,
            replication_factor: 8,
        };
        manager.content_store.store(content.clone(), metadata).await.unwrap();
        
        // Retrieve should find it locally
        let retrieved = manager.retrieve(&hash, RetrievalStrategy::Parallel).await.unwrap();
        assert_eq!(retrieved, content);
        
        // Check stats
        let stats = manager.get_stats().await;
        assert_eq!(stats.successful_retrievals, 1);
        assert_eq!(stats.success_by_strategy.get("local"), Some(&1));
    }
    
    #[tokio::test]
    async fn test_cache_retrieval() {
        let manager = create_test_retrieval_manager().await;
        let content = b"Cached content".to_vec();
        let hash = ContentStore::calculate_hash(&content);
        
        // Add to cache
        manager.cache_manager.insert(hash.clone(), content.clone()).await;
        
        // Retrieve should find it in cache
        let retrieved = manager.retrieve(&hash, RetrievalStrategy::Parallel).await.unwrap();
        assert_eq!(retrieved, content);
        
        // Check stats
        let stats = manager.get_stats().await;
        assert_eq!(stats.cache_hits, 1);
    }
    
    #[tokio::test]
    async fn test_parallel_strategy() {
        let manager = create_test_retrieval_manager().await;
        let hash = ContentHash([42u8; 32]);
        
        // Try parallel retrieval (will fail in test environment)
        let result = manager.retrieve(&hash, RetrievalStrategy::Parallel).await;
        
        // In test environment, this will likely fail
        assert!(result.is_err());
        
        // Check that attempts were made
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_retrievals, 1);
        assert_eq!(stats.failed_retrievals, 1);
    }
    
    #[tokio::test]
    async fn test_strategy_selection() {
        let manager = create_test_retrieval_manager().await;
        let hash = ContentHash([42u8; 32]);
        
        // Test different strategies
        for strategy in [
            RetrievalStrategy::Kademlia,
            RetrievalStrategy::Hyperbolic,
            RetrievalStrategy::SOMBroadcast,
            RetrievalStrategy::Sequential,
        ] {
            let _ = manager.retrieve(&hash, strategy.clone()).await;
        }
        
        // Check that all strategies were attempted
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_retrievals, 4);
    }
}