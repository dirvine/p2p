// Copyright 2025 Saorsa Labs Limited
//
// Communitas CLI Library - Personal AI Assistant
//
// This library provides the core functionality for the Communitas CLI,
// a comprehensive personal AI assistant with advanced capabilities.

pub mod config;
pub mod chat;
pub mod file;
pub mod tui;
pub mod voice;
pub mod network;
pub mod communication;
pub mod identity;

// Re-export commonly used types
pub use config::{ConfigManager, ConfigError};
pub use chat::{ChatEngine, AIModel, Message, Role, ChatOptions, SessionId};
pub use file::{FileProcessor, ContentExtractor, ExtractedContent, ProcessingOptions, FileProcessingError};

/// Result type alias for convenience
pub type Result<T> = anyhow::Result<T>;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_defined() {
        assert!(!VERSION.is_empty());
        assert!(VERSION.contains('.'));
    }
}