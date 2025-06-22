#!/bin/bash

# P2P Foundation Integration Test Runner
# Usage: ./test-runner.sh [environment] [options]

set -e

# Default configuration
DEFAULT_ENV="dev"
DEFAULT_NODE_COUNT=3
DEFAULT_BASE_PORT=9000
DEFAULT_TIMEOUT=300
DEFAULT_LOG_LEVEL="info"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Help function
show_help() {
    cat << EOF
P2P Foundation Integration Test Runner

Usage: $0 [ENVIRONMENT] [OPTIONS]

ENVIRONMENTS:
    dev         Development environment (default)
                - Verbose logging, all features enabled
                - Node count: 5, Timeout: 600s
    
    ci          Continuous Integration environment  
                - Minimal logging, IPv6 disabled
                - Node count: 3, Timeout: 180s
    
    bench       Benchmark environment
                - Performance tests enabled
                - Node count: 10, Timeout: 900s
    
    stress      Stress testing environment
                - Stress tests enabled, long timeouts
                - Node count: 8, Timeout: 1200s

OPTIONS:
    -h, --help              Show this help message
    -n, --node-count NUM    Number of nodes to use (default: $DEFAULT_NODE_COUNT)
    -p, --port NUM          Base port number (default: $DEFAULT_BASE_PORT)
    -t, --timeout NUM       Test timeout in seconds (default: $DEFAULT_TIMEOUT)
    -l, --log-level LEVEL   Log level: debug,info,warn,error (default: $DEFAULT_LOG_LEVEL)
    --ipv6                  Enable IPv6 tests
    --no-ipv6               Disable IPv6 tests
    --benchmarks            Enable benchmark tests
    --no-benchmarks         Disable benchmark tests
    --stress                Enable stress tests
    --no-stress             Disable stress tests
    --features FEATURES     Cargo features to enable
    --clean                 Clean build before running tests
    --release               Run tests in release mode
    --single-thread         Run tests single-threaded
    --module MODULE         Run specific test module only

EXAMPLES:
    $0                          # Run with dev environment
    $0 ci                       # Run with CI environment
    $0 dev --benchmarks         # Run dev tests with benchmarks
    $0 stress --node-count 10   # Run stress tests with 10 nodes
    $0 dev --module network     # Run only network module tests

MODULES:
    network     Network layer tests
    dht         DHT functionality tests  
    transport   Transport layer tests
    tunneling   Tunneling protocol tests
    mcp         MCP server tests
    security    Security and crypto tests
    scenarios   End-to-end scenario tests
    all         All integration tests (default)

EOF
}

# Set environment variables based on environment
setup_environment() {
    local env=$1
    
    case $env in
        "dev")
            export P2P_TEST_ENV="dev"
            export P2P_TEST_NODE_COUNT=5
            export P2P_TEST_BASE_PORT=8000
            export P2P_TEST_TIMEOUT=600
            export P2P_TEST_LOG_LEVEL="debug"
            export P2P_TEST_ENABLE_IPV6="true"
            export P2P_TEST_ENABLE_BENCHMARKS="true"
            export P2P_TEST_ENABLE_STRESS="true"
            ;;
        "ci")
            export P2P_TEST_ENV="ci"
            export P2P_TEST_NODE_COUNT=3
            export P2P_TEST_BASE_PORT=19000
            export P2P_TEST_TIMEOUT=180
            export P2P_TEST_LOG_LEVEL="warn"
            export P2P_TEST_ENABLE_IPV6="false"
            export P2P_TEST_ENABLE_BENCHMARKS="false"
            export P2P_TEST_ENABLE_STRESS="false"
            ;;
        "bench")
            export P2P_TEST_ENV="bench"
            export P2P_TEST_NODE_COUNT=10
            export P2P_TEST_BASE_PORT=10000
            export P2P_TEST_TIMEOUT=900
            export P2P_TEST_LOG_LEVEL="info"
            export P2P_TEST_ENABLE_IPV6="true"
            export P2P_TEST_ENABLE_BENCHMARKS="true"
            export P2P_TEST_ENABLE_STRESS="false"
            ;;
        "stress")
            export P2P_TEST_ENV="stress"
            export P2P_TEST_NODE_COUNT=8
            export P2P_TEST_BASE_PORT=11000
            export P2P_TEST_TIMEOUT=1200
            export P2P_TEST_LOG_LEVEL="info"
            export P2P_TEST_ENABLE_IPV6="true"
            export P2P_TEST_ENABLE_BENCHMARKS="false"
            export P2P_TEST_ENABLE_STRESS="true"
            ;;
        *)
            print_error "Unknown environment: $env"
            print_info "Valid environments: dev, ci, bench, stress"
            exit 1
            ;;
    esac
}

# Check system requirements
check_requirements() {
    print_info "Checking system requirements..."
    
    # Check Rust installation
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    
    local rust_version
    rust_version=$(rustc --version)
    print_info "Rust version: $rust_version"
    
    # Check available ports
    local base_port=${P2P_TEST_BASE_PORT:-$DEFAULT_BASE_PORT}
    if netstat -tuln 2>/dev/null | grep -q ":$base_port "; then
        print_warning "Port $base_port appears to be in use"
        print_info "Tests will attempt to use alternative ports"
    fi
    
    # Check available memory
    if command -v free &> /dev/null; then
        local available_mem
        available_mem=$(free -m | awk 'NR==2{printf "%.0f", $7}')
        if [ "$available_mem" -lt 1000 ]; then
            print_warning "Available memory is low (${available_mem}MB). Some tests may fail."
        else
            print_info "Available memory: ${available_mem}MB"
        fi
    fi
    
    # Check IPv6 availability
    if [ "${P2P_TEST_ENABLE_IPV6:-true}" = "true" ]; then
        # Check IPv6 availability (cross-platform)
        if command -v ip &> /dev/null; then
            # Linux
            if ! ip -6 addr show lo | grep -q "::1"; then
                print_warning "IPv6 localhost not available. IPv6 tests will be skipped."
                export P2P_TEST_ENABLE_IPV6="false"
            fi
        elif command -v ifconfig &> /dev/null; then
            # macOS/BSD
            if ! ifconfig lo0 | grep -q "inet6.*::1"; then
                print_warning "IPv6 localhost not available. IPv6 tests will be skipped."
                export P2P_TEST_ENABLE_IPV6="false"
            fi
        else
            print_warning "Cannot check IPv6 availability. IPv6 tests may fail."
        fi
    fi
}

