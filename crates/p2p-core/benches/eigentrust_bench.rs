use criterion::{black_box, criterion_group, criterion_main, Criterion};
use p2p_core::adaptive::trust::{EigenTrustEngine, NodeStatisticsUpdate};
use p2p_core::adaptive::NodeId;
use std::collections::HashSet;

fn bench_eigentrust_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("eigentrust_computation");
    
    for num_nodes in [10, 50, 100] {
        group.bench_function(format!("nodes_{}", num_nodes), |b| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            
            b.iter(|| {
                runtime.block_on(async {
                    let pre_trusted = create_pre_trusted_nodes(2);
                    let engine = EigenTrustEngine::new(pre_trusted);
                    
                    // Add interactions
                    for i in 0..num_nodes {
                        for j in 0..5 {
                            let from = create_node_id(i);
                            let to = create_node_id((i + j + 1) % num_nodes);
                            let success = (i + j) % 3 != 0;
                            engine.update_local_trust(&from, &to, success).await;
                        }
                    }
                    
                    black_box(engine.compute_global_trust().await);
                });
            });
        });
    }
    
    group.finish();
}

fn bench_trust_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("eigentrust_updates");
    
    group.bench_function("local_trust_update", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let engine = EigenTrustEngine::new(HashSet::new());
        let node1 = create_node_id(1);
        let node2 = create_node_id(2);
        
        b.iter(|| {
            runtime.block_on(async {
                black_box(engine.update_local_trust(&node1, &node2, true).await);
            });
        });
    });
    
    group.bench_function("node_stats_update", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let engine = EigenTrustEngine::new(HashSet::new());
        let node = create_node_id(1);
        
        b.iter(|| {
            runtime.block_on(async {
                black_box(engine.update_node_stats(
                    &node, 
                    NodeStatisticsUpdate::CorrectResponse
                ).await);
            });
        });
    });
    
    group.finish();
}

fn create_node_id(index: usize) -> NodeId {
    use rand::RngCore;
    let mut hash = [0u8; 32];
    hash[0] = index as u8;
    rand::thread_rng().fill_bytes(&mut hash[1..]);
    NodeId::from_bytes(hash)
}

fn create_pre_trusted_nodes(count: usize) -> HashSet<NodeId> {
    (0..count).map(create_node_id).collect()
}

criterion_group!(
    benches,
    bench_eigentrust_computation,
    bench_trust_updates
);

criterion_main!(benches);