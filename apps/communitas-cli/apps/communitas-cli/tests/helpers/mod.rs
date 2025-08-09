// Copyright 2025 Saorsa Labs Limited
// Test helpers and utilities for Communitas CLI tests

use std::path::PathBuf;
use tempfile::{TempDir, NamedTempFile};
use std::io::Write;

/// Get the path to test fixtures directory
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Create a temporary file with given content
pub fn create_temp_file(name: &str, content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::with_suffix(name).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file
}

/// Create a temporary directory with test configuration
pub fn create_test_config_dir() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    
    let test_config = r#"
[api]
openai_key = "test-key-12345"
anthropic_key = "test-ant-key-67890"
default_model = "gpt-4"
temperature = 0.7

[ui]
theme = "dark"
auto_save = true
history_limit = 1000

[network]
p2p_enabled = true
listen_port = 9000
bootstrap_nodes = ["/ip4/127.0.0.1/tcp/9000"]

[voice]
enabled = true
language = "en"
voice_activation = false
wake_word = "communitas"

[file]
auto_process = false
max_file_size = "100MB"
output_directory = "~/Documents/communitas_output"
supported_formats = ["pdf", "txt", "md", "jpg", "png"]

[privacy]
telemetry_enabled = false
crash_reporting = false
data_retention_days = 90
"#;
    
    std::fs::write(&config_path, test_config).unwrap();
    temp_dir
}

/// Create a test PDF file with given content (mock)
pub fn create_test_pdf(content: &str) -> NamedTempFile {
    // This is a mock PDF - in a real implementation, you'd generate actual PDF bytes
    let pdf_header = b"%PDF-1.4\n";
    let mut file = NamedTempFile::with_suffix(".pdf").unwrap();
    file.write_all(pdf_header).unwrap();
    file.write_all(b"Mock PDF content: ").unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file
}

/// Create a test image file with embedded text for OCR testing (mock)
pub fn create_test_image_with_text(text: &str) -> NamedTempFile {
    // This is a mock image - in a real implementation, you'd generate actual image bytes
    let png_header = b"\x89PNG\x0d\x0a\x1a\x0a";
    let mut file = NamedTempFile::with_suffix(".png").unwrap();
    file.write_all(png_header).unwrap();
    file.write_all(b"Mock PNG with text: ").unwrap();
    file.write_all(text.as_bytes()).unwrap();
    file
}

/// Create a test audio file (mock)
pub fn create_test_audio(content: &str) -> NamedTempFile {
    // This is a mock WAV file - in a real implementation, you'd generate actual audio bytes
    let wav_header = b"RIFF\x24\x00\x00\x00WAVEfmt ";
    let mut file = NamedTempFile::with_suffix(".wav").unwrap();
    file.write_all(wav_header).unwrap();
    file.write_all(b"Mock audio: ").unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file
}

/// Create multiple test files for batch processing tests
pub fn create_test_file_batch(count: usize) -> (TempDir, Vec<PathBuf>) {
    let temp_dir = TempDir::new().unwrap();
    let mut files = Vec::new();
    
    for i in 0..count {
        let file_path = temp_dir.path().join(format!("test_file_{}.txt", i));
        let content = format!("This is test file number {}. It contains sample content for processing.", i);
        std::fs::write(&file_path, content).unwrap();
        files.push(file_path);
    }
    
    (temp_dir, files)
}

/// Mock environment setup for testing
pub struct TestEnvironment {
    pub config_dir: TempDir,
    pub data_dir: TempDir,
}

impl TestEnvironment {
    pub fn new() -> Self {
        let config_dir = create_test_config_dir();
        let data_dir = TempDir::new().unwrap();
        
        Self {
            config_dir,
            data_dir,
        }
    }
    
    pub fn config_path(&self) -> PathBuf {
        self.config_dir.path().join("config.toml")
    }
    
    pub fn data_path(&self) -> PathBuf {
        self.data_dir.path().to_path_buf()
    }
}

/// Sleep for a short duration (useful for timing tests)
pub async fn short_delay() {
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
}

/// Sleep for a medium duration
pub async fn medium_delay() {
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
}

/// Create a test file with specific size (for size limit testing)
pub fn create_file_with_size(size_bytes: usize) -> NamedTempFile {
    let content = "x".repeat(size_bytes);
    create_temp_file(".txt", &content)
}

/// Mock AI response for testing
pub fn mock_ai_response(prompt: &str) -> String {
    match prompt.to_lowercase().as_str() {
        p if p.contains("hello") => "Hello! How can I help you today?".to_string(),
        p if p.contains("2+2") => "2+2 equals 4.".to_string(),
        p if p.contains("weather") => "I don't have access to current weather data.".to_string(),
        p if p.contains("summarize") => "Summary: This document contains important information about the topic.".to_string(),
        _ => "I understand your request and I'm here to help.".to_string(),
    }
}

/// Verify that a string contains expected content (case-insensitive)
pub fn contains_any(text: &str, expected: &[&str]) -> bool {
    let text_lower = text.to_lowercase();
    expected.iter().any(|&exp| text_lower.contains(&exp.to_lowercase()))
}

/// Extract file extension from path
pub fn get_file_extension(path: &std::path::Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase()
}

/// Calculate approximate token count for text (rough estimation)
pub fn estimate_tokens(text: &str) -> usize {
    // Very rough estimation: ~4 characters per token
    text.len() / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixtures_dir_exists() {
        let fixtures = fixtures_dir();
        // Directory might not exist in CI, but path should be correct
        assert!(fixtures.ends_with("tests/fixtures"));
    }

    #[test]
    fn test_create_temp_file() {
        let file = create_temp_file(".txt", "test content");
        let content = std::fs::read_to_string(file.path()).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_test_environment() {
        let env = TestEnvironment::new();
        assert!(env.config_path().exists());
        assert!(env.data_path().exists());
    }

    #[test]
    fn test_mock_ai_response() {
        assert_eq!(mock_ai_response("hello"), "Hello! How can I help you today?");
        assert_eq!(mock_ai_response("what is 2+2?"), "2+2 equals 4.");
        assert!(mock_ai_response("unknown").contains("understand"));
    }

    #[test]
    fn test_contains_any() {
        assert!(contains_any("Hello World", &["hello", "world"]));
        assert!(contains_any("TEST CONTENT", &["test", "missing"]));
        assert!(!contains_any("Hello World", &["missing", "absent"]));
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("test"), 1);
        assert_eq!(estimate_tokens("hello world"), 2);
    }
}