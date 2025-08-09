// Copyright 2025 Saorsa Labs Limited
// Tests for file processing system

use anyhow::Result;
use communitas_cli::file::{FileProcessor, ContentExtractor, ExtractedContent, ProcessingOptions, FileProcessingError};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::pin::Pin;
use std::future::Future;
use tempfile::{TempDir, NamedTempFile};

// Mock content extractor for testing
struct MockContentExtractor {
    responses: HashMap<String, ExtractedContent>,
    should_fail: bool,
}

impl MockContentExtractor {
    fn new() -> Self {
        Self {
            responses: HashMap::new(),
            should_fail: false,
        }
    }
    
    fn with_response(mut self, file_extension: &str, content: ExtractedContent) -> Self {
        self.responses.insert(file_extension.to_string(), content);
        self
    }
    
    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }
}

impl ContentExtractor for MockContentExtractor {
    fn extract(&self, path: &Path) -> Pin<Box<dyn Future<Output = Result<ExtractedContent>> + Send + '_>> {
        Box::pin(async move {
            if self.should_fail {
                return Err(anyhow::anyhow!("Mock extraction failure"));
            }
            
            let extension = path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");
                
            self.responses.get(extension)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("No mock response for extension: {}", extension))
        })
    }
    
    fn supported_types(&self) -> &[&str] {
        &["txt", "pdf", "png", "jpg"]
    }
    
    fn can_extract(&self, path: &Path) -> bool {
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        self.supported_types().contains(&extension)
    }
}

fn create_test_file(name: &str, content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::with_suffix(name).unwrap();
    std::io::Write::write_all(&mut file, content.as_bytes()).unwrap();
    file
}

fn create_test_extracted_content(text: &str, mime_type: &str) -> ExtractedContent {
    ExtractedContent {
        text: text.to_string(),
        mime_type: mime_type.to_string(),
        file_size: text.len() as u64,
        page_count: Some(1),
        metadata: HashMap::new(),
        created_at: std::time::SystemTime::now(),
    }
}

#[tokio::test]
async fn test_file_processor_creation() {
    let processor = FileProcessor::new();
    assert_eq!(processor.extractor_count(), 0);
    assert_eq!(processor.max_file_size(), 100 * 1024 * 1024); // 100MB default
}

#[tokio::test]
async fn test_add_content_extractor() {
    let mut processor = FileProcessor::new();
    let mock_extractor = MockContentExtractor::new();
    
    processor.add_extractor("text", Box::new(mock_extractor));
    assert_eq!(processor.extractor_count(), 1);
    assert!(processor.has_extractor("text"));
    assert!(!processor.has_extractor("unknown"));
}

#[tokio::test]
async fn test_text_file_processing() -> Result<()> {
    let test_file = create_test_file(".txt", "Hello, world! This is test content.");
    let expected_content = create_test_extracted_content(
        "Hello, world! This is test content.",
        "text/plain"
    );
    
    let mut processor = FileProcessor::new();
    let mock_extractor = MockContentExtractor::new()
        .with_response("txt", expected_content.clone());
    
    processor.add_extractor("text", Box::new(mock_extractor));
    
    let result = processor.process_file(test_file.path()).await?;
    assert_eq!(result.text, expected_content.text);
    assert_eq!(result.mime_type, expected_content.mime_type);
    assert_eq!(result.file_size, expected_content.file_size);
    
    Ok(())
}

#[tokio::test]
async fn test_pdf_file_processing() -> Result<()> {
    let test_file = create_test_file(".pdf", "mock pdf content");
    let expected_content = create_test_extracted_content(
        "This is extracted PDF content with multiple paragraphs.",
        "application/pdf"
    );
    
    let mut processor = FileProcessor::new();
    let mock_extractor = MockContentExtractor::new()
        .with_response("pdf", expected_content.clone());
    
    processor.add_extractor("pdf", Box::new(mock_extractor));
    
    let result = processor.process_file(test_file.path()).await?;
    assert_eq!(result.text, expected_content.text);
    assert_eq!(result.mime_type, "application/pdf");
    
    Ok(())
}

