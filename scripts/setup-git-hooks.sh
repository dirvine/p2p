#!/bin/bash
# Setup git hooks for license compliance

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "🔧 Setting up git hooks for P2P Foundation..."

# Create hooks directory if it doesn't exist
mkdir -p "$PROJECT_ROOT/.git/hooks"

# Install pre-commit hook
cat > "$PROJECT_ROOT/.git/hooks/pre-commit" << 'EOF'
#!/bin/bash
# Git pre-commit hook for P2P Foundation

# Run license compliance checks
if [ -x "scripts/pre-commit-license-check.sh" ]; then
    ./scripts/pre-commit-license-check.sh || exit 1
fi

# Run Rust formatting check
if command -v cargo fmt -- --check >/dev/null 2>&1; then
    echo "🎨 Checking Rust formatting..."
    cargo fmt -- --check || {
        echo "❌ Rust formatting issues found. Run 'cargo fmt' to fix."
        exit 1
    }
fi

# Run clippy if available
if command -v cargo clippy >/dev/null 2>&1; then
    echo "📎 Running clippy..."
    cargo clippy --all-targets -- -D warnings || {
        echo "❌ Clippy warnings found. Please fix before committing."
        exit 1
    }
fi

echo "✅ All pre-commit checks passed!"
EOF

chmod +x "$PROJECT_ROOT/.git/hooks/pre-commit"

echo "✅ Git hooks installed successfully!"
echo ""
echo "The following checks will run before each commit:"
echo "  - License header verification"
echo "  - Sensitive information detection"
echo "  - Rust formatting (cargo fmt)"
echo "  - Clippy linting"
echo ""
echo "To skip hooks temporarily, use: git commit --no-verify"