# Run specific test module
run_test_module() {
    local module=$1
    local cargo_args=$2
    
    case $module in
        "network")
            print_info "Running network module tests..."
            cargo test --test integration_tests network_tests $cargo_args
            ;;
        "dht")
            print_info "Running DHT module tests..."
            cargo test --test integration_tests dht_tests $cargo_args
            ;;
        "transport")
            print_info "Running transport layer tests..."
            cargo test --test integration_tests transport_tests $cargo_args
            ;;
        "tunneling")
            print_info "Running tunneling protocol tests..."
            cargo test --test integration_tests tunneling_tests $cargo_args
            ;;
        "mcp")
            print_info "Running MCP server tests..."
            cargo test --test integration_tests mcp_tests $cargo_args
            ;;
        "security")
            print_info "Running security module tests..."
            cargo test --test integration_tests security_tests $cargo_args
            ;;
        "scenarios")
            print_info "Running end-to-end scenario tests..."
            cargo test --test integration_tests scenario_tests $cargo_args
            ;;
        "all")
            print_info "Running all integration tests..."
            cargo test --test integration_tests $cargo_args
            ;;
        *)
            print_error "Unknown test module: $module"
            print_info "Valid modules: network, dht, transport, tunneling, mcp, security, scenarios, all"
            exit 1
            ;;
    esac
}

# Main function
main() {
    local environment="$DEFAULT_ENV"
    local module="all"
    local cargo_args=""
    local clean_build=false
    local release_mode=false
    local single_thread=false
    local features=""
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            dev|ci|bench|stress)
                environment="$1"
                shift
                ;;
            -n|--node-count)
                export P2P_TEST_NODE_COUNT="$2"
                shift 2
                ;;
            -p|--port)
                export P2P_TEST_BASE_PORT="$2"
                shift 2
                ;;
            -t|--timeout)
                export P2P_TEST_TIMEOUT="$2"
                shift 2
                ;;
            -l|--log-level)
                export P2P_TEST_LOG_LEVEL="$2"
                shift 2
                ;;
            --ipv6)
                export P2P_TEST_ENABLE_IPV6="true"
                shift
                ;;
            --no-ipv6)
                export P2P_TEST_ENABLE_IPV6="false"
                shift
                ;;
            --benchmarks)
                export P2P_TEST_ENABLE_BENCHMARKS="true"
                shift
                ;;
            --no-benchmarks)
                export P2P_TEST_ENABLE_BENCHMARKS="false"
                shift
                ;;
            --stress)
                export P2P_TEST_ENABLE_STRESS="true"
                shift
                ;;
            --no-stress)
                export P2P_TEST_ENABLE_STRESS="false"
                shift
                ;;
            --features)
                features="$2"
                shift 2
                ;;
            --clean)
                clean_build=true
                shift
                ;;
            --release)
                release_mode=true
                shift
                ;;
            --single-thread)
                single_thread=true
                shift
                ;;
            --module)
                module="$2"
                shift 2
                ;;
            *)
                print_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
    
    # Setup environment
    setup_environment "$environment"
    
    print_info "P2P Foundation Integration Test Runner"
    print_info "======================================"
    print_info "Environment: $environment"
    print_info "Module: $module"
    print_info "Node count: ${P2P_TEST_NODE_COUNT}"
    print_info "Base port: ${P2P_TEST_BASE_PORT}"
    print_info "Timeout: ${P2P_TEST_TIMEOUT}s"
    print_info "Log level: ${P2P_TEST_LOG_LEVEL}"
    print_info "IPv6: ${P2P_TEST_ENABLE_IPV6}"
    print_info "Benchmarks: ${P2P_TEST_ENABLE_BENCHMARKS}"
    print_info "Stress tests: ${P2P_TEST_ENABLE_STRESS}"
    echo
    
    # Check requirements
    check_requirements
    echo
    
    # Build cargo arguments
    if [ "$features" != "" ]; then
        cargo_args="$cargo_args --features $features"
    fi
    
    if [ "$release_mode" = true ]; then
        cargo_args="$cargo_args --release"
    fi
    
    if [ "$single_thread" = true ]; then
        cargo_args="$cargo_args -- --test-threads=1"
    fi
    
    # Clean build if requested
    if [ "$clean_build" = true ]; then
        print_info "Cleaning previous build..."
        cargo clean
    fi
    
    # Set logging environment
    export RUST_LOG="${P2P_TEST_LOG_LEVEL}"
    export RUST_BACKTRACE=1
    
    # Run tests
    print_info "Starting test execution..."
    echo
    
    local start_time
    start_time=$(date +%s)
    
    if run_test_module "$module" "$cargo_args"; then
        local end_time
        end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        echo
        print_success "All tests completed successfully!"
        print_info "Total execution time: ${duration}s"
    else
        local end_time
        end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        echo
        print_error "Some tests failed!"
        print_info "Execution time: ${duration}s"
        exit 1
    fi
}

# Run main function with all arguments
main "$@"