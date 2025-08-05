//! Test harness for property testing and network simulation

#[cfg(feature = "test-harness")]
mod harness;

#[cfg(feature = "test-harness")]
pub use harness::TestHarness;

/// Test scenario trait
#[cfg(feature = "test-harness")]
pub trait TestScenario: Send + Sync {
    /// Scenario name
    fn name(&self) -> &str;
    
    /// Setup the test
    fn setup(&mut self) -> anyhow::Result<()>;
    
    /// Execute the test
    fn execute(&mut self) -> anyhow::Result<TestResult>;
    
    /// Cleanup after test
    fn teardown(&mut self) -> anyhow::Result<()>;
}

/// Test result
#[cfg(feature = "test-harness")]
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Test passed
    pub passed: bool,
    /// Test duration
    pub duration_ms: u64,
    /// Error message if failed
    pub error: Option<String>,
    /// Metrics collected
    pub metrics: TestMetrics,
}

/// Test metrics
#[cfg(feature = "test-harness")]
#[derive(Debug, Clone, Default)]
pub struct TestMetrics {
    /// Messages sent
    pub messages_sent: u64,
    /// Messages delivered
    pub messages_delivered: u64,
    /// Average latency
    pub avg_latency_ms: f64,
    /// Packet loss rate
    pub packet_loss_rate: f64,
}