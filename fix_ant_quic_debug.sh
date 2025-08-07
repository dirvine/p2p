#!/bin/bash

# Fix Debug trait issues in ant-quic v0.6.1

cd /Users/davidirvine/Desktop/Devel/projects/ant-quic

echo "🔧 Fixing Debug trait issues in ant-quic v0.6.1..."

# Fix structs that definitely need Debug
echo "Adding Debug to NatTraversalStatistics..."
sed -i '' 's/pub struct NatTraversalStatistics {/#[derive(Debug, Clone)]\npub struct NatTraversalStatistics {/' src/nat_traversal_api.rs

echo "Adding Debug to NetworkOptimizationMetrics..."
sed -i '' 's/pub struct NetworkOptimizationMetrics {/#[derive(Debug, Clone)]\npub struct NetworkOptimizationMetrics {/' src/optimization/network.rs

echo "Adding Debug to MemoryOptimizationStats..."
sed -i '' 's/pub struct MemoryOptimizationStats {/#[derive(Debug, Clone)]\npub struct MemoryOptimizationStats {/' src/optimization/memory.rs

echo "Adding Debug to StructuredEventBuilder..."
sed -i '' 's/pub struct StructuredEventBuilder {/#[derive(Debug)]\npub struct StructuredEventBuilder {/' src/logging/structured.rs

echo "Adding Debug to ZeroRttAccepted..."
sed -i '' 's/pub struct ZeroRttAccepted/#[derive(Debug)]\npub struct ZeroRttAccepted/' src/high_level/connection.rs

echo "Adding Debug to ConnectionStats..."
sed -i '' 's/pub struct ConnectionStats {/#[derive(Debug, Clone)]\npub struct ConnectionStats {/' src/connection/stats.rs

echo "Adding Debug to EndpointStats..."
sed -i '' 's/pub struct EndpointStats {/#[derive(Debug, Clone)]\npub struct EndpointStats {/' src/endpoint.rs

# Fix atomic types in stats
echo "Adding Debug to structs with AtomicU64 fields..."
find src -name "*.rs" -exec grep -l "AtomicU64" {} \; | while read file; do
    # Check if the file has structs without Debug that contain AtomicU64
    if grep -q "pub struct.*{" "$file" && ! grep -B1 "pub struct" "$file" | grep -q "#\[derive.*Debug"; then
        echo "  Fixing $file..."
        # Add Debug derive to structs that don't have it
        sed -i '' '/pub struct.*{/i\
#[derive(Debug)]' "$file"
    fi
done

echo "✅ Debug trait fixes applied!"
echo ""
echo "Now testing compilation..."
cargo check --lib 2>&1 | grep -E "error\[E" | head -5