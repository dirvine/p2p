# Test Fixtures

This directory contains test files and data used by the Communitas CLI test suite.

## File Structure

```
fixtures/
├── documents/          # Sample documents for file processing tests
│   ├── sample.pdf      # Multi-page PDF with text and images
│   ├── report.docx     # Word document with formatting
│   ├── research.txt    # Plain text research paper
│   ├── data.csv        # CSV data file
│   └── config.json     # JSON configuration file
├── images/             # Sample images for vision and OCR testing
│   ├── text_image.png  # Image containing readable text
│   ├── chart.jpg       # Graph/chart image
│   ├── photo.jpg       # Regular photo without text
│   └── screenshot.png  # UI screenshot with text
├── audio/              # Audio files for voice processing tests
│   ├── hello_world.wav # Clear speech sample
│   ├── noisy_speech.wav# Speech with background noise
│   └── music.mp3       # Non-speech audio
├── configs/            # Sample configuration files
│   ├── minimal.toml    # Minimal configuration
│   ├── complete.toml   # Full configuration with all options
│   └── invalid.toml    # Invalid configuration for error testing
└── code/               # Source code files for processing
    ├── example.rs      # Rust source file
    ├── script.py       # Python script
    ├── app.js          # JavaScript application
    └── README.md       # Markdown documentation
```

## Usage in Tests

Test files should be accessed using the `fixtures_dir()` helper function:

```rust
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn test_with_fixture() {
    let pdf_path = fixtures_dir().join("documents/sample.pdf");
    // Use pdf_path in test...
}
```

## File Generation

Some fixture files are generated automatically by test helpers to ensure consistent test data.

For binary files that cannot be stored in git, tests should use the helper functions to create temporary test files:

```rust
// Create test PDF
let pdf_content = create_test_pdf("Sample PDF content for testing");

// Create test image with text
let image_with_text = create_test_image_with_text("Hello World");

// Create test audio file
let audio_file = create_test_audio("Hello, this is a test");
```

## Updating Fixtures

When adding new test cases that require specific file formats or content:

1. Add the fixture file to the appropriate subdirectory
2. Update this README with a description
3. Add helper functions if the fixture requires special handling
4. Ensure the fixture is used in at least one test

## Size Guidelines

- Keep fixture files small (< 1MB each)
- Use representative samples rather than full documents
- Compress images appropriately
- Use short audio samples (< 10 seconds)

This keeps the test suite fast and the repository size manageable.