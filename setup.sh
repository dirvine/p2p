#!/bin/bash

# P2P Fastlane Setup Script
# This script sets up the development environment for the P2P monorepo

set -e  # Exit on any error

echo "🚀 Setting up P2P Development Environment..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running on macOS (required for iOS development)
if [[ "$OSTYPE" == "darwin"* ]]; then
    PLATFORM="macos"
    print_status "Detected macOS - iOS development available"
else
    PLATFORM="linux"
    print_warning "Detected Linux - iOS development not available"
fi

# Check for required tools
print_status "Checking prerequisites..."

# Check Ruby
if command -v ruby &> /dev/null; then
    RUBY_VERSION=$(ruby --version)
    print_success "Ruby found: $RUBY_VERSION"
else
    print_error "Ruby not found. Install with: brew install ruby (macOS) or apt install ruby (Linux)"
    exit 1
fi

# Check Bundler
if command -v bundle &> /dev/null; then
    print_success "Bundler found"
else
    print_status "Installing Bundler..."
    gem install bundler
fi

# Check Flutter
if command -v flutter &> /dev/null; then
    FLUTTER_VERSION=$(flutter --version | head -n 1)
    print_success "Flutter found: $FLUTTER_VERSION"
else
    print_error "Flutter not found. Please install Flutter from https://flutter.dev/docs/get-started/install"
    exit 1
fi

# Check Rust
if command -v cargo &> /dev/null; then
    RUST_VERSION=$(rustc --version)
    print_success "Rust found: $RUST_VERSION"
else
    print_error "Rust not found. Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check Git
if command -v git &> /dev/null; then
    print_success "Git found"
else
    print_error "Git not found. Please install Git"
    exit 1
fi

# macOS specific checks
if [[ "$PLATFORM" == "macos" ]]; then
    # Check Xcode
    if command -v xcodebuild &> /dev/null; then
        XCODE_VERSION=$(xcodebuild -version | head -n 1)
        print_success "Xcode found: $XCODE_VERSION"
    else
        print_warning "Xcode not found. Install from Mac App Store for iOS development"
    fi
    
    # Check CocoaPods
    if command -v pod &> /dev/null; then
        print_success "CocoaPods found"
    else
        print_status "Installing CocoaPods..."
        sudo gem install cocoapods
    fi
fi

print_status "Installing Ruby dependencies..."
bundle install

print_status "Installing Rust targets for mobile development..."

# iOS targets (only on macOS)
if [[ "$PLATFORM" == "macos" ]]; then
    rustup target add aarch64-apple-ios x86_64-apple-ios
    print_success "iOS Rust targets installed"
fi

# Android targets
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
print_success "Android Rust targets installed"

# Install cross for Android builds
if ! command -v cross &> /dev/null; then
    print_status "Installing cross for Android compilation..."
    cargo install cross
fi

# Install cargo-lipo for iOS universal libraries (macOS only)
if [[ "$PLATFORM" == "macos" ]] && ! command -v cargo-lipo &> /dev/null; then
    print_status "Installing cargo-lipo for iOS compilation..."
    cargo install cargo-lipo
fi

print_status "Running Flutter doctor..."
flutter doctor

print_status "Setting up Flutter apps..."
for app_dir in apps/*/; do
    if [ -f "$app_dir/pubspec.yaml" ]; then
        app_name=$(basename "$app_dir")
        print_status "Setting up $app_name..."
        cd "$app_dir"
        flutter pub get
        cd ../..
        print_success "$app_name setup complete"
    fi
done

print_status "Testing Rust build..."
cargo build --release

print_success "🎉 Setup complete!"
echo
echo "📋 Next steps:"
echo "1. Configure your Apple Developer account in fastlane/Appfile (iOS only)"
echo "2. Set up certificate management: fastlane match init (iOS only)"
echo "3. Configure Android keystore (Android only)"
echo "4. Test with: fastlane build_dev app:ant-connect"
echo
echo "📚 See fastlane/README.md for detailed instructions"
echo
echo "🚀 Happy coding!"