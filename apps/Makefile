.PHONY: all build release clean chat tester test help

# Default target
all: release

# Build in debug mode
build:
	@echo "🔨 Building terminal apps in debug mode..."
	cargo build -p saorsa-terminal-chat -p saorsa-network-tester

# Build in release mode with optimizations
release:
	@echo "🔨 Building terminal apps in release mode..."
	@echo "This will optimize for size and performance..."
	cargo build --release -p saorsa-terminal-chat -p saorsa-network-tester
	@echo ""
	@echo "✅ Build complete!"
	@echo "Binaries:"
	@ls -lh ../target/release/saorsa-terminal-chat ../target/release/saorsa-network-tester 2>/dev/null || echo "Build may have failed"

# Build just the chat app
chat:
	cargo build --release -p saorsa-terminal-chat

# Build just the network tester
tester:
	cargo build --release -p saorsa-network-tester

# Run tests
test:
	cargo test -p saorsa-terminal-chat -p saorsa-network-tester

# Clean build artifacts
clean:
	cargo clean

# Run the chat app
run-chat: chat
	../target/release/saorsa-terminal-chat

# Run the network tester
run-tester: tester
	../target/release/saorsa-network-tester

# Create macOS app bundles
mac-apps: release
	./create_macos_apps.sh

# Package for distribution
package: mac-apps
	./package_for_distribution.sh

# Install apps to /Applications
install: mac-apps
	@echo "Installing apps to /Applications..."
	@cp -R "Saorsa Terminal Chat.app" /Applications/ 2>/dev/null && echo "✅ Installed Saorsa Terminal Chat" || echo "❌ Failed to install chat app (need sudo?)"
	@cp -R "Saorsa Network Tester.app" /Applications/ 2>/dev/null && echo "✅ Installed Saorsa Network Tester" || echo "❌ Failed to install tester app (need sudo?)"

# Help
help:
	@echo "Saorsa Terminal Apps - Build Targets"
	@echo "===================================="
	@echo ""
	@echo "  make              - Build both apps in release mode"
	@echo "  make release      - Build both apps in release mode (explicit)"
	@echo "  make build        - Build both apps in debug mode"
	@echo "  make chat         - Build only the chat app"
	@echo "  make tester       - Build only the network tester"
	@echo "  make test         - Run tests"
	@echo "  make clean        - Clean build artifacts"
	@echo "  make run-chat     - Build and run the chat app"
	@echo "  make run-tester   - Build and run the network tester"
	@echo "  make mac-apps     - Create macOS .app bundles"
	@echo "  make package      - Create distribution package (ZIP)"
	@echo "  make install      - Install apps to /Applications"
	@echo "  make help         - Show this help"