#[tokio::test]
async fn test_image_file_processing_with_ocr() -> Result<()> {
    let test_file = create_test_file(".png", "mock image data");
    let mut expected_content = create_test_extracted_content(
        "Text extracted from image via OCR",
        "image/png"
    );
    
    // Add image metadata
    expected_content.metadata.insert("width".to_string(), "1920".to_string());
    expected_content.metadata.insert("height".to_string(), "1080".to_string());
    expected_content.metadata.insert("has_text".to_string(), "true".to_string());
    
    let mut processor = FileProcessor::new();
    let mock_extractor = MockContentExtractor::new()
        .with_response("png", expected_content.clone());
    
    processor.add_extractor("image", Box::new(mock_extractor));
    
    let result = processor.process_file(test_file.path()).await?;
    assert_eq!(result.text, "Text extracted from image via OCR");
    assert_eq!(result.metadata.get("width"), Some(&"1920".to_string()));
    assert_eq!(result.metadata.get("height"), Some(&"1080".to_string()));
    
    Ok(())
}

#[tokio::test]
async fn test_batch_file_processing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // Create test files
    let file1_path = temp_dir.path().join("file1.txt");
    let file2_path = temp_dir.path().join("file2.txt");
    let file3_path = temp_dir.path().join("file3.txt");
    
    std::fs::write(&file1_path, "Content of file 1")?;
    std::fs::write(&file2_path, "Content of file 2")?;
    std::fs::write(&file3_path, "Content of file 3")?;
    
    let files = vec![file1_path, file2_path, file3_path];
    
    let mut processor = FileProcessor::new();
    let mock_extractor = MockContentExtractor::new()
        .with_response("txt", create_test_extracted_content("Extracted content", "text/plain"));
    
    processor.add_extractor("text", Box::new(mock_extractor));
    
    let results = processor.process_batch(&files, ProcessingOptions::default()).await?;
    
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.is_ok()));
    
    for (i, result) in results.iter().enumerate() {
        let content = result.as_ref().unwrap();
        assert_eq!(content.text, "Extracted content");
        assert_eq!(content.source_file, Some(files[i].clone()));
    }
    
    Ok(())
}

#[tokio::test]
async fn test_unsupported_file_type() -> Result<()> {
    let test_file = create_test_file(".unknown", "unknown file content");
    let processor = FileProcessor::new();
    
    let result = processor.process_file(test_file.path()).await;
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(matches!(
        error.downcast_ref::<FileProcessingError>(),
        Some(FileProcessingError::UnsupportedFormat(_))
    ));
    
    Ok(())
}

#[tokio::test]
async fn test_file_size_limits() -> Result<()> {
    let processor = FileProcessor::with_max_size(1024); // 1KB limit
    let large_content = "x".repeat(2048); // 2KB content
    let test_file = create_test_file(".txt", &large_content);
    
    let result = processor.process_file(test_file.path()).await;
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(matches!(
        error.downcast_ref::<FileProcessingError>(),
        Some(FileProcessingError::FileTooLarge { .. })
    ));
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_file_processing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut files = Vec::new();
    
    // Create multiple test files
    for i in 0..10 {
        let file_path = temp_dir.path().join(format!("file_{}.txt", i));
        std::fs::write(&file_path, format!("Content of file {}", i))?;
        files.push(file_path);
    }
    
    let mut processor = FileProcessor::with_concurrency(4); // Allow 4 concurrent operations
    let mock_extractor = MockContentExtractor::new()
        .with_response("txt", create_test_extracted_content("Processed content", "text/plain"));
    
    processor.add_extractor("text", Box::new(mock_extractor));
    
    let start_time = std::time::Instant::now();
    let results = processor.process_batch(&files, ProcessingOptions::default()).await?;
    let duration = start_time.elapsed();
    
    assert_eq!(results.len(), 10);
    assert!(results.iter().all(|r| r.is_ok()));
    
    // With concurrency, should complete faster than sequential processing
    assert!(duration < std::time::Duration::from_secs(5));
    
    Ok(())
}

