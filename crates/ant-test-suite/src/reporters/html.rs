
//! HTML output reporter

use anyhow::Result;
use crate::reporters::TestReporter;
use crate::utils::VerificationResult;
use std::path::Path;
use std::time::SystemTime;

pub struct HtmlReporter;

impl HtmlReporter {
    pub fn new() -> Self {
        Self
    }
}

impl TestReporter for HtmlReporter {
    fn generate_report(&self, results: &[VerificationResult]) -> Result<String> {
        let total = results.len();
        let passed = results.iter().filter(|r| r.success).count();
        let failed = total - passed;
        let success_rate = if total > 0 { (passed * 100) / total } else { 0 };
        
        let mut html = String::new();
        
        // HTML header
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<meta charset='UTF-8'>\n");
        html.push_str("<title>Ant Network Test Suite Results</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: Arial, sans-serif; margin: 40px; }\n");
        html.push_str(".header { background: #f5f5f5; padding: 20px; border-radius: 8px; }\n");
        html.push_str(".summary { margin: 20px 0; }\n");
        html.push_str(".pass { color: #28a745; }\n");
        html.push_str(".fail { color: #dc3545; }\n");
        html.push_str(".test-result { margin: 10px 0; padding: 10px; border-left: 4px solid #ddd; }\n");
        html.push_str(".test-result.pass { border-left-color: #28a745; }\n");
        html.push_str(".test-result.fail { border-left-color: #dc3545; }\n");
        html.push_str(".error { color: #dc3545; font-style: italic; margin-top: 5px; }\n");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");
        
        // Header
        html.push_str("<div class='header'>\n");
        html.push_str("<h1>🐜 Ant Network Test Suite Results</h1>\n");
        html.push_str(&format!("<p>Generated: {}</p>\n", 
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| format!("{}", d.as_secs()))
                .unwrap_or_else(|_| "Unknown".to_string())
        ));
        html.push_str("</div>\n");
        
        // Summary
        html.push_str("<div class='summary'>\n");
        html.push_str("<h2>Summary</h2>\n");
        html.push_str(&format!("<p><strong>Total:</strong> {}</p>\n", total));
        html.push_str(&format!("<p><strong class='pass'>Passed:</strong> {}</p>\n", passed));
        html.push_str(&format!("<p><strong class='fail'>Failed:</strong> {}</p>\n", failed));
        html.push_str(&format!("<p><strong>Success Rate:</strong> {}%</p>\n", success_rate));
        html.push_str("</div>\n");
        
        // Results
        html.push_str("<h2>Test Results</h2>\n");
        for result in results {
            let status_class = if result.success { "pass" } else { "fail" };
            let status_icon = if result.success { "✅" } else { "❌" };
            let default_name = "Unknown".to_string();
            let test_name = result.metadata.get("test_name").unwrap_or(&default_name);
            
            html.push_str(&format!("<div class='test-result {}'>\n", status_class));
            html.push_str(&format!("<strong>{} {}</strong> ({:?})\n", status_icon, test_name, result.duration));
            
            if let Some(error) = &result.error {
                html.push_str(&format!("<div class='error'>{}</div>\n", error));
            }
            
            html.push_str("</div>\n");
        }
        
        html.push_str("</body>\n</html>");
        
        Ok(html)
    }
    
    fn save_report(&self, results: &[VerificationResult], output_path: &Path) -> Result<()> {
        let report = self.generate_report(results)?;
        std::fs::write(output_path, report)?;
        Ok(())
    }
}