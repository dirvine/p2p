#!/usr/bin/env python3
"""Fix all test functions that use ? operator but don't have Result return type."""

import re
import os
import glob

def fix_test_functions(filepath):
    """Fix test functions in a single file."""
    with open(filepath, 'r') as f:
        content = f.read()
    
    lines = content.split('\n')
    modified = False
    i = 0
    
    while i < len(lines):
        line = lines[i]
        
        # Check for test attribute
        if re.match(r'^\s*#\[(tokio::)?test\]', line):
            # Look at next line for function signature
            if i + 1 < len(lines):
                next_line = lines[i + 1]
                func_match = re.match(r'^(\s*)(async\s+)?fn\s+(test_\w+)\s*\([^)]*\)\s*(?:->.*?)?\s*\{', next_line)
                
                if func_match and ' -> ' not in next_line:
                    # Check if this function uses ? operator
                    func_indent = func_match.group(1)
                    brace_count = 1
                    uses_question = False
                    j = i + 2
                    
                    while j < len(lines) and brace_count > 0:
                        func_line = lines[j]
                        brace_count += func_line.count('{') - func_line.count('}')
                        
                        # Check for ? operator (not in comments)
                        if '?' in func_line and not func_line.strip().startswith('//'):
                            uses_question = True
                        
                        # If we found the closing brace and function uses ?
                        if brace_count == 0 and uses_question:
                            # Add Result return type
                            lines[i + 1] = re.sub(r'\s*\{', ' -> Result<()> {', lines[i + 1])
                            
                            # Add Ok(()) before closing brace
                            if re.match(rf'^{func_indent}\}}', lines[j]):
                                lines[j] = f'{func_indent}    Ok(())\n{func_indent}}}'
                            
                            modified = True
                            break
                        
                        j += 1
        
        i += 1
    
    if modified:
        with open(filepath, 'w') as f:
            f.write('\n'.join(lines))
        return True
    
    return False

def main():
    """Fix all test functions in the project."""
    print("Fixing test functions with ? operator...")
    
    # Find all Rust source files
    rust_files = []
    for pattern in ['crates/p2p-core/src/**/*.rs', 'crates/p2p-core/tests/**/*.rs']:
        rust_files.extend(glob.glob(pattern, recursive=True))
    
    fixed_count = 0
    for filepath in rust_files:
        if fix_test_functions(filepath):
            print(f"  Fixed: {filepath}")
            fixed_count += 1
    
    print(f"\nFixed {fixed_count} files")

if __name__ == '__main__':
    main()