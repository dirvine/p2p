#!/usr/bin/env python3
"""Fix P2PError::Parse references to use NetworkError::InvalidAddress."""

import re

def fix_parse_errors():
    filepath = "crates/p2p-core/src/transport.rs"
    
    with open(filepath, 'r') as f:
        content = f.read()
    
    # Replace P2PError::Parse with NetworkError
    fixed_content = content.replace(
        'P2PError::Parse(format!("Parse error: {}", e))',
        'P2PError::Network(NetworkError::InvalidAddress(format!("{}", e).into()))'
    )
    
    with open(filepath, 'w') as f:
        f.write(fixed_content)
    
    print(f"Fixed P2PError::Parse references in {filepath}")

if __name__ == "__main__":
    fix_parse_errors()