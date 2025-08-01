#!/bin/bash

# Fix format! errors that need .into() for Cow<'_, str> conversion

# Find all Rust source files in p2p-core
find crates/p2p-core/src -name "*.rs" -type f | while read file; do
    # Create a temporary file
    tmp_file="${file}.tmp"
    
    # Process the file line by line
    awk '
    # If line ends with format!(...) and not followed by .into()
    /format!\(.*\)$/ && !/\.into\(\)/ {
        # Check if the next line has a closing parenthesis
        getline next_line
        if (next_line ~ /^[[:space:]]*\)\)/) {
            # This is part of a function call that needs .into()
            print $0 ".into()"
            print next_line
        } else {
            # Normal case - just add .into()
            print $0
            print next_line
        }
        next
    }
    # Print all other lines as-is
    { print }
    ' "$file" > "$tmp_file"
    
    # Replace original file if different
    if ! cmp -s "$file" "$tmp_file"; then
        mv "$tmp_file" "$file"
        echo "Fixed: $file"
    else
        rm "$tmp_file"
    fi
done

echo "Format error fixing complete"