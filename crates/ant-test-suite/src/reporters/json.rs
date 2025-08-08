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

//! JSON output reporter

use crate::reporters::TestReporter;
use crate::utils::VerificationResult;
use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;
use std::time::SystemTime;

pub struct JsonReporter;

impl JsonReporter {
    pub fn new() -> Self {
        Self
    }
}

impl TestReporter for JsonReporter {
    fn generate_report(&self, results: &[VerificationResult]) -> Result<String> {
        let total = results.len();
        let passed = results.iter().filter(|r| r.success).count();
        let failed = total - passed;

        let report = json!({
            "timestamp": SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs(),
            "summary": {
                "total": total,
                "passed": passed,
                "failed": failed,
                "success_rate": if total > 0 { (passed as f64 / total as f64) * 100.0 } else { 0.0 }
            },
            "results": results.iter().map(|r| {
                json!({
                    "success": r.success,
                    "duration_ms": r.duration.as_millis(),
                    "metadata": r.metadata,
                    "error": r.error,
                    "timestamp": r.timestamp
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                })
            }).collect::<Vec<Value>>()
        });

        Ok(serde_json::to_string_pretty(&report)?)
    }

    fn save_report(&self, results: &[VerificationResult], output_path: &Path) -> Result<()> {
        let report = self.generate_report(results)?;
        std::fs::write(output_path, report)?;
        Ok(())
    }
}
