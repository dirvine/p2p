# Self-Organizing Map (SOM) Implementation

Date: July 26, 2025

## Overview

The P2P Foundation includes a comprehensive Self-Organizing Map implementation that provides dynamic content and capability clustering. The SOM enables efficient content discovery and capability-based routing by clustering nodes with similar characteristics.

## Implementation Status

### ✅ Core Features Already Implemented

The existing implementation in `crates/p2p-core/src/adaptive/som.rs` includes:

1. **2D SOM Grid Structure**
   - Configurable grid dimensions (width × height)
   - Dynamic sizing based on network size
   - Weight vectors for each grid node
   - Node assignment tracking

2. **4D Feature Space**
   - **Content Affinity**: Historical content type preferences
   - **Storage Capacity**: Normalized storage capabilities
   - **Compute Capability**: Normalized computational power
   - **Network Quality**: Bandwidth × trust score

3. **Learning Algorithm**
   - Competitive learning with neighborhood function
   - Adaptive learning rate (exponential decay)
   - Adaptive neighborhood radius
   - Online update mode for real-time adaptation

4. **Content Mapping**
   - Maps nodes to SOM regions based on features
   - Content type-specific node discovery
   - Similarity-based clustering
   - Dynamic remapping as patterns change

5. **Integration with Routing**
   - `SOMRoutingStrategy` implements `RoutingStrategy` trait
   - Feature similarity-based path finding
   - Integration with adaptive router

## Key Components

### Core Types

```rust
/// Main SOM structure
pub struct SelfOrganizingMap {
    map: Vec<Vec<SOMNode>>,              // 2D grid
    feature_dim: usize,                  // Fixed at 4
    learning_rate: f64,                  // Initial: 0.1
    neighborhood_radius: f64,            // Initial: 3.0
    iteration: u64,                      // Training counter
    extractor: Arc<RwLock<FeatureExtractor>>,
    feature_cache: Arc<RwLock<HashMap<NodeId, [f64; 4]>>>,
}

/// Feature extraction
pub struct FeatureExtractor {
    content_history: HashMap<NodeId, HashMap<ContentType, u64>>,
    max_storage: f64,     // For normalization
    max_compute: f64,     // For normalization
    max_bandwidth: f64,   // For normalization
}

/// SOM-based routing
pub struct SOMRoutingStrategy {
    som: Arc<RwLock<SelfOrganizingMap>>,
    local_id: NodeId,
}
```

### Feature Extraction

The SOM uses a 4-dimensional feature space:

1. **Content Affinity** (0.0-1.0)
   - Based on historical content type distribution
   - Dominant content type ratio
   - Default: 0.5 for new nodes

2. **Storage Capacity** (0.0-1.0)
   - Normalized by maximum seen storage
   - Represents data storage capability

3. **Compute Capability** (0.0-1.0)
   - Normalized computational resources
   - For compute-intensive tasks

4. **Network Quality** (0.0-1.0)
   - Bandwidth × trust score
   - Represents connectivity reliability

### Learning Process

The SOM updates using standard Kohonen learning:

1. **Find Best Matching Unit (BMU)**
   - Euclidean distance in feature space
   - O(n×m) for n×m grid

2. **Update Weights**
   - BMU and neighbors updated
   - Gaussian neighborhood function
   - Learning rate: `0.1 × exp(-iteration/1000)`
   - Radius: `3.0 × exp(-iteration/500)`

3. **Node Assignment**
   - Nodes assigned to their BMU
   - Multiple nodes can share a grid position

## Content Type Mapping

The implementation maps content types to feature patterns:

```rust
match content_type {
    ContentType::DHTLookup => [0.8, 0.2, 0.5, 0.9],      // High affinity, high network
    ContentType::DataRetrieval => [0.7, 0.9, 0.3, 0.8],  // High storage, high network
    ContentType::ComputeRequest => [0.5, 0.3, 0.9, 0.7], // High compute
    ContentType::RealtimeMessage => [0.6, 0.1, 0.4, 1.0], // Highest network quality
}
```

## API Usage

### Creating and Training

```rust
// Create SOM with fixed dimensions
let mut som = SelfOrganizingMap::new(10, 10);

// Or create with dynamic sizing
let mut som = SelfOrganizingMap::new_dynamic(expected_nodes);

// Update with node descriptor
som.update_node(&node_descriptor).await;

// Update content history
som.update_content_history(&node_id, ContentType::DataRetrieval).await;
```

### Finding Similar Nodes

```rust
// Find nodes in same SOM region
let similar = som.find_similar_nodes(&node_id);

// Find nodes best suited for content type
let storage_nodes = som.find_nodes_for_content(
    ContentType::DataRetrieval, 
    5  // count
).await;
```

### Routing Integration

```rust
// Create SOM-based routing strategy
let strategy = SOMRoutingStrategy::new(som, local_id);

// Use with adaptive router
let path = strategy.find_path(&target_id).await?;
```

## Testing

The implementation includes comprehensive tests:

1. **Grid Creation** - Verifies proper initialization
2. **Dynamic Sizing** - Tests size calculation formula
3. **Node Updates** - Validates feature extraction and assignment
4. **Learning Rate Decay** - Ensures proper decay over time
5. **Feature Extraction** - Tests normalization and ranges
6. **Content Discovery** - Validates content-based node finding
7. **Routing Strategy** - Tests path finding

## Integration Points

### 1. With Adaptive Router
- Plugs in as a routing strategy
- Complements DHT and hyperbolic routing
- Content-aware routing decisions

### 2. With Node Descriptors
- Extracts features from capabilities
- Tracks content history
- Updates dynamically

### 3. With Content Types
- Maps content to optimal nodes
- Enables semantic routing
- Improves content discovery

## Performance Characteristics

1. **Update Complexity**: O(n×m) for n×m grid
2. **BMU Search**: O(n×m) 
3. **Neighbor Update**: O(r²) for radius r
4. **Memory Usage**: O(n×m×d) for d features
5. **Feature Extraction**: O(1)

## Task 5 Completion Summary

Task 5 (Self-Organizing Map Implementation) is effectively complete as the implementation already exists and is comprehensive. The implementation includes:

1. ✅ Full SOM algorithm with competitive learning
2. ✅ 4D feature space for node characteristics
3. ✅ Content type mapping and discovery
4. ✅ Dynamic grid sizing
5. ✅ Integration with routing system
6. ✅ Comprehensive test coverage

The SOM provides efficient content-based clustering and discovery, enabling semantic routing in the P2P network.

## Future Enhancement Opportunities

1. **Hexagonal Grid Topology**
   - More uniform neighbor distances
   - Better coverage of feature space

2. **Hierarchical SOM**
   - Multi-resolution clustering
   - Faster lookups for large networks

3. **Online Batch Updates**
   - Process multiple updates efficiently
   - Better convergence properties

4. **Visualization Tools**
   - Real-time SOM state display
   - Feature space exploration

5. **Persistent State**
   - Save/restore trained SOM
   - Faster network rejoin

## Conclusion

The Self-Organizing Map implementation provides sophisticated content and capability clustering for the P2P Foundation. It enables efficient content discovery and semantic routing by grouping nodes with similar characteristics, complementing the DHT and geometric routing layers.