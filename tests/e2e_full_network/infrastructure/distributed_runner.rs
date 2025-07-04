
//! Distributed test runner for multi-computer coordination

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::{Mutex, RwLock, broadcast},
    time::{interval, timeout},
};
use uuid::Uuid;

/// Distributed test coordinator
pub struct DistributedTestCoordinator {
    bind_addr: SocketAddr,
    remote_nodes: Arc<RwLock<HashMap<String, RemoteTestNode>>>,
    command_tx: broadcast::Sender<TestCommand>,
    status: Arc<RwLock<CoordinatorStatus>>,
}

impl DistributedTestCoordinator {
    /// Create new coordinator
    pub async fn new(bind_addr: SocketAddr) -> Result<Self> {
        let (command_tx, _) = broadcast::channel(1024);
        
        Ok(Self {
            bind_addr,
            remote_nodes: Arc::new(RwLock::new(HashMap::new())),
            command_tx,
            status: Arc::new(RwLock::new(CoordinatorStatus::default())),
        })
    }
    
    /// Run the coordinator
    pub async fn run(self) -> Result<()> {
        let listener = TcpListener::bind(self.bind_addr).await
            .context("Failed to bind coordinator")?;
        
        println!("🎮 Coordinator listening on {}", self.bind_addr);
        
        // Spawn status reporter
        let status = self.status.clone();
        let nodes = self.remote_nodes.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                Self::report_status(&status, &nodes).await;
            }
        });
        
        // Accept remote connections
        loop {
            let (stream, addr) = listener.accept().await?;
            println!("📡 Remote node connected from {}", addr);
            
            let nodes = self.remote_nodes.clone();
            let cmd_tx = self.command_tx.clone();
            let status = self.status.clone();
            
            tokio::spawn(async move {
                if let Err(e) = Self::handle_remote_node(stream, addr, nodes, cmd_tx, status).await {
                    eprintln!("❌ Error handling remote node {}: {}", addr, e);
                }
            });
        }
    }
    
    /// Handle a remote node connection
    async fn handle_remote_node(
        stream: TcpStream,
        addr: SocketAddr,
        nodes: Arc<RwLock<HashMap<String, RemoteTestNode>>>,
        cmd_tx: broadcast::Sender<TestCommand>,
        status: Arc<RwLock<CoordinatorStatus>>,
    ) -> Result<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut cmd_rx = cmd_tx.subscribe();
        
        // Read node info
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let node_info: NodeInfo = serde_json::from_str(&line)?;
        
        let node_id = node_info.id.clone();
        
        // Register node
        {
            let mut nodes = nodes.write().await;
            nodes.insert(node_id.clone(), RemoteTestNode {
                id: node_id.clone(),
                name: node_info.name,
                address: addr,
                node_count: node_info.node_count,
                status: NodeStatus::Connected,
                last_heartbeat: Instant::now(),
            });
            
            let mut status = status.write().await;
            status.total_remote_nodes += node_info.node_count;
        }
        
        println!("✅ Registered remote node {} with {} nodes", node_id, node_info.node_count);
        
        // Send acknowledgment
        let ack = serde_json::to_string(&TestResponse::Acknowledged)?;
        writer.write_all(ack.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        
        // Handle bidirectional communication
        let writer = Arc::new(Mutex::new(writer));
        
        // Spawn command sender
        let writer_clone = writer.clone();
        let cmd_handle = tokio::spawn(async move {
            while let Ok(cmd) = cmd_rx.recv().await {
                let writer = writer_clone.lock().await;
                if let Err(e) = Self::send_command(&*writer, &cmd).await {
                    eprintln!("Failed to send command to {}: {}", node_id, e);
                    break;
                }
            }
        });
        
        // Read responses
        loop {
            let mut line = String::new();
            match timeout(Duration::from_secs(30), reader.read_line(&mut line)).await {
                Ok(Ok(0)) => break, // Connection closed
                Ok(Ok(_)) => {
                    if let Ok(response) = serde_json::from_str::<TestResponse>(&line) {
                        Self::handle_response(&node_id, response, &nodes, &status).await?;
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("Error reading from {}: {}", node_id, e);
                    break;
                }
                Err(_) => {
                    eprintln!("Timeout reading from {}", node_id);
                    break;
                }
            }
        }
        
        // Cleanup
        cmd_handle.abort();
        {
            let mut nodes = nodes.write().await;
            if let Some(node) = nodes.get_mut(&node_id) {
                node.status = NodeStatus::Disconnected;
            }
        }
        
        println!("🔌 Remote node {} disconnected", node_id);
        Ok(())
    }
    
    /// Send command to remote node
    async fn send_command(
        mut writer: &tokio::net::tcp::OwnedWriteHalf,
        cmd: &TestCommand,
    ) -> Result<()> {
        let msg = serde_json::to_string(cmd)?;
        writer.write_all(msg.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(())
    }
    
    /// Handle response from remote node
    async fn handle_response(
        node_id: &str,
        response: TestResponse,
        nodes: &Arc<RwLock<HashMap<String, RemoteTestNode>>>,
        status: &Arc<RwLock<CoordinatorStatus>>,
    ) -> Result<()> {
        match response {
            TestResponse::Heartbeat => {
                let mut nodes = nodes.write().await;
                if let Some(node) = nodes.get_mut(node_id) {
                    node.last_heartbeat = Instant::now();
                }
            }
            TestResponse::TestProgress(progress) => {
                let mut status = status.write().await;
                status.test_progress.insert(node_id.to_string(), progress);
            }
            TestResponse::TestComplete(result) => {
                let mut status = status.write().await;
                status.completed_nodes.insert(node_id.to_string(), result);
            }
            TestResponse::Error(e) => {
                eprintln!("❌ Error from {}: {}", node_id, e);
                let mut status = status.write().await;
                status.errors.push((node_id.to_string(), e));
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Report coordinator status
    async fn report_status(
        status: &Arc<RwLock<CoordinatorStatus>>,
        nodes: &Arc<RwLock<HashMap<String, RemoteTestNode>>>,
    ) {
        let status = status.read().await;
        let nodes = nodes.read().await;
        
        println!("\n📊 Coordinator Status");
        println!("====================");
        println!("Remote nodes: {} ({} total test nodes)", 
            nodes.len(), 
            status.total_remote_nodes
        );
        println!("Test phase: {}", status.current_phase);
        
        if !status.test_progress.is_empty() {
            println!("\nTest Progress:");
            for (node, progress) in &status.test_progress {
                println!("  {}: {}% - {}", node, progress.percentage, progress.current_test);
            }
        }
        
        if !status.completed_nodes.is_empty() {
            println!("\nCompleted Nodes:");
            for (node, result) in &status.completed_nodes {
                println!("  {}: {} passed, {} failed", 
                    node, 
                    result.tests_passed, 
                    result.tests_failed
                );
            }
        }
        
        if !status.errors.is_empty() {
            println!("\n⚠️  Errors:");
            for (node, error) in &status.errors {
                println!("  {}: {}", node, error);
            }
        }
        
        println!();
    }
    
    /// Broadcast command to all nodes
    pub async fn broadcast_command(&self, command: TestCommand) -> Result<()> {
        self.command_tx.send(command)
            .map_err(|_| anyhow::anyhow!("Failed to broadcast command"))?;
        Ok(())
    }
    
    /// Wait for all nodes to complete
    pub async fn wait_for_completion(&self, timeout_duration: Duration) -> Result<TestSummary> {
        let start = Instant::now();
        
        loop {
            if start.elapsed() > timeout_duration {
                return Err(anyhow::anyhow!("Test timeout"));
            }
            
            let status = self.status.read().await;
            let nodes = self.remote_nodes.read().await;
            
            if status.completed_nodes.len() == nodes.len() {
                // All nodes completed
                break;
            }
            
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        
        // Compile summary
        let status = self.status.read().await;
        let mut summary = TestSummary {
            total_nodes: status.total_remote_nodes,
            total_tests_passed: 0,
            total_tests_failed: 0,
            duration: start.elapsed(),
            node_results: HashMap::new(),
        };
        
        for (node, result) in &status.completed_nodes {
            summary.total_tests_passed += result.tests_passed;
            summary.total_tests_failed += result.tests_failed;
            summary.node_results.insert(node.clone(), result.clone());
        }
        
        Ok(summary)
    }
}

/// Remote test node information
struct RemoteTestNode {
    id: String,
    name: Option<String>,
    address: SocketAddr,
    node_count: usize,
    status: NodeStatus,
    last_heartbeat: Instant,
}

#[derive(Clone)]
enum NodeStatus {
    Connected,
    Disconnected,
    Failed,
}

/// Coordinator status
#[derive(Default)]
struct CoordinatorStatus {
    current_phase: String,
    total_remote_nodes: usize,
    test_progress: HashMap<String, TestProgress>,
    completed_nodes: HashMap<String, TestResult>,
    errors: Vec<(String, String)>,
}

/// Node information sent on connection
#[derive(Serialize, Deserialize)]
struct NodeInfo {
    id: String,
    name: Option<String>,
    node_count: usize,
    version: String,
}

/// Test commands sent from coordinator
#[derive(Clone, Serialize, Deserialize)]
pub enum TestCommand {
    StartTest(TestConfig),
    StopTest,
    GetStatus,
    SetPhase(String),
    Configure(HashMap<String, String>),
}

/// Test configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub test_suites: Vec<String>,
    pub timeout_secs: u64,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Responses from remote nodes
#[derive(Serialize, Deserialize)]
enum TestResponse {
    Acknowledged,
    Heartbeat,
    TestProgress(TestProgress),
    TestComplete(TestResult),
    Error(String),
}

/// Test progress information
#[derive(Clone, Serialize, Deserialize)]
struct TestProgress {
    percentage: u8,
    current_test: String,
    tests_completed: u32,
    tests_remaining: u32,
}

/// Test result from a node
#[derive(Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub tests_passed: u32,
    pub tests_failed: u32,
    pub duration: Duration,
    pub details: HashMap<String, serde_json::Value>,
}

/// Test summary across all nodes
pub struct TestSummary {
    pub total_nodes: usize,
    pub total_tests_passed: u32,
    pub total_tests_failed: u32,
    pub duration: Duration,
    pub node_results: HashMap<String, TestResult>,
}

/// Remote test participant
pub struct RemoteTestParticipant {
    coordinator_addr: SocketAddr,
    node_info: NodeInfo,
    stream: Option<TcpStream>,
}

impl RemoteTestParticipant {
    /// Create new remote participant
    pub async fn new(
        coordinator_addr: SocketAddr,
        node_count: usize,
        name: Option<String>,
    ) -> Result<Self> {
        Ok(Self {
            coordinator_addr,
            node_info: NodeInfo {
                id: Uuid::new_v4().to_string(),
                name,
                node_count,
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            stream: None,
        })
    }
    
    /// Connect to coordinator
    pub async fn connect(&mut self) -> Result<()> {
        let stream = TcpStream::connect(self.coordinator_addr).await
            .context("Failed to connect to coordinator")?;
        
        self.stream = Some(stream);
        
        // Send node info
        let info = serde_json::to_string(&self.node_info)?;
        if let Some(stream) = &mut self.stream {
            stream.write_all(info.as_bytes()).await?;
            stream.write_all(b"\n").await?;
            stream.flush().await?;
        }
        
        // Wait for acknowledgment
        let mut reader = BufReader::new(self.stream.as_ref().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        
        if let Ok(TestResponse::Acknowledged) = serde_json::from_str(&line) {
            println!("✅ Connected to coordinator at {}", self.coordinator_addr);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to receive acknowledgment"))
        }
    }
    
    /// Run as remote participant
    pub async fn run(&mut self) -> Result<()> {
        let stream = self.stream.take()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        
        let (reader, writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let writer = Arc::new(Mutex::new(writer));
        
        // Spawn heartbeat sender
        let writer_clone = writer.clone();
        let heartbeat_handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                let writer = writer_clone.lock().await;
                if let Err(e) = Self::send_response(&*writer, &TestResponse::Heartbeat).await {
                    eprintln!("Failed to send heartbeat: {}", e);
                    break;
                }
            }
        });
        
        // Process commands
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // Connection closed
                Ok(_) => {
                    if let Ok(command) = serde_json::from_str::<TestCommand>(&line) {
                        if let Err(e) = self.handle_command(command, &writer).await {
                            eprintln!("Error handling command: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading command: {}", e);
                    break;
                }
            }
        }
        
        heartbeat_handle.abort();
        println!("🔌 Disconnected from coordinator");
        
        Ok(())
    }
    
    /// Handle command from coordinator
    async fn handle_command(
        &self,
        command: TestCommand,
        writer: &Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
    ) -> Result<()> {
        match command {
            TestCommand::StartTest(config) => {
                println!("🚀 Starting tests: {:?}", config.test_suites);
                // Test execution would happen here
                
                // Send progress updates
                let progress = TestProgress {
                    percentage: 50,
                    current_test: "identity_tests".to_string(),
                    tests_completed: 10,
                    tests_remaining: 10,
                };
                
                let writer = writer.lock().await;
                Self::send_response(&*writer, &TestResponse::TestProgress(progress)).await?;
            }
            TestCommand::StopTest => {
                println!("🛑 Stopping tests");
            }
            TestCommand::GetStatus => {
                // Send current status
            }
            TestCommand::SetPhase(phase) => {
                println!("📍 Test phase: {}", phase);
            }
            TestCommand::Configure(params) => {
                println!("⚙️  Configuring: {:?}", params);
            }
        }
        
        Ok(())
    }
    
    /// Send response to coordinator
    async fn send_response(
        mut writer: &tokio::net::tcp::OwnedWriteHalf,
        response: &TestResponse,
    ) -> Result<()> {
        let msg = serde_json::to_string(response)?;
        writer.write_all(msg.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(())
    }
}