//! JSON output reporter

use anyhow::Result;
use crate::reporters::TestReporter;
use crate::utils::VerificationResult;
use serde_json::{json, Value};
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