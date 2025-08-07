#!/bin/bash

echo "🔧 Comprehensive fix for ant-quic v0.6.1 compilation issues..."

cd /Users/davidirvine/Desktop/Devel/projects/ant-quic

# Add Debug derives to structs that need them (avoiding duplicates)
echo "🔍 Adding missing Debug traits..."

# Add Debug to MetricsCollector
sed -i '' '/pub struct MetricsCollector {/i\
#[derive(Debug)]
' src/logging/metrics.rs

# Add Debug to ThroughputTracker
sed -i '' '/pub struct ThroughputTracker {/i\
#[derive(Debug)]
' src/logging/metrics.rs

# Add Debug to LatencyTracker
sed -i '' '/pub struct LatencyTracker {/i\
#[derive(Debug)]
' src/logging/metrics.rs

# Add Debug to ConnectionMetrics
sed -i '' '/pub struct ConnectionMetrics {/i\
#[derive(Debug)]
' src/logging/metrics.rs

# Fix elided lifetime warnings by adding explicit lifetimes
echo "🔧 Fixing elided lifetime warnings..."

# Fix Chunks<'_> in mod.rs
sed -i '' "s/Result<Chunks, ReadableError>/Result<Chunks<'_>, ReadableError>/g" src/connection/streams/mod.rs

# Fix fmt::Formatter<'_> in various files
find src -name "*.rs" -exec sed -i '' "s/fmt::Formatter)/fmt::Formatter<'_>)/g" {} \;
find src -name "*.rs" -exec sed -i '' "s/&mut fmt::Formatter/\&mut fmt::Formatter<'_>/g" {} \;

echo "✅ Comprehensive ant-quic v0.6.1 fixes applied!"

# Test compilation
echo "🧪 Testing compilation..."
cargo check 2>&1 | head -20