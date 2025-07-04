
//! Test result reporting modules

pub mod console;
pub mod json;
pub mod html;

use anyhow::Result;
use crate::utils::VerificationResult;
use crate::OutputFormat;
use std::path::Path;

/// Common trait for test result reporters
pub trait TestReporter {
    /// Generate report from verification results
    fn generate_report(&self, results: &[VerificationResult]) -> Result<String>;
    
    /// Save report to file
    fn save_report(&self, results: &[VerificationResult], output_path: &Path) -> Result<()>;
}

/// Create appropriate reporter based on output format
pub fn create_reporter(format: OutputFormat) -> Box<dyn TestReporter> {
    match format {
        OutputFormat::Console => Box::new(console::ConsoleReporter::new()),
        OutputFormat::Json => Box::new(json::JsonReporter::new()),
        OutputFormat::Html => Box::new(html::HtmlReporter::new()),
        OutputFormat::Text => Box::new(console::ConsoleReporter::new()), // Use console for text
    }
}