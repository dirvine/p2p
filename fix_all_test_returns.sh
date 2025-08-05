#!/bin/bash

# Find and fix all test functions that use ? operator but don't have Result return type

echo "Finding and fixing test return types..."

# First, find all test functions that use ? operator
find crates/p2p-core/src -name "*.rs" -type f | while read -r file; do
    # Create a temporary file
    temp_file="${file}.tmp"
    
    # Use perl for more complex pattern matching
    perl -pe '
    BEGIN { 
        $in_test = 0; 
        $needs_result = 0; 
        $brace_count = 0;
        $test_line = "";
        $buffer = "";
    }
    
    # Detect test attribute
    if (/^\s*#\[(tokio::)?test\]/) {
        $in_test = 1;
        $buffer = $_;
        next;
    }
    
    # If we have a test attribute buffered
    if ($in_test && /^\s*(async\s+)?fn\s+test_\w+\s*\([^)]*\)\s*(?:->.*?)?\s*\{/) {
        $test_line = $_;
        
        # Check if it already has a return type
        if ($test_line !~ /->/) {
            # Mark that we need to check for ? operator
            $needs_check = 1;
            $brace_count = 1;
            $func_start_line = $.;
            $original_test_line = $test_line;
        } else {
            $in_test = 0;
            $buffer = "";
        }
        
        print $buffer;
        $buffer = "";
        $_ = $test_line;
    }
    
    # Count braces if we are checking a function
    if ($needs_check) {
        $brace_count += tr/{/{/ - tr/}/}/;
        
        # Check if function uses ? operator
        if (/\?/) {
            $needs_result = 1;
        }
        
        # If we reach the end of the function
        if ($brace_count == 0) {
            # If function uses ? and needs Result return type
            if ($needs_result && $func_start_line) {
                # Go back and fix the function signature
                # Mark this line for special handling
                $_ = "##FIX_END##" . $_;
            }
            $needs_check = 0;
            $needs_result = 0;
            $func_start_line = 0;
        }
    }
    
    # Special handling for lines that need fixing
    if (/^##FIX_END##/) {
        s/^##FIX_END##//;
        # Insert Ok(()) before the closing brace
        s/^(\s*)\}/$1    Ok(())\n$1}/;
    }
    ' "$file" > "$temp_file"
    
    # Now fix the function signatures in a second pass
    perl -i -pe '
    BEGIN { 
        $fix_next_fn = 0;
        @lines = ();
        $line_num = 0;
    }
    
    # Read entire file into array
    push @lines, $_;
    END {
        for ($i = 0; $i < @lines; $i++) {
            $_ = $lines[$i];
            
            # Look for test functions
            if (/^\s*#\[(tokio::)?test\]/) {
                # Check next line
                if ($i + 1 < @lines && $lines[$i + 1] =~ /^\s*(async\s+)?fn\s+test_\w+\s*\([^)]*\)\s*\{/) {
                    # Look ahead to see if function uses ?
                    my $has_question = 0;
                    my $brace_count = 1;
                    for ($j = $i + 2; $j < @lines && $brace_count > 0; $j++) {
                        my $line = $lines[$j];
                        $brace_count += ($line =~ tr/{/{/) - ($line =~ tr/}/}/);
                        if ($line =~ /\?/ && $line !~ /^\s*\/\//) {
                            $has_question = 1;
                        }
                    }
                    
                    # If function uses ? and doesn\'t have return type, fix it
                    if ($has_question && $lines[$i + 1] !~ /->/) {
                        $lines[$i + 1] =~ s/\s*\{/ -> Result<()> {/;
                    }
                }
            }
            
            print $_;
        }
    }
    ' "$temp_file"
    
    # Check if the file was actually modified
    if ! cmp -s "$file" "$temp_file"; then
        mv "$temp_file" "$file"
        echo "  Fixed: $file"
    else
        rm "$temp_file"
    fi
done

echo "Finished fixing test return types"