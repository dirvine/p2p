// Copyright 2024 MaidSafe Limited
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

//! Console output reporter

use anyhow::Result;
use crate::reporters::TestReporter;
use crate::utils::{VerificationResult, ColoredOutput};
use std::path::Path;

pub struct ConsoleReporter;

impl ConsoleReporter {
    pub fn new() -> Self {
        Self
    }
}

impl TestReporter for ConsoleReporter {
    fn generate_report(&self, results: &[VerificationResult]) -> Result<String> {
        let mut output = String::new();
        
        output.push_str(&ColoredOutput::highlight("📊 Test Results Summary\n"));
        output.push_str(&format!("{}\n", "=".repeat(60)));
        
        let total = results.len();
        let passed = results.iter().filter(|r| r.success).count();
        let failed = total - passed;
        
        for result in results {
            let status = if result.success {
                ColoredOutput::success("✅ PASS")
            } else {
                ColoredOutput::error("❌ FAIL")
            };
            
            output.push_str(&format!(
                "{} {} ({:?})\n",
                status,
                result.metadata.get("test_name").unwrap_or(&"Unknown".to_string()),
                result.duration
            ));
            
            if let Some(error) = &result.error {
                output.push_str(&format!("   {}\n", ColoredOutput::error(error)));
            }
        }
        
        output.push_str(&format!("{}\n", "=".repeat(60)));
        output.push_str(&format!(
            "Summary: {}/{} passed ({}%), {} failed\n",
            passed,
            total,
            if total > 0 { (passed * 100) / total } else { 0 },
            failed
        ));
        
        Ok(output)
    }
    
    fn save_report(&self, results: &[VerificationResult], output_path: &Path) -> Result<()> {
        let report = self.generate_report(results)?;
        std::fs::write(output_path, report)?;
        Ok(())
    }
}