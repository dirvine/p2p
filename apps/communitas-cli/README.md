# Communitas CLI

Command-line interface for the Communitas personal AI assistant.

## Features

- 🤖 Interactive AI chat with multiple model support
- 🎤 Voice input/output capabilities  
- 👁️ Vision support for image analysis
- 📁 File processing (PDFs, images, documents)
- 🌐 P2P network connectivity
- 🎨 Beautiful TUI interface
- 🔐 Secure configuration management

## Installation

```bash
# From the workspace root
cargo build --release --bin communitas

# Or install globally
cargo install --path apps/communitas-cli
```

## Usage

### Interactive Chat
```bash
# Basic chat
communitas chat

# With voice I/O
communitas chat --voice

# With vision capabilities
communitas chat --vision

# Using specific model
communitas chat --model claude-3
```

### File Processing
```bash
# Process a PDF
communitas process document.pdf --instruction "Summarize this document"

# Analyze an image
communitas process image.jpg --instruction "Describe what you see"
```

### TUI Interface
```bash
# Launch beautiful terminal UI
communitas tui

# With light theme
communitas tui --theme light
```

### P2P Network
```bash
# Connect to network
communitas connect --port 9000

# With bootstrap node
communitas connect --bootstrap /ip4/1.2.3.4/tcp/9000
```

### Configuration
```bash
# Set API key
communitas config --api-key OPENAI_API_KEY

# Show configuration
communitas config --show
```

## Configuration File

Configuration is stored in `~/.config/communitas/config.toml`:

```toml
[api]
openai_key = "sk-..."
anthropic_key = "sk-ant-..."

[network]
default_port = 9000
bootstrap_nodes = [
    "/ip4/1.2.3.4/tcp/9000",
    "/ip6/::1/tcp/9000"
]

[ui]
theme = "dark"
voice_enabled = false
vision_enabled = false

[models]
default = "gpt-4"
temperature = 0.7
```

## Development

```bash
# Run in development
cargo run --bin communitas -- chat

# Run tests
cargo test -p communitas-cli

# Enable all features
cargo run --features "tui,voice" --bin communitas
```

## Architecture

The CLI is built with:
- **Clap** for command parsing
- **Tokio** for async runtime
- **Ratatui** for TUI interface
- **Whisper** for voice transcription
- **Rodio** for audio playback
- **Saorsa Core** for P2P networking

## License

AGPL-3.0-or-later