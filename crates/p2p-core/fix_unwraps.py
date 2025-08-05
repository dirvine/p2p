#\!/usr/bin/env python3
"""
Systematic unwrap() elimination script for P2P Foundation production code.
"""

import os
import re
from pathlib import Path

def analyze_unwraps():
    """Analyze unwrap() calls in the codebase."""
    print("🔍 Analyzing unwrap() calls...")
    
    src_dir = Path("src")
    unwrap_files = {}
    
    for rust_file in src_dir.rglob("*.rs"):
        with open(rust_file, 'r', encoding='utf-8') as f:
            content = f.read()
            
        # Count unwrap() calls
        unwrap_matches = re.findall(r'\.unwrap\(\)', content)
        if unwrap_matches:
            unwrap_files[str(rust_file)] = len(unwrap_matches)
    
    # Sort by count
    sorted_files = sorted(unwrap_files.items(), key=lambda x: x[1], reverse=True)
    
    print(f"📊 Found {sum(unwrap_files.values())} unwrap() calls in {len(unwrap_files)} files")
    print("\nTop files with unwrap() calls:")
    for file_path, count in sorted_files[:10]:
        print(f"  {count:3d} - {file_path}")
    
    return sorted_files

def identify_test_vs_production_unwraps(file_path):
    """Identify which unwrap() calls are in test functions vs production code."""
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    test_unwraps = []
    production_unwraps = []
    in_test_context = False
    
    for i, line in enumerate(lines):
        # Check if we're in test context
        if any(marker in line for marker in ['#[test]', '#[tokio::test]', 'mod test', 'fn test_']):
            in_test_context = True
        elif line.strip().startswith('fn ') and 'test' not in line.lower():
            # Reset when we hit a non-test function
            if not any(marker in lines[max(0, i-3):i] for marker in ['#[test]', '#[tokio::test]']):
                in_test_context = False
        
        # Find unwrap() calls
        if '.unwrap()' in line:
            line_info = {
                'line_num': i + 1,
                'content': line.strip(),
                'is_test': in_test_context
            }
            
            if in_test_context:
                test_unwraps.append(line_info)
            else:
                production_unwraps.append(line_info)
    
    return test_unwraps, production_unwraps

def main():
    """Main execution function."""
    print("🎯 P2P Foundation Unwrap() Elimination Analysis")
    print("=" * 50)
    
    # Analyze unwraps
    sorted_files = analyze_unwraps()
    
    total_production_unwraps = 0
    total_test_unwraps = 0
    
    print("\n🔍 Detailed Analysis:")
    print("-" * 30)
    
    for file_path, count in sorted_files[:10]:  # Top 10 files
        print(f"\n📁 {file_path} ({count} unwraps)")
        test_unwraps, production_unwraps = identify_test_vs_production_unwraps(file_path)
        
        print(f"  Test unwraps: {len(test_unwraps)}")
        print(f"  Production unwraps: {len(production_unwraps)}")
        
        total_production_unwraps += len(production_unwraps)
        total_test_unwraps += len(test_unwraps)
        
        # Show first few production unwraps for manual fixing
        if production_unwraps:
            print("  Critical production unwraps:")
            for unwrap in production_unwraps[:2]:
                print(f"    Line {unwrap['line_num']}: {unwrap['content']}")
    
    print(f"\n📊 FINAL SUMMARY:")
    print(f"  Total production unwraps needing fixes: {total_production_unwraps}")
    print(f"  Total test unwraps (acceptable): {total_test_unwraps}")
    
    return total_production_unwraps

if __name__ == "__main__":
    main()
