#!/bin/bash

# Script to identify and help fix unwrap() calls in production code
# Excludes test modules and test files

echo "=== Production Unwrap() Analysis ==="
echo

# Function to check if a line is inside a test module
is_in_test_context() {
    local file=$1
    local line_num=$2
    
    # Get the content up to the line number
    local content=$(head -n "$line_num" "$file" | tac)
    
    # Check if we're inside #[cfg(test)] or #[test]
    local in_test=0
    local brace_count=0
    
    while IFS= read -r line; do
        # Count braces
        brace_count=$((brace_count + $(echo "$line" | tr -cd '{' | wc -c)))
        brace_count=$((brace_count - $(echo "$line" | tr -cd '}' | wc -c)))
        
        # Check for test markers
        if [[ "$line" =~ \#\[cfg\(test\)\] ]] || [[ "$line" =~ \#\[test\] ]]; then
            if [ $brace_count -ge 0 ]; then
                in_test=1
                break
            fi
        fi
        
        # Check for mod tests
        if [[ "$line" =~ ^[[:space:]]*mod[[:space:]]+tests[[:space:]]*\{ ]]; then
            if [ $brace_count -ge 0 ]; then
                in_test=1
                break
            fi
        fi
    done <<< "$content"
    
    return $in_test
}

# Find all Rust files excluding test directories
files=$(find crates/p2p-core/src -name "*.rs" -type f | grep -v "/tests/" | grep -v "_test.rs" | grep -v "_tests.rs")

total_unwraps=0
production_unwraps=0
files_with_unwraps=""

echo "Analyzing files for production unwrap() calls..."
echo

for file in $files; do
    # Skip if file doesn't exist or is empty
    [ ! -s "$file" ] && continue
    
    # Find all unwrap() calls with line numbers
    unwraps=$(grep -n "\.unwrap()" "$file" 2>/dev/null | grep -v "^[[:space:]]*//")
    
    if [ -n "$unwraps" ]; then
        file_unwrap_count=0
        production_lines=""
        
        while IFS= read -r match; do
            line_num=$(echo "$match" | cut -d: -f1)
            line_content=$(echo "$match" | cut -d: -f2-)
            
            # Skip if in test context
            if ! is_in_test_context "$file" "$line_num"; then
                file_unwrap_count=$((file_unwrap_count + 1))
                production_unwraps=$((production_unwraps + 1))
                production_lines="${production_lines}  Line $line_num: $(echo "$line_content" | sed 's/^[[:space:]]*//' | cut -c1-80)...\n"
            fi
        done <<< "$unwraps"
        
        if [ $file_unwrap_count -gt 0 ]; then
            echo "📁 $file: $file_unwrap_count production unwrap() calls"
            echo -e "$production_lines"
            files_with_unwraps="${files_with_unwraps}$file\n"
        fi
    fi
done

echo "=== Summary ==="
echo "Total production unwrap() calls found: $production_unwraps"
echo
echo "Files that need fixing:"
echo -e "$files_with_unwraps"

# Create a fix template
cat > fix_unwraps_template.rs << 'EOF'
// Template for fixing common unwrap() patterns

// Pattern 1: Simple unwrap() -> Result propagation
// Before: let value = something.unwrap();
// After:  let value = something?;

// Pattern 2: unwrap() with context
// Before: let value = something.unwrap();
// After:  let value = something.context("Failed to get value")?;

// Pattern 3: unwrap() in match/if let
// Before: if condition { value.unwrap() }
// After:  if condition { value? }

// Pattern 4: Default fallback
// Before: let value = option.unwrap();
// After:  let value = option.unwrap_or_default();
// Or:     let value = option.ok_or(Error::MissingValue)?;

// Pattern 5: Expect with proper error
// Before: let value = result.unwrap();
// After:  let value = result.map_err(|e| Error::Custom(format!("Failed: {}", e)))?;

// Pattern 6: In initialization (lazy_static, once_cell)
// Before: static VAR: Lazy<Type> = Lazy::new(|| create().unwrap());
// After:  static VAR: Lazy<Result<Type>> = Lazy::new(|| create());
//         // Then use VAR.as_ref()? when accessing

// Pattern 7: Test helpers in production code
// Move to test module or use Result type
EOF

echo
echo "Fix template created in: fix_unwraps_template.rs"
echo
echo "Next steps:"
echo "1. Review each file listed above"
echo "2. Replace unwrap() with proper error handling"
echo "3. Run 'cargo check' after each fix"
echo "4. Update tests if needed"