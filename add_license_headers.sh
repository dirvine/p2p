#\!/bin/bash

# License header to add
LICENSE_HEADER="// Copyright (c) 2025 Saorsa Labs Limited

// This file is part of the Saorsa P2P network.

// Licensed under the AGPL-3.0 license:
// <https://www.gnu.org/licenses/agpl-3.0.html>

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
"

# Function to add license header to a file
add_header() {
    local file="$1"
    
    # Check if file already has a copyright notice
    if head -n 15 "$file" | grep -q "Copyright.*Saorsa Labs Limited"; then
        echo "✓ Already has header: $file"
        return
    fi
    
    # Create temp file with header + original content
    echo "$LICENSE_HEADER" > "$file.tmp"
    echo "" >> "$file.tmp"
    cat "$file" >> "$file.tmp"
    
    # Replace original file
    mv "$file.tmp" "$file"
    echo "✓ Added header to: $file"
}

# Process all Rust files
echo "Adding license headers to Rust files..."
find . -name "*.rs" -not -path "./target/*" -not -path "./node_modules/*" | while read -r file; do
    add_header "$file"
done

echo "Done\! License headers added to all Rust files."
