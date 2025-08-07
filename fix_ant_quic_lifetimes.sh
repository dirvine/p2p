#!/bin/bash
# Script to help fix ant-quic Rust 2024 edition lifetime issues

echo "🔧 Analyzing ant-quic lifetime issues..."

cd /Users/davidirvine/Desktop/Devel/projects/ant-quic

# Count total errors
echo "📊 Total errors to fix:"
cargo check 2>&1 | grep "error:" | wc -l

# Find all files with lifetime issues
echo -e "\n📁 Files needing lifetime fixes:"
cargo check 2>&1 | grep "hidden lifetime parameters" | grep -oE "src/[^:]+\.rs" | sort | uniq

# Most common patterns that need fixing:
echo -e "\n🔍 Common patterns to fix:"
echo "1. fmt::Formatter needs fmt::Formatter<'_>"
echo "2. Chunks needs Chunks<'_>"
echo "3. Iterator types need explicit lifetimes"

# Offer automated fix for common patterns
echo -e "\n🚀 Quick fixes available:"
echo "1. Fix all Formatter references:"
echo "   find src -name '*.rs' -exec sed -i '' 's/fmt::Formatter)/fmt::Formatter<'\''_>)/g' {} +"
echo ""
echo "2. Fix Result<Chunks> references:"
echo "   find src -name '*.rs' -exec sed -i '' 's/Result<Chunks,/Result<Chunks<'\''_>,/g' {} +"
echo ""
echo "3. Fix Option<Chunks> references:"
echo "   find src -name '*.rs' -exec sed -i '' 's/Option<Chunks>/Option<Chunks<'\''_>>/g' {} +"

echo -e "\n📝 To apply all fixes automatically, run:"
echo "   ./fix_ant_quic_lifetimes.sh --apply"

if [ "$1" == "--apply" ]; then
    echo -e "\n✨ Applying automatic fixes..."
    
    # Fix Formatter references
    find src -name '*.rs' -exec sed -i '' "s/fmt::Formatter)/fmt::Formatter<'_>)/g" {} +
    echo "✅ Fixed Formatter references"
    
    # Fix Chunks in Result types
    find src -name '*.rs' -exec sed -i '' "s/Result<Chunks,/Result<Chunks<'_>,/g" {} +
    echo "✅ Fixed Result<Chunks> references"
    
    # Fix Chunks in Option types
    find src -name '*.rs' -exec sed -i '' "s/Option<Chunks>/Option<Chunks<'_>>/g" {} +
    echo "✅ Fixed Option<Chunks> references"
    
    # Fix standalone Chunks references in function signatures
    find src -name '*.rs' -exec sed -i '' "s/) -> Chunks {/) -> Chunks<'_> {/g" {} +
    echo "✅ Fixed function return types"
    
    echo -e "\n🔍 Checking remaining errors..."
    cargo check 2>&1 | grep "error:" | wc -l
    echo "errors remaining"
fi