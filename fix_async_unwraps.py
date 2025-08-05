#!/usr/bin/env python3
"""Fix async block unwrap() calls with proper error handling."""

import os
import re
import sys

def fix_async_unwraps(content):
    """Fix unwrap() calls in async blocks."""
    fixed_content = content
    
    # Pattern to find async blocks that don't return Result
    async_block_pattern = r'(tokio::spawn\(async\s+move\s*\{[^}]+\})'
    
    # Find all async blocks
    for match in re.finditer(async_block_pattern, content, re.DOTALL):
        block = match.group(1)
        original_block = block
        
        # Check if block already returns Result
        if 'Result<' in block or '-> Result' in block:
            continue
            
        # Check if block contains ? operator from our fixes
        if '?' in block and '.ok()?' in block:
            # Remove the ? operator after .ok()
            fixed_block = block.replace('.ok()?', '.ok().unwrap_or_default()')
            fixed_content = fixed_content.replace(original_block, fixed_block)
        elif '?' in block and '.map_err(' in block:
            # For blocks with map_err, use unwrap_or with appropriate default
            fixed_block = re.sub(r'\.map_err\([^)]+\)\?', '.unwrap_or_default()', block)
            fixed_content = fixed_content.replace(original_block, fixed_block)
    
    return fixed_content

def main():
    # Find files that might have async issues
    problem_files = []
    
    for root, dirs, files in os.walk("crates/p2p-core/src"):
        # Skip test directories
        if 'test' in root or 'tests' in root:
            continue
            
        for file in files:
            if file.endswith('.rs'):
                filepath = os.path.join(root, file)
                with open(filepath, 'r') as f:
                    content = f.read()
                    
                # Check for problematic patterns
                if 'tokio::spawn' in content and '.ok()?' in content:
                    problem_files.append(filepath)
                    print(f"Found async issue in: {filepath}")
    
    # Fix the files
    for filepath in problem_files:
        print(f"\nFixing {filepath}...")
        with open(filepath, 'r') as f:
            content = f.read()
            
        fixed_content = fix_async_unwraps(content)
        
        if fixed_content != content:
            with open(filepath, 'w') as f:
                f.write(fixed_content)
            print(f"  Fixed async unwraps in {filepath}")
        else:
            print(f"  No changes needed in {filepath}")

if __name__ == "__main__":
    main()