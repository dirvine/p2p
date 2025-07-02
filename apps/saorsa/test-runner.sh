#!/bin/bash

# Saorsa Comprehensive Test Runner
# This script runs all tests for the Saorsa application

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test categories
UNIT_TESTS=true
INTEGRATION_TESTS=true
FRONTEND_TESTS=true
PERFORMANCE_TESTS=true

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --unit-only)
            INTEGRATION_TESTS=false
            FRONTEND_TESTS=false
            PERFORMANCE_TESTS=false
            shift
            ;;
        --integration-only)
            UNIT_TESTS=false
            FRONTEND_TESTS=false
            PERFORMANCE_TESTS=false
            shift
            ;;
        --frontend-only)
            UNIT_TESTS=false
            INTEGRATION_TESTS=false
            PERFORMANCE_TESTS=false
            shift
            ;;
        --skip-frontend)
            FRONTEND_TESTS=false
            shift
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  --unit-only        Run only unit tests"
            echo "  --integration-only Run only integration tests"
            echo "  --frontend-only    Run only frontend tests"
            echo "  --skip-frontend    Skip frontend tests"
            echo "  --help            Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}=== Saorsa Comprehensive Test Suite ===${NC}"
echo ""

# Function to run tests and check results
run_test() {
    local test_name=$1
    local test_command=$2
    
    echo -e "${YELLOW}Running $test_name...${NC}"
    
    if eval "$test_command"; then
        echo -e "${GREEN}✓ $test_name passed${NC}"
        return 0
    else
        echo -e "${RED}✗ $test_name failed${NC}"
        return 1
    fi
}

# Track test results
FAILED_TESTS=()
TOTAL_TESTS=0
PASSED_TESTS=0

# Change to Tauri app directory
cd src-tauri

# 1. Unit Tests
if [ "$UNIT_TESTS" = true ]; then
    echo -e "${BLUE}=== Unit Tests ===${NC}"
    echo ""
    
    # Run individual unit test files
    for test_file in "lib_tests" "passkey_auth_tests" "identity_storage_tests"; do
        ((TOTAL_TESTS++))
        if run_test "$test_file" "cargo test --test $test_file -- --nocapture"; then
            ((PASSED_TESTS++))
        else
            FAILED_TESTS+=("$test_file")
        fi
        echo ""
    done
fi

# 2. Integration Tests
if [ "$INTEGRATION_TESTS" = true ]; then
    echo -e "${BLUE}=== Integration Tests ===${NC}"
    echo ""
    
    ((TOTAL_TESTS++))
    if run_test "Integration Tests" "cargo test --test integration_tests -- --nocapture --test-threads=1"; then
        ((PASSED_TESTS++))
    else
        FAILED_TESTS+=("Integration Tests")
    fi
    echo ""
fi

# 3. Frontend Tests
if [ "$FRONTEND_TESTS" = true ]; then
    echo -e "${BLUE}=== Frontend Tests ===${NC}"
    echo ""
    
    # Check if dev server is running
    if ! curl -s http://localhost:1420 > /dev/null; then
        echo -e "${YELLOW}Starting dev server for frontend tests...${NC}"
        cd ..
        npm run tauri dev &
        DEV_SERVER_PID=$!
        
        # Wait for dev server to start
        echo "Waiting for dev server to start..."
        for i in {1..30}; do
            if curl -s http://localhost:1420 > /dev/null; then
                break
            fi
            sleep 1
        done
        
        cd src-tauri
    fi
    
    ((TOTAL_TESTS++))
    if run_test "Frontend Tests" "cargo test --test frontend_tests -- --nocapture"; then
        ((PASSED_TESTS++))
    else
        FAILED_TESTS+=("Frontend Tests")
    fi
    
    # Kill dev server if we started it
    if [ ! -z "$DEV_SERVER_PID" ]; then
        echo "Stopping dev server..."
        kill $DEV_SERVER_PID 2>/dev/null || true
    fi
    echo ""
fi

# 4. Performance Tests (optional)
if [ "$PERFORMANCE_TESTS" = true ] && [ -f "tests/performance_tests.rs" ]; then
    echo -e "${BLUE}=== Performance Tests ===${NC}"
    echo ""
    
    ((TOTAL_TESTS++))
    if run_test "Performance Tests" "cargo test --test performance_tests --release -- --nocapture"; then
        ((PASSED_TESTS++))
    else
        FAILED_TESTS+=("Performance Tests")
    fi
    echo ""
fi

# 5. Clippy (Linting)
echo -e "${BLUE}=== Running Clippy ===${NC}"
echo ""

((TOTAL_TESTS++))
if run_test "Clippy" "cargo clippy --all-targets --all-features -- -D warnings"; then
    ((PASSED_TESTS++))
else
    FAILED_TESTS+=("Clippy")
fi
echo ""

# 6. Format Check
echo -e "${BLUE}=== Format Check ===${NC}"
echo ""

((TOTAL_TESTS++))
if run_test "Format Check" "cargo fmt -- --check"; then
    ((PASSED_TESTS++))
else
    FAILED_TESTS+=("Format Check")
fi
echo ""

# Summary
echo -e "${BLUE}=== Test Summary ===${NC}"
echo ""
echo -e "Total tests run: $TOTAL_TESTS"
echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
echo -e "${RED}Failed: $((TOTAL_TESTS - PASSED_TESTS))${NC}"

if [ ${#FAILED_TESTS[@]} -gt 0 ]; then
    echo ""
    echo -e "${RED}Failed tests:${NC}"
    for test in "${FAILED_TESTS[@]}"; do
        echo -e "  - $test"
    done
    exit 1
else
    echo ""
    echo -e "${GREEN}All tests passed! 🎉${NC}"
    exit 0
fi