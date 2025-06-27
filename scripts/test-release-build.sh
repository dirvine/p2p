#!/bin/bash

# Test release build locally before pushing to GitHub
# This script simulates the GitHub Actions build process

set -e

echo "🔨 Testing Tauri Release Build Locally"
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Detect OS
OS="unknown"
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    OS="windows"
fi

echo "Detected OS: $OS"

# Check prerequisites
echo -e "\n${YELLOW}Checking prerequisites...${NC}"

# Check Node.js
if ! command -v node &> /dev/null; then
    echo -e "${RED}❌ Node.js is not installed${NC}"
    exit 1
else
    echo -e "${GREEN}✓ Node.js $(node --version)${NC}"
fi

# Check npm/yarn/pnpm
if command -v npm &> /dev/null; then
    echo -e "${GREEN}✓ npm $(npm --version)${NC}"
    PACKAGE_MANAGER="npm"
elif command -v yarn &> /dev/null; then
    echo -e "${GREEN}✓ yarn $(yarn --version)${NC}"
    PACKAGE_MANAGER="yarn"
elif command -v pnpm &> /dev/null; then
    echo -e "${GREEN}✓ pnpm $(pnpm --version)${NC}"
    PACKAGE_MANAGER="pnpm"
else
    echo -e "${RED}❌ No package manager found (npm/yarn/pnpm)${NC}"
    exit 1
fi

# Check Rust
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Rust is not installed${NC}"
    exit 1
else
    echo -e "${GREEN}✓ Rust $(rustc --version | cut -d' ' -f2)${NC}"
fi

# Check Tauri CLI
if ! command -v cargo-tauri &> /dev/null; then
    echo -e "${YELLOW}⚠️  Tauri CLI not found, installing...${NC}"
    cargo install tauri-cli
fi

# Platform-specific checks
if [[ "$OS" == "linux" ]]; then
    echo -e "\n${YELLOW}Checking Linux dependencies...${NC}"
    
    deps=("libwebkit2gtk-4.0-dev" "libappindicator3-dev" "librsvg2-dev" "patchelf")
    missing_deps=()
    
    for dep in "${deps[@]}"; do
        if ! dpkg -l | grep -q "^ii  $dep"; then
            missing_deps+=("$dep")
        fi
    done
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        echo -e "${RED}❌ Missing dependencies: ${missing_deps[*]}${NC}"
        echo "Run: sudo apt-get install -y ${missing_deps[*]}"
        exit 1
    else
        echo -e "${GREEN}✓ All Linux dependencies installed${NC}"
    fi
fi

# Navigate to Tauri project
cd "$(dirname "$0")/../apps/desktop-tauri" || exit 1

# Install dependencies
echo -e "\n${YELLOW}Installing frontend dependencies...${NC}"
case $PACKAGE_MANAGER in
    npm)
        npm ci || npm install
        ;;
    yarn)
        yarn install --frozen-lockfile || yarn install
        ;;
    pnpm)
        pnpm install --frozen-lockfile || pnpm install
        ;;
esac

# Build frontend
echo -e "\n${YELLOW}Building frontend...${NC}"
case $PACKAGE_MANAGER in
    npm)
        npm run build
        ;;
    yarn)
        yarn build
        ;;
    pnpm)
        pnpm build
        ;;
esac

# Get version from tauri.conf.json
VERSION=$(grep '"version"' src-tauri/tauri.conf.json | cut -d '"' -f 4 | head -n 1)
echo -e "\n${GREEN}Building Saorsa Desktop v$VERSION${NC}"

# Build Tauri app
echo -e "\n${YELLOW}Building Tauri application...${NC}"

# Platform-specific build args
BUILD_ARGS=""
if [[ "$OS" == "macos" ]]; then
    # Check architecture
    if [[ $(uname -m) == "arm64" ]]; then
        echo "Building for Apple Silicon..."
        BUILD_ARGS="--target aarch64-apple-darwin"
    else
        echo "Building for Intel Mac..."
        BUILD_ARGS="--target x86_64-apple-darwin"
    fi
fi

# Run the build
cargo tauri build $BUILD_ARGS

# Check results
echo -e "\n${GREEN}✨ Build completed successfully!${NC}"
echo -e "\nBuild artifacts location:"

if [[ "$OS" == "macos" ]]; then
    echo "  DMG: src-tauri/target/release/bundle/dmg/"
    echo "  App: src-tauri/target/release/bundle/macos/"
elif [[ "$OS" == "linux" ]]; then
    echo "  AppImage: src-tauri/target/release/bundle/appimage/"
    echo "  Deb: src-tauri/target/release/bundle/deb/"
elif [[ "$OS" == "windows" ]]; then
    echo "  MSI: src-tauri/target/release/bundle/msi/"
    echo "  EXE: src-tauri/target/release/bundle/nsis/"
fi

echo -e "\n${YELLOW}Next steps:${NC}"
echo "1. Test the built application locally"
echo "2. Commit your changes"
echo "3. Push to the 'release' branch to trigger GitHub Actions"
echo "   git push origin HEAD:release"

# Optional: Open bundle directory
read -p "Open bundle directory? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    if [[ "$OS" == "macos" ]]; then
        open src-tauri/target/release/bundle/
    elif [[ "$OS" == "linux" ]]; then
        xdg-open src-tauri/target/release/bundle/
    elif [[ "$OS" == "windows" ]]; then
        explorer src-tauri/target/release/bundle/
    fi
fi