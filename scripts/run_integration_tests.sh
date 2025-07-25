#!/bin/bash
# Run integration tests for the adaptive P2P network

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_DIR="$PROJECT_ROOT/crates/p2p-integration-tests"
LOG_DIR="$PROJECT_ROOT/target/test-logs"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Default options
TEST_SUITE="all"
LOG_LEVEL="info"
TEST_THREADS="2"
SHOW_OUTPUT=""
RUN_IGNORED=""

# Parse command line arguments
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -s, --suite SUITE      Test suite to run (all, multi-node, churn, attack, performance, chaos)"
    echo "  -l, --log-level LEVEL  Log level (error, warn, info, debug, trace)"
    echo "  -t, --threads N        Number of test threads (default: 2)"
    echo "  -o, --output           Show test output (nocapture)"
    echo "  -i, --ignored          Run ignored (long-running) tests"
    echo "  -h, --help             Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                     # Run all tests"
    echo "  $0 -s multi-node       # Run only multi-node tests"
    echo "  $0 -l debug -o         # Run with debug logging and output"
    echo "  $0 -s performance -i   # Run performance tests including long-running ones"
}

while [[ $# -gt 0 ]]; do
    case $1 in
        -s|--suite)
            TEST_SUITE="$2"
            shift 2
            ;;
        -l|--log-level)
            LOG_LEVEL="$2"
            shift 2
            ;;
        -t|--threads)
            TEST_THREADS="$2"
            shift 2
            ;;
        -o|--output)
            SHOW_OUTPUT="--nocapture"
            shift
            ;;
        -i|--ignored)
            RUN_IGNORED="--ignored"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Create log directory
mkdir -p "$LOG_DIR"

# Function to run a test suite
run_test_suite() {
    local suite_name=$1
    local test_binary=$2
    local extra_args=${3:-}
    
    echo -e "${YELLOW}Running $suite_name tests...${NC}"
    
    local log_file="$LOG_DIR/${suite_name}_${TIMESTAMP}.log"
    
    if RUST_LOG="$LOG_LEVEL" cargo test \
        --manifest-path "$TEST_DIR/Cargo.toml" \
        --test "$test_binary" \
        -- --test-threads="$TEST_THREADS" $SHOW_OUTPUT $RUN_IGNORED $extra_args \
        2>&1 | tee "$log_file"; then
        echo -e "${GREEN}✓ $suite_name tests passed${NC}"
        return 0
    else
        echo -e "${RED}✗ $suite_name tests failed${NC}"
        echo "  See log: $log_file"
        return 1
    fi
}

# Function to run all test suites
run_all_tests() {
    local failed=0
    
    run_test_suite "Multi-Node" "multi_node" || ((failed++))
    run_test_suite "Churn Simulation" "churn_simulation" || ((failed++))
    run_test_suite "Attack Scenarios" "attack_scenarios" || ((failed++))
    run_test_suite "Performance" "performance_benchmarks" || ((failed++))
    run_test_suite "Chaos" "chaos_testing" || ((failed++))
    
    return $failed
}

# Main execution
echo "P2P Integration Test Runner"
echo "=========================="
echo "Test suite: $TEST_SUITE"
echo "Log level: $LOG_LEVEL"
echo "Test threads: $TEST_THREADS"
echo "Log directory: $LOG_DIR"
echo ""

# Change to project root
cd "$PROJECT_ROOT"

# Ensure the project is built
echo -e "${YELLOW}Building project...${NC}"
cargo build --workspace

# Run requested test suite
case $TEST_SUITE in
    all)
        run_all_tests
        failed=$?
        ;;
    multi-node)
        run_test_suite "Multi-Node" "multi_node"
        failed=$?
        ;;
    churn)
        run_test_suite "Churn Simulation" "churn_simulation"
        failed=$?
        ;;
    attack)
        run_test_suite "Attack Scenarios" "attack_scenarios"
        failed=$?
        ;;
    performance)
        run_test_suite "Performance" "performance_benchmarks"
        failed=$?
        ;;
    chaos)
        run_test_suite "Chaos" "chaos_testing"
        failed=$?
        ;;
    *)
        echo -e "${RED}Unknown test suite: $TEST_SUITE${NC}"
        usage
        exit 1
        ;;
esac

# Summary
echo ""
echo "Test Summary"
echo "============"

if [[ $failed -eq 0 ]]; then
    echo -e "${GREEN}All tests passed!${NC}"
else
    echo -e "${RED}$failed test suite(s) failed${NC}"
    echo "Check logs in: $LOG_DIR"
fi

# Generate HTML report if tests passed and criterion is available
if [[ $failed -eq 0 ]] && [[ -d "$PROJECT_ROOT/target/criterion" ]]; then
    echo ""
    echo -e "${YELLOW}Generating performance report...${NC}"
    if command -v python3 &> /dev/null; then
        cat > "$LOG_DIR/report_${TIMESTAMP}.html" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>P2P Integration Test Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        h1 { color: #333; }
        .suite { margin: 20px 0; padding: 10px; border: 1px solid #ddd; }
        .passed { background-color: #d4edda; }
        .failed { background-color: #f8d7da; }
        .metric { margin: 5px 0; }
    </style>
</head>
<body>
    <h1>P2P Integration Test Report</h1>
    <p>Generated: TIMESTAMP</p>
    <div class="suite passed">
        <h2>All Tests Passed</h2>
        <p>See detailed performance metrics in target/criterion/report/index.html</p>
    </div>
</body>
</html>
EOF
        sed -i.bak "s/TIMESTAMP/$(date)/" "$LOG_DIR/report_${TIMESTAMP}.html"
        rm "$LOG_DIR/report_${TIMESTAMP}.html.bak"
        echo -e "${GREEN}Report generated: $LOG_DIR/report_${TIMESTAMP}.html${NC}"
    fi
fi

exit $failed