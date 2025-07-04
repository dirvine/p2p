#!/bin/bash
# Detailed build check with specific error capturing

cd "$(dirname "$0")"

echo "🔍 Detailed Build Check for Saorsa Terminal Apps"
echo "==============================================="
echo ""

# Function to run cargo and capture output
run_cargo() {
    local cmd="$1"
    local desc="$2"
    echo "Running: $cmd"
    echo "Description: $desc"
    echo "----------------------------------------"
    
    # Run command and capture both stdout and stderr
    output=$($cmd 2>&1)
    exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        echo "✅ Success!"
    else
        echo "❌ Failed with exit code: $exit_code"
        echo ""
        echo "Output:"
        echo "$output" | head -50
        echo ""
        
        # Extract specific error types
        echo "Compilation errors:"
        echo "$output" | grep -E "error\[E[0-9]+\]" | head -10
        
        echo ""
        echo "Cannot find errors:"
        echo "$output" | grep -E "cannot find|unresolved import" | head -10
        
        echo ""
        echo "Type errors:"
        echo "$output" | grep -E "expected|found|mismatch" | head -10
    fi
    echo ""
    return $exit_code
}

# Set up environment
export PATH="$HOME/.cargo/bin:$PATH"

# Check workspace
echo "1. Checking workspace configuration..."
if grep -q "saorsa-terminal-chat" Cargo.toml && grep -q "saorsa-network-tester" Cargo.toml; then
    echo "✅ Both apps found in workspace"
else
    echo "❌ Apps not found in workspace Cargo.toml"
fi
echo ""

# Check dependencies
echo "2. Checking dependencies..."
echo "saorsa-terminal-chat dependencies:"
cat apps/saorsa-terminal-chat/Cargo.toml | grep -A5 "\[dependencies\]" | head -10
echo ""
echo "saorsa-network-tester dependencies:"
cat apps/saorsa-network-tester/Cargo.toml | grep -A5 "\[dependencies\]" | head -10
echo ""

# Try to build each app
echo "3. Building saorsa-terminal-chat..."
run_cargo "cargo build -p saorsa-terminal-chat" "Build terminal chat app"

echo "4. Building saorsa-network-tester..."
run_cargo "cargo build -p saorsa-network-tester" "Build network tester app"

# Try cargo check for more detailed errors
echo "5. Running cargo check for detailed errors..."
run_cargo "cargo check -p saorsa-terminal-chat --message-format=short" "Check terminal chat"
run_cargo "cargo check -p saorsa-network-tester --message-format=short" "Check network tester"

# Check if saorsa-core exists
echo "6. Checking saorsa-core (p2p-core) crate..."
if [ -d "crates/p2p-core" ]; then
    echo "✅ Found p2p-core at crates/p2p-core"
    echo "Package name in Cargo.toml:"
    grep "name = " crates/p2p-core/Cargo.toml | head -1
else
    echo "❌ Cannot find p2p-core crate"
fi

echo ""
echo "Build check complete!"