#!/bin/bash

echo "🔧 Carefully fixing ant-quic v0.6.1 Debug trait issues..."

cd /Users/davidirvine/Desktop/Devel/projects/ant-quic

# Let's examine which structs actually need Debug traits
echo "🔍 Finding structs missing Debug trait..."

# Add Debug to specific structs that need it
echo "Adding Debug to MetricsCollector..."
sed -i '' 's/^pub struct MetricsCollector/#[derive(Debug)]\npub struct MetricsCollector/' src/logging/metrics.rs

echo "Adding Debug to ThroughputTracker..."
sed -i '' 's/^pub struct ThroughputTracker/#[derive(Debug)]\npub struct ThroughputTracker/' src/logging/metrics.rs

echo "Adding Debug to LatencyTracker..."
sed -i '' 's/^pub struct LatencyTracker/#[derive(Debug)]\npub struct LatencyTracker/' src/logging/metrics.rs

echo "Adding Debug to ConnectionMetrics..."
sed -i '' 's/^pub struct ConnectionMetrics/#[derive(Debug)]\npub struct ConnectionMetrics/' src/logging/metrics.rs

echo "✅ Debug traits added carefully!"

# Test compilation
echo "🧪 Testing compilation..."
cargo check 2>&1 | head -10