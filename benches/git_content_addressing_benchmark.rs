
#!/usr/bin/env rust
//! Performance Benchmarks for Git-Like Content Addressing
//!
//! This benchmark suite measures the performance of git-like content addressing
//! operations to ensure they meet production requirements.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Instant;
use std::sync::Arc;

// Import the git content addressing types (these would be actual imports in real code)
use saorsa_core::{
    ContentHash, ObjectType, GitObject, 
    BlobObject, TreeObject, CommitObject,
    CommitAuthor, CommitType,
    GitDhtStorage, GitApplicationLayer,
    DocumentFormat,
};

/// Benchmark configuration
struct BenchmarkConfig {
    small_blob_size: usize,
    medium_blob_size: usize,
    large_blob_size: usize,
    tree_entry_count: usize,
    commit_chain_length: usize,
    concurrent_operations: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            small_blob_size: 1024,           // 1KB
            medium_blob_size: 1024 * 1024,   // 1MB
            large_blob_size: 10 * 1024 * 1024, // 10MB
            tree_entry_count: 100,
            commit_chain_length: 50,
            concurrent_operations: 10,
        }
    }
}

fn create_test_app_layer() -> GitApplicationLayer {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mock_dht = Arc::new(MockDhtStorage::new("benchmark_peer".to_string()));
        let storage = GitDhtStorage::new(mock_dht, 10000, "benchmark_peer".to_string());
        GitApplicationLayer::new(storage)
    })
}

