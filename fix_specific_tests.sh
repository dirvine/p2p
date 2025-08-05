#!/bin/bash

# Fix specific test functions that we know have issues

echo "Fixing specific test functions with return type issues..."

# Fix remaining tests in production.rs
echo "Fixing production.rs..."
sed -i '' 's/async fn test_rate_limiting_mcp_operations()/async fn test_rate_limiting_mcp_operations() -> Result<()>/' crates/p2p-core/src/production.rs

# Add Ok(()) to end of test functions that need it
perl -i -pe '
    if (/async fn test_rate_limiting_mcp_operations\(\) -> Result<\(\)>/) {
        $in_func = 1;
        $brace_count = 0;
    }
    if ($in_func) {
        $brace_count += tr/{/{/ - tr/}/}/;
        if ($brace_count == 0 && /^\s*\}/) {
            s/^(\s*)\}/$1    Ok(())\n$1}/;
            $in_func = 0;
        }
    }
' crates/p2p-core/src/production.rs

# Check for any remaining test functions with ? operator
echo "Checking for remaining test functions with ? operator..."
grep -n "^\s*async fn test_" crates/p2p-core/src/*.rs | while read -r line; do
    file=$(echo "$line" | cut -d: -f1)
    line_num=$(echo "$line" | cut -d: -f2)
    func_name=$(echo "$line" | grep -o "test_[a-zA-Z0-9_]*")
    
    # Check if this function uses ? operator
    if sed -n "${line_num},/^[[:space:]]*}/p" "$file" | grep -q '\?'; then
        # Check if it already has Result return type
        if ! sed -n "${line_num}p" "$file" | grep -q "-> Result"; then
            echo "  Need to fix: $file:$line_num - $func_name"
            
            # Fix the function signature
            sed -i '' "${line_num}s/\s*{/ -> Result<()> {/" "$file"
            
            # Find the closing brace and add Ok(())
            end_line=$(sed -n "${line_num},\$p" "$file" | awk 'BEGIN{bc=0} /{/{bc++} /}/{bc--; if(bc==0){print NR; exit}}')
            end_line=$((line_num + end_line - 1))
            
            # Insert Ok(()) before the closing brace
            sed -i '' "${end_line}i\\
    Ok(())
" "$file"
        fi
    fi
done

echo "Done fixing specific tests"