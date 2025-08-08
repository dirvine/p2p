//! Integration Test Runner
//!
//! Provides utilities for running and managing comprehensive integration tests.

use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::process::Command;

/// Test suite configuration
#[derive(Debug, Clone)]
pub struct TestSuiteConfig {
    pub parallel_execution: bool,
    pub timeout_seconds: u64,
    pub retry_count: u32,
    pub log_level: String,
    pub performance_benchmarks: bool,
}

impl Default for TestSuiteConfig {
    fn default() -> Self {
        Self {
            parallel_execution: true,
            timeout_seconds: 300, // 5 minutes per test
            retry_count: 2,
            log_level: "info".to_string(),
            performance_benchmarks: true,
        }
    }
}

/// Test result information
#[derive(Debug)]
pub struct TestResult {
    pub name: String,
    pub success: bool,
    pub duration: Duration,
    pub error_message: Option<String>,
    pub performance_metrics: HashMap<String, f64>,
}

/// Integration test suite runner
pub struct IntegrationTestRunner {
    config: TestSuiteConfig,
    results: Vec<TestResult>,
}

impl IntegrationTestRunner {
    pub fn new(config: TestSuiteConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
        }
    }

    /// Run all integration tests
    pub async fn run_all_tests(&mut self) -> Result<bool> {
        println!("🚀 Starting Comprehensive Integration Test Suite");
        println!("=" * 60);

        let test_categories = vec![
            ("Network Integration", self.run_network_tests()),
            ("Storage Integration", self.run_storage_tests()),
            ("Security Integration", self.run_security_tests()),
            ("End-to-End Scenarios", self.run_e2e_tests()),
        ];

        let mut all_passed = true;
        let overall_start = Instant::now();

        for (category, test_future) in test_categories {
            println!("\n📋 Running {} Tests...", category);
            println!("-" * 40);

            let category_start = Instant::now();
            let category_passed = test_future.await?;
            let category_duration = category_start.elapsed();

            if category_passed {
                println!(
                    "✅ {} tests PASSED ({:.2}s)",
                    category,
                    category_duration.as_secs_f64()
                );
            } else {
                println!(
                    "❌ {} tests FAILED ({:.2}s)",
                    category,
                    category_duration.as_secs_f64()
                );
                all_passed = false;
            }
        }

        let total_duration = overall_start.elapsed();

        println!("\n" + "=" * 60);
        if all_passed {
            println!("🎉 ALL INTEGRATION TESTS PASSED!");
        } else {
            println!("💥 SOME INTEGRATION TESTS FAILED!");
        }
        println!("Total execution time: {:.2}s", total_duration.as_secs_f64());

        self.generate_test_report().await?;

        Ok(all_passed)
    }

    async fn run_network_tests(&mut self) -> Result<bool> {
        let network_tests = vec![
            "test_multi_node_startup_and_discovery",
            "test_network_partition_and_healing",
            "test_peer_discovery_under_load",
            "test_connection_failure_recovery",
            "test_high_throughput_messaging",
        ];

        self.run_test_category("network_integration_comprehensive_test", &network_tests)
            .await
    }

    async fn run_storage_tests(&mut self) -> Result<bool> {
        let storage_tests = vec![
            "test_basic_store_and_retrieve",
            "test_replication_factor_compliance",
            "test_storage_consistency_under_load",
            "test_storage_recovery_after_node_failure",
            "test_large_data_storage_and_retrieval",
            "test_storage_space_management",
            "test_concurrent_read_write_performance",
        ];

        self.run_test_category("storage_integration_comprehensive_test", &storage_tests)
            .await
    }

    async fn run_security_tests(&mut self) -> Result<bool> {
        let security_tests = vec![
            "test_secure_node_authentication",
            "test_message_encryption_integrity",
            "test_attack_resistance",
            "test_access_control_enforcement",
            "test_crypto_performance",
            "test_identity_verification",
            "test_secure_group_communication",
        ];

        self.run_test_category("security_integration_comprehensive_test", &security_tests)
            .await
    }

    async fn run_e2e_tests(&mut self) -> Result<bool> {
        let e2e_tests = vec![
            "test_complete_social_network_scenario",
            "test_file_sharing_workflow",
            "test_network_performance_benchmarks",
            "test_high_load_stress_scenario",
            "test_network_resilience_scenario",
            "test_real_world_usage_simulation",
        ];

        self.run_test_category("end_to_end_scenarios_test", &e2e_tests)
            .await
    }

    async fn run_test_category(&mut self, test_file: &str, tests: &[&str]) -> Result<bool> {
        let mut category_passed = true;

        for test_name in tests {
            let result = self.run_single_test(test_file, test_name).await?;

            if result.success {
                println!("  ✅ {} ({:.2}s)", test_name, result.duration.as_secs_f64());
            } else {
                println!("  ❌ {} ({:.2}s)", test_name, result.duration.as_secs_f64());
                if let Some(error) = &result.error_message {
                    println!("     Error: {}", error);
                }
                category_passed = false;
            }

            self.results.push(result);
        }

        Ok(category_passed)
    }

    async fn run_single_test(&self, test_file: &str, test_name: &str) -> Result<TestResult> {
        let start_time = Instant::now();

        let mut cmd = Command::new("cargo");
        cmd.args(&["test", "--test", test_file, test_name]);
        cmd.env("RUST_LOG", &self.config.log_level);
        cmd.env("RUST_BACKTRACE", "1");

        // Set timeout
        let output = tokio::time::timeout(
            Duration::from_secs(self.config.timeout_seconds),
            cmd.output(),
        )
        .await;

        let duration = start_time.elapsed();

        match output {
            Ok(Ok(output)) => {
                let success = output.status.success();
                let error_message = if !success {
                    Some(String::from_utf8_lossy(&output.stderr).to_string())
                } else {
                    None
                };

                Ok(TestResult {
                    name: test_name.to_string(),
                    success,
                    duration,
                    error_message,
                    performance_metrics: HashMap::new(), // TODO: Parse from output
                })
            }
            Ok(Err(e)) => Ok(TestResult {
                name: test_name.to_string(),
                success: false,
                duration,
                error_message: Some(format!("Failed to execute test: {}", e)),
                performance_metrics: HashMap::new(),
            }),
            Err(_) => Ok(TestResult {
                name: test_name.to_string(),
                success: false,
                duration,
                error_message: Some("Test timed out".to_string()),
                performance_metrics: HashMap::new(),
            }),
        }
    }

    async fn generate_test_report(&self) -> Result<()> {
        let report_content = self.format_test_report();

        tokio::fs::write("integration_test_report.md", report_content).await?;
        println!("\n📊 Test report generated: integration_test_report.md");

        Ok(())
    }

    fn format_test_report(&self) -> String {
        let mut report = String::new();

        report.push_str("# Integration Test Report\n\n");
        report.push_str(&format!(
            "Generated: {}\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        let total_tests = self.results.len();
        let passed_tests = self.results.iter().filter(|r| r.success).count();
        let failed_tests = total_tests - passed_tests;

        report.push_str("## Summary\n\n");
        report.push_str(&format!("- **Total Tests**: {}\n", total_tests));
        report.push_str(&format!(
            "- **Passed**: {} ({:.1}%)\n",
            passed_tests,
            (passed_tests as f64 / total_tests as f64) * 100.0
        ));
        report.push_str(&format!(
            "- **Failed**: {} ({:.1}%)\n",
            failed_tests,
            (failed_tests as f64 / total_tests as f64) * 100.0
        ));

        let total_duration: Duration = self.results.iter().map(|r| r.duration).sum();
        report.push_str(&format!(
            "- **Total Duration**: {:.2}s\n",
            total_duration.as_secs_f64()
        ));
        report.push_str(&format!(
            "- **Average per Test**: {:.2}s\n\n",
            total_duration.as_secs_f64() / total_tests as f64
        ));

        report.push_str("## Test Results\n\n");

        for result in &self.results {
            let status = if result.success {
                "✅ PASS"
            } else {
                "❌ FAIL"
            };
            report.push_str(&format!(
                "### {} - {} ({:.2}s)\n\n",
                result.name,
                status,
                result.duration.as_secs_f64()
            ));

            if let Some(error) = &result.error_message {
                report.push_str("**Error:**\n```\n");
                report.push_str(error);
                report.push_str("\n```\n\n");
            }

            if !result.performance_metrics.is_empty() {
                report.push_str("**Performance Metrics:**\n");
                for (metric, value) in &result.performance_metrics {
                    report.push_str(&format!("- {}: {:.2}\n", metric, value));
                }
                report.push_str("\n");
            }
        }

        report.push_str("## Recommendations\n\n");

        if failed_tests > 0 {
            report.push_str("### Failed Tests\n");
            report.push_str(
                "Please review and fix the failed tests before proceeding to production.\n\n",
            );
        }

        if total_duration.as_secs() > 600 {
            // 10 minutes
            report.push_str("### Performance\n");
            report.push_str(
                "Test suite execution time is quite long. Consider optimizing slower tests.\n\n",
            );
        }

        report.push_str("### Next Steps\n");
        report.push_str("1. Address any failing tests\n");
        report.push_str("2. Review performance metrics\n");
        report.push_str("3. Consider additional test scenarios based on requirements\n");
        report.push_str("4. Integrate with CI/CD pipeline\n\n");

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_integration_runner() -> Result<()> {
        let mut config = TestSuiteConfig::default();
        config.timeout_seconds = 60; // Shorter timeout for testing

        let mut runner = IntegrationTestRunner::new(config);

        // This would run all tests in a real scenario
        // For now, just test the runner setup
        assert_eq!(runner.results.len(), 0);

        Ok(())
    }
}
