#!/usr/bin/env python3
"""
Systematically find and report all unwrap() calls in production code.
"""

import os
import re
from pathlib import Path
from typing import List, Tuple

def find_test_module_line(file_path: str) -> int:
    """Find the line where test module starts."""
    with open(file_path, 'r') as f:
        lines = f.readlines()
        for i, line in enumerate(lines, 1):
            if line.strip().startswith('#[cfg(test)]'):
                return i
    return len(lines) + 1  # No test module

def find_unwraps_in_file(file_path: str) -> List[Tuple[int, str]]:
    """Find all unwrap() calls in production code."""
    # Skip test files
    if '_test.rs' in file_path or '_tests.rs' in file_path:
        return []
    
    test_line = find_test_module_line(file_path)
    unwraps = []
    
    with open(file_path, 'r') as f:
        lines = f.readlines()
        for i, line in enumerate(lines[:test_line-1], 1):
            # Skip comments and doc strings
            stripped = line.strip()
            if stripped.startswith('//') or stripped.startswith('///'):
                continue
            if '/// ' in line and 'unwrap()' in line:
                continue  # Doc example
            
            # Find unwrap() calls
            if '.unwrap()' in line:
                # Check if it's in a string literal
                if '"' in line:
                    # Simple check - could be improved
                    parts = line.split('"')
                    code_parts = [parts[i] for i in range(0, len(parts), 2)]
                    if any('.unwrap()' in part for part in code_parts):
                        unwraps.append((i, line.rstrip()))
                else:
                    unwraps.append((i, line.rstrip()))
    
    return unwraps

def main():
    src_dir = Path('crates/p2p-core/src')
    all_unwraps = {}
    
    for rust_file in src_dir.rglob('*.rs'):
        unwraps = find_unwraps_in_file(str(rust_file))
        if unwraps:
            rel_path = rust_file.relative_to(src_dir)
            all_unwraps[str(rel_path)] = unwraps
    
    # Print summary
    total = sum(len(unwraps) for unwraps in all_unwraps.values())
    print(f"Found {total} unwrap() calls in {len(all_unwraps)} files:")
    print()
    
    # Sort by number of unwraps
    sorted_files = sorted(all_unwraps.items(), key=lambda x: len(x[1]), reverse=True)
    
    for file_path, unwraps in sorted_files[:10]:  # Top 10 files
        print(f"{file_path}: {len(unwraps)} unwraps")
        for line_no, line in unwraps[:3]:  # Show first 3
            print(f"  Line {line_no}: {line[:80]}...")
        if len(unwraps) > 3:
            print(f"  ... and {len(unwraps) - 3} more")
        print()

if __name__ == '__main__':
    main()