#[tokio::test]
async fn test_processing_options() -> Result<()> {
    let test_file = create_test_file(".txt", "Test content for processing options");
    
    let options = ProcessingOptions {
        extract_metadata: true,
        ocr_enabled: true,
        language: Some("en".to_string()),
        output_format: "json".to_string(),
        custom_instructions: Some("Summarize the content".to_string()),
    };
    
    let mut processor = FileProcessor::new();
    let mock_extractor = MockContentExtractor::new()
        .with_response("txt", create_test_extracted_content("Summarized content", "text/plain"));
    
    processor.add_extractor("text", Box::new(mock_extractor));
    
    let result = processor.process_file_with_options(test_file.path(), options).await?;
    assert_eq!(result.text, "Summarized content");
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling_and_recovery() -> Result<()> {
    let test_file = create_test_file(".txt", "Test content");
    
    let mut processor = FileProcessor::new();
    let failing_extractor = MockContentExtractor::new().with_failure();
    
    processor.add_extractor("text", Box::new(failing_extractor));
    
    let result = processor.process_file(test_file.path()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Mock extraction failure"));
    
    Ok(())
}

#[tokio::test]
async fn test_file_metadata_extraction() -> Result<()> {
    let test_file = create_test_file(".txt", "Test content");
    
    let mut expected_content = create_test_extracted_content("Test content", "text/plain");
    expected_content.metadata.insert("encoding".to_string(), "utf-8".to_string());
    expected_content.metadata.insert("line_count".to_string(), "1".to_string());
    expected_content.metadata.insert("word_count".to_string(), "2".to_string());
    
    let mut processor = FileProcessor::new();
    let mock_extractor = MockContentExtractor::new()
        .with_response("txt", expected_content.clone());
    
    processor.add_extractor("text", Box::new(mock_extractor));
    
    let result = processor.process_file(test_file.path()).await?;
    
    assert_eq!(result.metadata.get("encoding"), Some(&"utf-8".to_string()));
    assert_eq!(result.metadata.get("line_count"), Some(&"1".to_string()));
    assert_eq!(result.metadata.get("word_count"), Some(&"2".to_string()));
    
    Ok(())
}

#[tokio::test]
async fn test_content_transformation() -> Result<()> {
    let test_file = create_test_file(".txt", "Original content for transformation");
    let processor = FileProcessor::new();
    
    let original_content = processor.process_file(test_file.path()).await;
    // This will fail initially as we haven't implemented the processor yet
    
    // Test different transformation formats
    let formats = vec!["json", "markdown", "html", "summary"];
    
    for format in formats {
        let options = ProcessingOptions {
            output_format: format.to_string(),
            ..Default::default()
        };
        
        let result = processor.process_file_with_options(test_file.path(), options).await;
        // Should handle each format appropriately
        assert!(result.is_ok() || format == "unsupported_format");
    }
    
    Ok(())
}

#[tokio::test]
async fn test_progress_reporting() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut files = Vec::new();
    
    // Create test files
    for i in 0..5 {
        let file_path = temp_dir.path().join(format!("file_{}.txt", i));
        std::fs::write(&file_path, format!("Content {}", i))?;
        files.push(file_path);
    }
    
    let mut processor = FileProcessor::new();
    let mut progress_updates = Vec::new();
    
    processor.on_progress(|current, total, file_name| {
        progress_updates.push((current, total, file_name.to_string()));
    });
    
    let mock_extractor = MockContentExtractor::new()
        .with_response("txt", create_test_extracted_content("Processed", "text/plain"));
    
    processor.add_extractor("text", Box::new(mock_extractor));
    
    let _results = processor.process_batch(&files, ProcessingOptions::default()).await?;
    
    // Should have received progress updates
    assert!(progress_updates.len() >= 5);
    assert_eq!(progress_updates.last().unwrap().0, 5); // Last update should be 5/5
    
    Ok(())
}

// This test will fail initially because the FileProcessor doesn't exist yet
#[test]
#[should_panic(expected = "FileProcessor not implemented")]
fn test_file_processor_not_implemented() {
    let _processor = FileProcessor::new();
}