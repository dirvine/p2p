#!/bin/bash

# Fix test return types in all Rust test files

echo "Fixing test return types in Rust files..."

# Find all test functions that use ? operator but don't have Result return type
find crates/p2p-core/src -name "*.rs" -type f | while read -r file; do
    echo "Processing: $file"
    
    # Create a temporary file
    temp_file="${file}.tmp"
    
    # Process the file
    awk '
    BEGIN { in_test = 0; needs_result = 0; brace_count = 0 }
    
    # Detect test function
    /^[[:space:]]*#\[tokio::test\]/ || /^[[:space:]]*#\[test\]/ {
        in_test = 1
        next
    }
    
    # Process test function signature
    in_test && /^[[:space:]]*async fn test_.*\([^)]*\)[[:space:]]*{/ {
        # Check if it already has a return type
        if ($0 !~ /->/) {
            # Store the line to check for ? operator later
            test_start_line = NR
            test_signature = $0
            gsub(/{/, "-> Result<()> {", test_signature)
            brace_count = 1
            needs_check = 1
        }
        in_test = 0
    }
    
    in_test && /^[[:space:]]*fn test_.*\([^)]*\)[[:space:]]*{/ {
        # Check if it already has a return type
        if ($0 !~ /->/) {
            # Store the line to check for ? operator later
            test_start_line = NR
            test_signature = $0
            gsub(/{/, "-> Result<()> {", test_signature)
            brace_count = 1
            needs_check = 1
        }
        in_test = 0
    }
    
    # Count braces if we are checking a function
    needs_check && /{/ { brace_count += gsub(/{/, "") }
    needs_check && /}/ { 
        brace_count -= gsub(/}/, "")
        if (brace_count == 0) {
            # End of function - add Ok(()) before the closing brace
            if (needs_result) {
                # Add Ok(()) before the closing brace
                sub(/^[[:space:]]*}/, "        Ok(())\n    }")
            }
            needs_check = 0
            needs_result = 0
        }
    }
    
    # Check if function uses ? operator
    needs_check && /\?/ {
        needs_result = 1
        if (test_start_line > 0) {
            # We need to update the signature
            lines[test_start_line] = test_signature
            test_start_line = 0
        }
    }
    
    # Store all lines
    { lines[NR] = $0 }
    
    END {
        # Print all lines
        for (i = 1; i <= NR; i++) {
            print lines[i]
        }
    }
    ' "$file" > "$temp_file"
    
    # Check if the file was modified
    if ! cmp -s "$file" "$temp_file"; then
        mv "$temp_file" "$file"
        echo "  Fixed: $file"
    else
        rm "$temp_file"
    fi
done

echo "Finished fixing test return types"