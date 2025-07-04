
//! Enhanced logging utilities for test suite
//!
//! Provides structured logging with correlation IDs, performance metrics,
//! and specialized formatting for test results and data verification.

use colored::*;
use std::fmt;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Test execution context with correlation tracking
#[derive(Debug, Clone)]
pub struct TestContext {
    /// Unique correlation ID for this test run
    pub correlation_id: String,
    
    /// Test name or identifier
    pub test_name: String,
    
    /// Start time of the test
    pub start_time: Instant,
    
    /// Additional context metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl TestContext {
    pub fn new(test_name: impl Into<String>) -> Self {
        Self {
            correlation_id: Uuid::new_v4().to_string(),
            test_name: test_name.into(),
            start_time: Instant::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn log_start(&self) {
        info!(
            correlation_id = %self.correlation_id,
            test_name = %self.test_name,
            "🚀 Test started"
        );
    }

    pub fn log_success(&self, message: Option<&str>) {
        let duration = self.elapsed();
        info!(
            correlation_id = %self.correlation_id,
            test_name = %self.test_name,
            duration_ms = duration.as_millis(),
            "✅ Test completed successfully{}",
            message.map_or(String::new(), |m| format!(": {}", m))
        );
    }

    pub fn log_failure(&self, error: &str) {
        let duration = self.elapsed();
        error!(
            correlation_id = %self.correlation_id,
            test_name = %self.test_name,
            duration_ms = duration.as_millis(),
            error = %error,
            "❌ Test failed"
        );
    }

    pub fn log_info(&self, message: &str) {
        info!(
            correlation_id = %self.correlation_id,
            test_name = %self.test_name,
            "ℹ️ {}",
            message
        );
    }

    pub fn log_warning(&self, message: &str) {
        warn!(
            correlation_id = %self.correlation_id,
            test_name = %self.test_name,
            "⚠️ {}",
            message
        );
    }

    pub fn log_error(&self, message: &str) {
        error!(
            correlation_id = %self.correlation_id,
            test_name = %self.test_name,
            "❌ {}",
            message
        );
    }
}

/// Performance metrics tracking
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub start_time: Instant,
    pub operation_count: u64,
    pub bytes_processed: u64,
    pub error_count: u64,
    pub custom_metrics: std::collections::HashMap<String, f64>,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            operation_count: 0,
            bytes_processed: 0,
            error_count: 0,
            custom_metrics: std::collections::HashMap::new(),
        }
    }

    pub fn record_operation(&mut self) {
        self.operation_count += 1;
    }

    pub fn record_bytes(&mut self, bytes: u64) {
        self.bytes_processed += bytes;
    }

    pub fn record_error(&mut self) {
        self.error_count += 1;
    }

    pub fn record_custom_metric(&mut self, name: String, value: f64) {
        self.custom_metrics.insert(name, value);
    }

    pub fn duration(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn operations_per_second(&self) -> f64 {
        let duration_secs = self.duration().as_secs_f64();
        if duration_secs > 0.0 {
            self.operation_count as f64 / duration_secs
        } else {
            0.0
        }
    }

    pub fn bytes_per_second(&self) -> f64 {
        let duration_secs = self.duration().as_secs_f64();
        if duration_secs > 0.0 {
            self.bytes_processed as f64 / duration_secs
        } else {
            0.0
        }
    }

    pub fn error_rate(&self) -> f64 {
        if self.operation_count > 0 {
            self.error_count as f64 / self.operation_count as f64
        } else {
            0.0
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "Operations: {} ({:.1}/s), Bytes: {} ({:.1}/s), Errors: {} ({:.1}%), Duration: {:?}",
            self.operation_count,
            self.operations_per_second(),
            self.bytes_processed,
            self.bytes_per_second(),
            self.error_count,
            self.error_rate() * 100.0,
            self.duration()
        )
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Colored console output utilities
pub struct ColoredOutput;

impl ColoredOutput {
    pub fn success(text: &str) -> String {
        format!("{}", text.green().bold())
    }

    pub fn error(text: &str) -> String {
        format!("{}", text.red().bold())
    }

    pub fn warning(text: &str) -> String {
        format!("{}", text.yellow().bold())
    }

    pub fn info(text: &str) -> String {
        format!("{}", text.blue())
    }

    pub fn highlight(text: &str) -> String {
        format!("{}", text.cyan().bold())
    }

    pub fn dim(text: &str) -> String {
        format!("{}", text.dimmed())
    }

    pub fn progress_bar(current: usize, total: usize, width: usize) -> String {
        let percentage = if total > 0 {
            (current * 100) / total
        } else {
            0
        };

        let filled = (current * width) / total.max(1);
        let empty = width - filled;

        format!(
            "[{}{}] {}/{}  ({}%)",
            "█".repeat(filled).green(),
            "░".repeat(empty).dimmed(),
            current,
            total,
            percentage
        )
    }

    pub fn test_result_table(results: &[TestResult]) -> String {
        let mut output = String::new();
        
        output.push_str(&format!("{}\n", "Test Results".cyan().bold()));
        output.push_str(&format!("{}\n", "─".repeat(80).dimmed()));
        
        for result in results {
            let status = if result.success {
                "✅ PASS".green().bold()
            } else {
                "❌ FAIL".red().bold()
            };
            
            output.push_str(&format!(
                "{:<50} {} {:>8}\n",
                result.name.bold(),
                status,
                format!("{:?}", result.duration).dimmed()
            ));
            
            if let Some(error) = &result.error {
                output.push_str(&format!("   {}\n", error.red()));
            }
        }
        
        let passed = results.iter().filter(|r| r.success).count();
        let total = results.len();
        let pass_rate = if total > 0 { (passed * 100) / total } else { 0 };
        
        output.push_str(&format!("{}\n", "─".repeat(80).dimmed()));
        output.push_str(&format!(
            "Summary: {}/{} passed ({}%)\n",
            passed,
            total,
            pass_rate
        ));
        
        output
    }
}

/// Test result for reporting
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub success: bool,
    pub duration: Duration,
    pub error: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl TestResult {
    pub fn success(name: String, duration: Duration) -> Self {
        Self {
            name,
            success: true,
            duration,
            error: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn failure(name: String, duration: Duration, error: String) -> Self {
        Self {
            name,
            success: false,
            duration,
            error: Some(error),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Structured test suite progress reporting
pub struct ProgressReporter {
    total_tests: usize,
    completed_tests: usize,
    passed_tests: usize,
    start_time: Instant,
    current_test: Option<String>,
}

impl ProgressReporter {
    pub fn new(total_tests: usize) -> Self {
        Self {
            total_tests,
            completed_tests: 0,
            passed_tests: 0,
            start_time: Instant::now(),
            current_test: None,
        }
    }

    pub fn start_test(&mut self, test_name: String) {
        self.current_test = Some(test_name.clone());
        println!(
            "{} Starting test: {}",
            ColoredOutput::info("🧪"),
            ColoredOutput::highlight(&test_name)
        );
    }

    pub fn complete_test(&mut self, result: &TestResult) {
        self.completed_tests += 1;
        if result.success {
            self.passed_tests += 1;
        }
        
        let status = if result.success {
            ColoredOutput::success("✅ PASS")
        } else {
            ColoredOutput::error("❌ FAIL")
        };
        
        println!(
            "{} {} ({:?})",
            status,
            result.name,
            result.duration
        );
        
        if let Some(error) = &result.error {
            println!("   {}", ColoredOutput::error(error));
        }
        
        self.print_progress();
        self.current_test = None;
    }

    pub fn print_progress(&self) {
        let progress = ColoredOutput::progress_bar(
            self.completed_tests,
            self.total_tests,
            40
        );
        
        let elapsed = self.start_time.elapsed();
        let eta = if self.completed_tests > 0 {
            let avg_time = elapsed / self.completed_tests as u32;
            let remaining = self.total_tests - self.completed_tests;
            avg_time * remaining as u32
        } else {
            Duration::ZERO
        };
        
        println!(
            "{} ETA: {:?} Elapsed: {:?}",
            progress,
            eta,
            elapsed
        );
    }

    pub fn print_summary(&self) {
        let pass_rate = if self.total_tests > 0 {
            (self.passed_tests * 100) / self.total_tests
        } else {
            0
        };
        
        println!("\n{}", "=".repeat(80).cyan());
        println!(
            "{} Test Suite Complete",
            ColoredOutput::highlight("🎉")
        );
        println!("{}", "=".repeat(80).cyan());
        
        println!(
            "Total Tests: {}, Passed: {}, Failed: {}, Pass Rate: {}%",
            self.total_tests,
            self.passed_tests,
            self.total_tests - self.passed_tests,
            pass_rate
        );
        
        println!("Total Duration: {:?}", self.start_time.elapsed());
        
        if self.passed_tests == self.total_tests {
            println!("{}", ColoredOutput::success("🎊 All tests passed!"));
        } else {
            println!(
                "{}",
                ColoredOutput::warning(&format!(
                    "⚠️  {} test(s) failed",
                    self.total_tests - self.passed_tests
                ))
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics() {
        let mut metrics = PerformanceMetrics::new();
        
        metrics.record_operation();
        metrics.record_bytes(1024);
        metrics.record_custom_metric("custom".to_string(), 42.0);
        
        assert_eq!(metrics.operation_count, 1);
        assert_eq!(metrics.bytes_processed, 1024);
        assert_eq!(metrics.custom_metrics.get("custom"), Some(&42.0));
    }

    #[test]
    fn test_test_context() {
        let ctx = TestContext::new("test_example")
            .with_metadata("key", "value");
        
        assert_eq!(ctx.test_name, "test_example");
        assert_eq!(ctx.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_progress_reporter() {
        let mut reporter = ProgressReporter::new(3);
        
        reporter.start_test("test1".to_string());
        let result = TestResult::success("test1".to_string(), Duration::from_millis(100));
        reporter.complete_test(&result);
        
        assert_eq!(reporter.completed_tests, 1);
        assert_eq!(reporter.passed_tests, 1);
    }
}