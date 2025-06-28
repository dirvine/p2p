#!/bin/bash
# Pre-commit hook for license compliance checking

set -e

echo "🔍 Running license compliance checks..."

# Check for license headers in new/modified Rust files
check_license_headers() {
    local files_without_headers=()
    
    # Get list of staged Rust files
    for file in $(git diff --cached --name-only --diff-filter=ACM | grep '\.rs$'); do
        if [ -f "$file" ]; then
            # Check for copyright header in first 15 lines
            if ! head -n 15 "$file" | grep -q "Copyright.*MaidSafe Limited"; then
                files_without_headers+=("$file")
            fi
        fi
    done
    
    if [ ${#files_without_headers[@]} -gt 0 ]; then
        echo "❌ The following files are missing license headers:"
        printf '%s\n' "${files_without_headers[@]}"
        echo ""
        echo "Please add the following header to each file:"
        echo ""
        cat << 'EOF'
// Copyright 2024 MaidSafe Limited
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
EOF
        return 1
    fi
    
    echo "✅ All Rust files have proper license headers"
    return 0
}

# Check if commercial features are used without license
check_commercial_features() {
    # Check if any file uses commercial features
    if git diff --cached --name-only | xargs grep -l '#\[cfg(feature.*=.*"commercial"' 2>/dev/null; then
        if [ -z "$P2P_LICENSE_PATH" ] && [ -z "$P2P_LICENSE_KEY" ] && [ ! -f "license.json" ]; then
            echo "⚠️  Warning: Commercial features detected but no license found"
            echo "   Set P2P_LICENSE_PATH or P2P_LICENSE_KEY for commercial builds"
        fi
    fi
}

# Check for sensitive information
check_sensitive_info() {
    local patterns=(
        "PRIVATE KEY"
        "SECRET"
        "PASSWORD"
        "API_KEY"
        "LICENSE_KEY"
        "maidsafe\.net.*key"
    )
    
    for pattern in "${patterns[@]}"; do
        if git diff --cached | grep -i "$pattern" > /dev/null; then
            echo "❌ Potential sensitive information detected!"
            echo "   Pattern found: $pattern"
            echo "   Please review your changes and remove any secrets"
            return 1
        fi
    done
    
    echo "✅ No sensitive information detected"
    return 0
}

# Check Cargo.toml license fields
check_cargo_license() {
    for toml in $(git diff --cached --name-only | grep 'Cargo\.toml$'); do
        if [ -f "$toml" ]; then
            if grep -q '^license\s*=' "$toml"; then
                if ! grep -q 'AGPL-3.0-or-later OR Commercial' "$toml"; then
                    echo "⚠️  Warning: $toml has incorrect license field"
                    echo "   Expected: license = \"AGPL-3.0-or-later OR Commercial\""
                fi
            fi
        fi
    done
}

# Main execution
main() {
    local failed=0
    
    # Run all checks
    check_license_headers || failed=1
    check_commercial_features
    check_sensitive_info || failed=1
    check_cargo_license
    
    if [ $failed -eq 0 ]; then
        echo ""
        echo "✅ All license checks passed!"
        exit 0
    else
        echo ""
        echo "❌ License compliance checks failed!"
        echo "   Please fix the issues above and try again"
        exit 1
    fi
}

# Only run if there are staged files
if git diff --cached --name-only | grep -q .; then
    main
else
    echo "No staged files to check"
    exit 0
fi