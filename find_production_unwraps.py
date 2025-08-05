#!/usr/bin/env python3
"""
Find unwrap() calls in production code (excluding tests)
"""
import os
import re
from pathlib import Path

def is_test_code(file_path, line_num, lines):
    """Check if a line is in test code"""
    # Check if file is a test file
    if 'test' in file_path.lower() or file_path.endswith('_test.rs'):
        return True
    
    # Look for test module markers before the line
    for i in range(max(0, line_num - 100), line_num):
        if i < len(lines):
            line = lines[i]
            if '#[cfg(test)]' in line or '#[test]' in line:
                return True
            if re.match(r'^\s*mod\s+tests\s*\{', line):
                return True
    
    return False

def find_unwraps(directory):
    """Find all unwrap() calls in non-test code"""
    results = {}
    total_unwraps = 0
    
    for root, dirs, files in os.walk(directory):
        # Skip test directories
        dirs[:] = [d for d in dirs if 'test' not in d.lower()]
        
        for file in files:
            if file.endswith('.rs'):
                file_path = os.path.join(root, file)
                
                try:
                    with open(file_path, 'r') as f:
                        lines = f.readlines()
                    
                    unwraps = []
                    for i, line in enumerate(lines):
                        if '.unwrap()' in line and not line.strip().startswith('//'):
                            if not is_test_code(file_path, i, lines):
                                unwraps.append({
                                    'line_num': i + 1,
                                    'line': line.strip()[:100]
                                })
                                total_unwraps += 1
                    
                    if unwraps:
                        results[file_path] = unwraps
                        
                except Exception as e:
                    print(f"Error reading {file_path}: {e}")
    
    return results, total_unwraps

def main():
    directory = 'crates/p2p-core/src'
    results, total = find_unwraps(directory)
    
    print("=== Production unwrap() Analysis ===\n")
    
    # Sort by number of unwraps
    sorted_files = sorted(results.items(), key=lambda x: len(x[1]), reverse=True)
    
    for file_path, unwraps in sorted_files[:20]:  # Top 20 files
        rel_path = os.path.relpath(file_path)
        print(f"📁 {rel_path}: {len(unwraps)} unwrap() calls")
        for unwrap in unwraps[:5]:  # Show first 5 unwraps
            print(f"   Line {unwrap['line_num']}: {unwrap['line']}")
        if len(unwraps) > 5:
            print(f"   ... and {len(unwraps) - 5} more")
        print()
    
    print(f"\n=== Summary ===")
    print(f"Total production unwrap() calls: {total}")
    print(f"Files affected: {len(results)}")
    
    # Create prioritized fix list
    print("\n=== Priority Fix Order ===")
    critical_files = [
        'transport/quic.rs',
        'identity_manager.rs', 
        'encrypted_key_storage.rs',
        'mcp.rs',
        'network.rs',
        'dht.rs'
    ]
    
    for cf in critical_files:
        for file_path, unwraps in results.items():
            if cf in file_path:
                print(f"🔴 CRITICAL: {os.path.relpath(file_path)} - {len(unwraps)} unwraps")
                break

if __name__ == '__main__':
    main()