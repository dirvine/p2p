#!/bin/bash

# License header template
LICENSE_HEADER='// Copyright 2024 MaidSafe Limited
//
// This software is dual-licensed under:
// - GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later)
// - Commercial License
//
// For AGPL-3.0 license, see LICENSE-AGPL-3.0
// For commercial licensing, contact: saorsalabs@gmail.com
//
// Unless required by applicable law or agreed to in writing, software
// distributed under these licenses is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.

'

# Function to add license header to a file
add_license_header() {
    local file="$1"
    echo "Adding license header to: $file"
    
    # Create temporary file with license header + original content
    temp_file=$(mktemp)
    echo -n "$LICENSE_HEADER" > "$temp_file"
    cat "$file" >> "$temp_file"
    
    # Replace original file
    mv "$temp_file" "$file"
    
    # Remove any empty lines at the beginning
    sed -i '' '1{/^$/d;}' "$file"
}

# Core files that MUST have license headers for CI
CORE_FILES=(
    "crates/p2p-core/src/transport/tcp.rs"
    "crates/p2p-core/src/transport/tunneled.rs"
    "crates/p2p-core/src/bootstrap/discovery.rs"
    "crates/p2p-core/src/bootstrap/merge.rs"
    "crates/p2p-core/src/bootstrap/mod.rs"
    "crates/p2p-core/src/bootstrap/contact.rs"
    "crates/p2p-core/src/identity/manager.rs"
    "crates/p2p-core/src/mcp.rs"
    "crates/p2p-core/src/dht.rs"
    "crates/p2p-core/src/production.rs"
    "crates/p2p-core/src/security.rs"
    "crates/p2p-core/src/error.rs"
    "crates/p2p-ffi/src/lib.rs"
)

echo "Adding license headers to core files..."

for file in "${CORE_FILES[@]}"; do
    if [ -f "$file" ]; then
        if ! head -n 15 "$file" | grep -q "Copyright.*MaidSafe Limited"; then
            add_license_header "$file"
        else
            echo "✅ $file already has license header"
        fi
    else
        echo "⚠️  File not found: $file"
    fi
done

echo "License headers updated for core files!"