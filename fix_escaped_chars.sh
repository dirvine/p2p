#!/bin/bash

echo "Fixing escaped characters in Rust files..."

# Fix escaped exclamation marks in test and benchmark files
files=(
    "./crates/p2p-core/tests/storage_integration_comprehensive_test.rs"
    "./crates/p2p-core/tests/production_integration_test.rs"
    "./crates/p2p-core/tests/security_integration_comprehensive_test.rs"
    "./crates/p2p-core/tests/integration_test_runner.rs"
    "./crates/p2p-core/tests/network_integration_comprehensive_test.rs"
    "./crates/p2p-core/benches/integration_benchmarks.rs"
    "./crates/p2p-core/benches/performance_monitor.rs"
    "./crates/p2p-core/benches/load_testing_scenarios.rs"
    "./crates/p2p-core/benches/comprehensive_performance_test.rs"
    "./crates/p2p-core/src/health/business_metrics.rs"
    "./apps/communitas/src-tauri/src/main_complex.rs"
)

for file in "${files[@]}"; do
    if [ -f "$file" ]; then
        echo "Fixing $file..."
        # Replace \! with ! in string literals
        sed -i '' 's/\\!/!/g' "$file"
    fi
done

echo "Running cargo fmt to format all code..."
cargo fmt --all

echo "Done!"