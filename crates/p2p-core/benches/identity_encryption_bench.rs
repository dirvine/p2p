use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use saorsa_core::identity_manager::{IdentityManager, IdentityCreationParams, SecurityLevel};
use saorsa_core::secure_memory::SecureString;
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn encryption_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    
    // Setup identity manager
    let manager = rt.block_on(async {
        let mgr = IdentityManager::new(temp_dir.path(), SecurityLevel::High)
            .await
            .unwrap();
        let password = SecureString::from_str("test_password").unwrap();
        mgr.initialize(&password).await.unwrap();
        
        // Create test identity
        let params = IdentityCreationParams {
            display_name: Some("Benchmark User".to_string()),
            bio: Some("Test bio for benchmarking".to_string()),
            derivation_path: None,
            recovery_threshold: None,
        };
        let identity = mgr.create_identity(&password, params).await.unwrap();
        
        (mgr, identity.id, password)
    });
    
    let (mgr, identity_id, password) = manager;
    let device_password = SecureString::from_str("device_password").unwrap();
    
    // Benchmark sync package creation (includes encryption)
    c.bench_function("identity_encryption_sync_package", |b| {
        b.iter(|| {
            rt.block_on(async {
                mgr.create_sync_package(
                    black_box(&identity_id),
                    black_box(&device_password)
                ).await.unwrap()
            })
        })
    });
    
    // Create a sync package for import benchmark
    let sync_package = rt.block_on(async {
        mgr.create_sync_package(&identity_id, &device_password).await.unwrap()
    });
    
    // Benchmark sync package import (includes decryption)
    c.bench_function("identity_decryption_sync_package", |b| {
        b.iter(|| {
            rt.block_on(async {
                let temp_dir2 = TempDir::new().unwrap();
                let mgr2 = IdentityManager::new(temp_dir2.path(), SecurityLevel::High)
                    .await
                    .unwrap();
                mgr2.initialize(&password).await.unwrap();
                
                mgr2.import_sync_package(
                    black_box(&sync_package),
                    black_box(&device_password),
                    black_box(&password)
                ).await.unwrap()
            })
        })
    });
    
    // Benchmark just the encryption operation
    let mgr_ref = &mgr;
    c.bench_function("chacha20_encryption_only", |b| {
        let plaintext = b"Test data for encryption benchmark - 64 bytes of data here!!!";
        let salt = b"benchmark_salt_32_bytes_test_123";
        let nonce = [0u8; 12];
        
        b.iter(|| {
            rt.block_on(async {
                let key = mgr_ref.derive_encryption_key(&device_password, salt).unwrap();
                mgr_ref.encrypt_data(black_box(plaintext), &key, &nonce).unwrap()
            })
        })
    });
    
    // Benchmark different data sizes
    let mut group = c.benchmark_group("encryption_by_size");
    for size in [64, 256, 1024, 4096, 16384] {
        let data = vec![0u8; size];
        let salt = b"benchmark_salt_32_bytes_test_123";
        let nonce = [0u8; 12];
        
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                rt.block_on(async {
                    let key = mgr_ref.derive_encryption_key(&device_password, salt).unwrap();
                    mgr_ref.encrypt_data(black_box(&data), &key, &nonce).unwrap()
                })
            })
        });
    }
    group.finish();
}

criterion_group!(benches, encryption_benchmark);
criterion_main!(benches);