/// Benchmark content hash operations
fn benchmark_content_hashing(c: &mut Criterion) {
    let config = BenchmarkConfig::default();
    
    let mut group = c.benchmark_group("Content Hashing");
    
    // Benchmark different blob sizes
    for size in [config.small_blob_size, config.medium_blob_size].iter() {
        let data = vec![0x42u8; *size];
        
        group.bench_with_input(
            BenchmarkId::new("blake3_hash", size),
            size,
            |b, _| {
                b.iter(|| {
                    ContentHash::from_content(black_box(&data))
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("typed_hash", size),
            size,
            |b, _| {
                b.iter(|| {
                    ContentHash::from_typed_content(black_box(ObjectType::Blob), black_box(&data))
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark blob object operations
fn benchmark_blob_operations(c: &mut Criterion) {
    let config = BenchmarkConfig::default();
    
    let mut group = c.benchmark_group("Blob Operations");
    
    for size in [config.small_blob_size, config.medium_blob_size].iter() {
        let data = vec![0x42u8; *size];
        
        group.bench_with_input(
            BenchmarkId::new("blob_creation", size),
            size,
            |b, _| {
                b.iter(|| {
                    let blob = BlobObject::new(black_box(data.clone()));
                    black_box(blob)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("blob_serialization", size),
            size,
            |b, _| {
                let blob = BlobObject::new(data.clone());
                b.iter(|| {
                    let serialized = bincode::serialize(black_box(&blob)).unwrap();
                    black_box(serialized)
                });
            },
        );
        
        let blob = BlobObject::new(data.clone());
        let serialized = bincode::serialize(&blob).unwrap();
        
        group.bench_with_input(
            BenchmarkId::new("blob_deserialization", size),
            size,
            |b, _| {
                b.iter(|| {
                    let deserialized: BlobObject = bincode::deserialize(black_box(&serialized)).unwrap();
                    black_box(deserialized)
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark tree object operations
fn benchmark_tree_operations(c: &mut Criterion) {
    let config = BenchmarkConfig::default();
    
    let mut group = c.benchmark_group("Tree Operations");
    
    // Create tree with varying numbers of entries
    for entry_count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("tree_creation", entry_count),
            entry_count,
            |b, &count| {
                b.iter(|| {
                    let mut tree = TreeObject::new();
                    for i in 0..count {
                        let hash = ContentHash::from_content(format!("file_{}", i).as_bytes());
                        tree.add_blob(format!("file_{}.txt", i), hash, 100);
                    }
                    black_box(tree)
                });
            },
        );
        
        // Benchmark tree traversal
        let mut tree = TreeObject::new();
        for i in 0..*entry_count {
            let hash = ContentHash::from_content(format!("file_{}", i).as_bytes());
            tree.add_blob(format!("file_{}.txt", i), hash, 100);
        }
        
        group.bench_with_input(
            BenchmarkId::new("tree_lookup", entry_count),
            entry_count,
            |b, &count| {
                b.iter(|| {
                    let target = format!("file_{}.txt", count / 2);
                    let found = tree.find_entry(black_box(&target));
                    black_box(found)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("tree_serialization", entry_count),
            entry_count,
            |b, _| {
                b.iter(|| {
                    let serialized = bincode::serialize(black_box(&tree)).unwrap();
                    black_box(serialized)
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark commit operations
fn benchmark_commit_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Commit Operations");
    
    let tree_hash = ContentHash::from_content(b"tree content");
    let author = CommitAuthor {
        peer_id: "benchmark_peer".to_string(),
        name: "Benchmark User".to_string(),
        email: None,
        timestamp: std::time::SystemTime::now(),
    };
    
    // Benchmark commit creation
    group.bench_function("commit_creation", |b| {
        b.iter(|| {
            let commit = CommitObject::new(
                black_box(tree_hash.clone()),
                black_box(vec![]),
                black_box("Benchmark commit".to_string()),
                black_box(author.clone()),
                black_box("benchmark_app".to_string()),
                black_box("benchmark_repo".to_string()),
                black_box(CommitType::DocumentCreated),
            );
            black_box(commit)
        });
    });
    
    // Benchmark commit serialization
    let commit = CommitObject::new(
        tree_hash,
        vec![],
        "Benchmark commit".to_string(),
        author,
        "benchmark_app".to_string(),
        "benchmark_repo".to_string(),
        CommitType::DocumentCreated,
    );
    
    group.bench_function("commit_serialization", |b| {
        b.iter(|| {
            let serialized = bincode::serialize(black_box(&commit)).unwrap();
            black_box(serialized)
        });
    });
    
    group.finish();
}

/// Benchmark application-layer operations
fn benchmark_application_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("Application Operations");
    
    // Set longer measurement time for async operations
    group.measurement_time(std::time::Duration::from_secs(20));
    
    group.bench_function("chat_message_send", |b| {
        let app_layer = create_test_app_layer();
        b.to_async(&rt).iter(|| async {
            let result = app_layer.send_chat_message(
                black_box("benchmark_channel"),
                black_box("Benchmark message"),
                black_box("user".to_string()),
                black_box("User".to_string()),
                black_box(None),
                black_box(vec![]),
            ).await;
            black_box(result)
        });
    });
    
    group.bench_function("document_creation", |b| {
        let app_layer = create_test_app_layer();
        b.to_async(&rt).iter(|| async {
            let result = app_layer.create_document(
                black_box("benchmark_doc"),
                black_box("Benchmark Document"),
                black_box("This is benchmark content for testing performance."),
                black_box(DocumentFormat::PlainText),
                black_box("user".to_string()),
            ).await;
            black_box(result)
        });
    });
    
    group.bench_function("forum_post_creation", |b| {
        let app_layer = create_test_app_layer();
        b.to_async(&rt).iter(|| async {
            let result = app_layer.create_forum_post(
                black_box("benchmark_topic"),
                black_box("Benchmark Post"),
                black_box("This is a benchmark forum post for performance testing."),
                black_box("user".to_string()),
                black_box("User".to_string()),
                black_box(vec!["benchmark".to_string()]),
            ).await;
            black_box(result)
        });
    });
    
    group.finish();
}

/// Benchmark storage operations
fn benchmark_storage_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("Storage Operations");
    
    group.measurement_time(std::time::Duration::from_secs(15));
    
    let mock_dht = Arc::new(MockDhtStorage::new("benchmark_peer".to_string()));
    let storage = rt.block_on(async {
        GitDhtStorage::new(mock_dht, 10000, "benchmark_peer".to_string())
    });
    
    // Benchmark object storage
    group.bench_function("object_storage", |b| {
        b.to_async(&rt).iter(|| async {
            let content = b"benchmark object content".to_vec();
            let obj = GitObject::new(
                ObjectType::Blob,
                content,
                saorsa_core::DataAccessLevel::Public {
                    signature: Default::default(),
                    content_hash: [0u8; 32],
                },
                "benchmark_peer".to_string(),
                None,
            );
            let result = storage.store_object(black_box(obj)).await;
            black_box(result)
        });
    });
    
    // Benchmark object retrieval
    let test_obj = rt.block_on(async {
        let content = b"test retrieval content".to_vec();
        let obj = GitObject::new(
            ObjectType::Blob,
            content,
            saorsa_core::DataAccessLevel::Public {
                signature: Default::default(),
                content_hash: [0u8; 32],
            },
            "benchmark_peer".to_string(),
            None,
        );
        let hash = storage.store_object(obj).await.unwrap();
        hash
    });
    
    group.bench_function("object_retrieval", |b| {
        b.to_async(&rt).iter(|| async {
            let result = storage.get_object(black_box(&test_obj)).await;
            black_box(result)
        });
    });
    
    group.finish();
}

/// Benchmark throughput with multiple operations
fn benchmark_throughput(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("Throughput");
    
    group.measurement_time(std::time::Duration::from_secs(30));
    
    // Benchmark message throughput
    for msg_count in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("chat_messages", msg_count),
            msg_count,
            |b, &count| {
                let app_layer = create_test_app_layer();
                b.to_async(&rt).iter(|| async {
                    let start = Instant::now();
                    
                    for i in 0..count {
                        let result = app_layer.send_chat_message(
                            "throughput_test",
                            &format!("Message {}", i),
                            "user".to_string(),
                            "User".to_string(),
                            None,
                            vec![],
                        ).await;
                        black_box(result);
                    }
                    
                    let duration = start.elapsed();
                    black_box(duration)
                });
            },
        );
    }
    
    group.finish();
}

/// Mock DHT storage for benchmarking
pub struct MockDhtStorage {
    local_peer_id: String,
}

impl MockDhtStorage {
    pub fn new(local_peer_id: String) -> Self {
        Self { local_peer_id }
    }
}

impl saorsa_core::DhtStorageProvider for MockDhtStorage {
    async fn store_secure_record(&self, _record: saorsa_core::EnhancedDhtRecord) -> saorsa_core::GitResult<()> {
        // Simulate minimal storage latency
        tokio::time::sleep(tokio::time::Duration::from_micros(10)).await;
        Ok(())
    }
    
    async fn get_secure_record_with_k_consistency(
        &self,
        _key: &saorsa_core::Key,
        _requester: &str,
        _context: &saorsa_core::AccessContext,
    ) -> saorsa_core::GitResult<Option<saorsa_core::EnhancedDhtRecord>> {
        // Simulate minimal retrieval latency
        tokio::time::sleep(tokio::time::Duration::from_micros(5)).await;
        Ok(None) // Return None for benchmarking (no actual data needed)
    }
    
    fn local_id(&self) -> &str {
        &self.local_peer_id
    }
}

// Define benchmark groups
criterion_group!(
    git_content_benches,
    benchmark_content_hashing,
    benchmark_blob_operations,
    benchmark_tree_operations,
    benchmark_commit_operations,
    benchmark_application_operations,
    benchmark_storage_operations,
    benchmark_throughput
);

criterion_main!(git_content_benches);

#[cfg(test)]
mod benchmark_tests {
    use super::*;
    
    #[test]
    fn test_benchmark_setup() {
        let config = BenchmarkConfig::default();
        assert!(config.small_blob_size > 0);
        assert!(config.medium_blob_size > config.small_blob_size);
        assert!(config.large_blob_size > config.medium_blob_size);
    }
    
    #[tokio::test]
    async fn test_mock_storage_performance() {
        let storage = MockDhtStorage::new("test".to_string());
        
        let start = Instant::now();
        
        // Test multiple operations
        for _ in 0..100 {
            let _ = storage.store_secure_record(
                saorsa_core::EnhancedDhtRecord::default()
            ).await;
        }
        
        let duration = start.elapsed();
        
        // Should complete 100 operations in reasonable time (< 10ms)
        assert!(duration < std::time::Duration::from_millis(10));
    }
}