// Copyright 2024 Saorsa Labs Limited
//
// This software is dual-licensed under:
// - GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later)
// - Commercial License
//
// For AGPL-3.0 license, see LICENSE-AGPL-3.0
// For commercial licensing, contact: saorsalabs@gmail.com
//
// Unless required by applicable law or agreed to in writing, software
// distributed under these licenses is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.

//! Test result reporting modules

pub mod console;
pub mod html;
pub mod json;

use crate::OutputFormat;
use crate::utils::VerificationResult;
use anyhow::Result;
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
