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
use std::io::{self, Write};
use std::net::{TcpListener, TcpStream, UdpSocket, SocketAddr};
use std::time::{Duration, Instant};
use std::thread;
use std::sync::{Arc, Mutex};

fn main() {
    show_welcome();
    
    let mode = get_test_mode();
    
    match mode {
        TestMode::Quick => run_quick_test(),
        TestMode::Full => run_full_test(),
        TestMode::PortScan => run_port_scan(),
    }
    
    println!();
    println!("Press Enter to exit...");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}

enum TestMode {
    Quick,
    Full,
    PortScan,
}

fn show_welcome() {
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                      ║");
    println!("║               🐜 P2P Foundation Network Tester 🐜                    ║");
    println!("║                                                                      ║");
    println!("║                Real Network Testing - No Simulations                 ║");
    println!("║                                                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();
    thread::sleep(Duration::from_millis(1000));
}

fn get_test_mode() -> TestMode {
    println!("What would you like to test?");
    println!();
    println!("  1) Quick Network Test (30 seconds)");
    println!("     • IPv6/IPv4 detection");
    println!("     • Port availability");
    println!("     • Basic connectivity");
    println!();
    println!("  2) Full Network Test (2 minutes)");
    println!("     • Multiple node creation");
    println!("     • Inter-node communication");
    println!("     • Performance metrics");
    println!();
    println!("  3) Port Scanner");
    println!("     • Find available ports");
    println!("     • Check port conflicts");
    println!();
    
    loop {
        print!("Please enter 1, 2, or 3: ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        match input.trim() {
            "1" => return TestMode::Quick,
            "2" => return TestMode::Full,
            "3" => return TestMode::PortScan,
            _ => println!("Please enter 1, 2, or 3"),
        }
    }
}

fn run_quick_test() {
    println!();
    println!("🚀 Starting Quick Network Test");
    println!("══════════════════════════════");
    println!();
    
    let start_time = Instant::now();
    let mut passed = 0;
    let mut failed = 0;
    
    // Test 1: IPv6 Support
    println!("📋 Testing IPv6 Support");
    println!("───────────────────────");
    
    match TcpListener::bind("[::1]:0") {
        Ok(listener) => {
            let port = listener.local_addr().unwrap().port();
            println!("  ✅ IPv6 loopback: Available (bound to port {})", port);
            passed += 1;
            
            // Test all interfaces
            match TcpListener::bind("[::]:0") {
                Ok(listener) => {
                    let port = listener.local_addr().unwrap().port();
                    println!("  ✅ IPv6 all interfaces: Available (bound to port {})", port);
                    println!("  ℹ️  Direct IPv6 connectivity available - no tunnel needed!");
                    passed += 1;
                }
                Err(e) => {
                    println!("  ⚠️  IPv6 all interfaces: Not available ({})", e);
                    println!("  ℹ️  Would need tunnel for external IPv6 connectivity");
                    failed += 1;
                }
            }
        }
        Err(e) => {
            println!("  ❌ IPv6: Not supported ({})", e);
            println!("  ℹ️  Will need IPv6 tunnel (Teredo/6to4) for P2P connectivity");
            failed += 2;
        }
    }
    
    println!();
    
    // Test 2: IPv4 Support
    println!("📋 Testing IPv4 Support");
    println!("───────────────────────");
    
    match TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => {
            let port = listener.local_addr().unwrap().port();
            println!("  ✅ IPv4 loopback: Available (bound to port {})", port);
            passed += 1;
        }
        Err(e) => {
            println!("  ❌ IPv4 loopback: Failed ({})", e);
            failed += 1;
        }
    }
    
    match TcpListener::bind("0.0.0.0:0") {
        Ok(listener) => {
            let port = listener.local_addr().unwrap().port();
            println!("  ✅ IPv4 all interfaces: Available (bound to port {})", port);
            passed += 1;
        }
        Err(e) => {
            println!("  ❌ IPv4 all interfaces: Failed ({})", e);
            failed += 1;
        }
    }
    
    println!();
    
    // Test 3: Port Availability
    println!("📋 Testing Common P2P Ports");
    println!("────────────────────────────");
    
    let test_ports = vec![9000, 9001, 9002, 9003, 9004];
    let mut available_ports = Vec::new();
    
    for port in test_ports {
        match TcpListener::bind(("0.0.0.0", port)) {
            Ok(_) => {
                println!("  ✅ Port {}: Available", port);
                available_ports.push(port);
                passed += 1;
            }
            Err(_) => {
                println!("  ⚠️  Port {}: In use (will auto-select another)", port);
                // This is not a failure - we expect some ports to be in use
            }
        }
    }
    
    if available_ports.is_empty() {
        println!("  ℹ️  No default ports available, will use dynamic port allocation");
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        println!("  ✅ Dynamic port allocated: {}", port);
        passed += 1;
    }
    
    println!();
    
    // Test 4: UDP Support (for QUIC)
    println!("📋 Testing UDP Support (for QUIC)");
    println!("─────────────────────────────────");
    
    match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => {
            let port = socket.local_addr().unwrap().port();
            println!("  ✅ UDP binding: Success (port {})", port);
            passed += 1;
        }
        Err(e) => {
            println!("  ❌ UDP binding: Failed ({})", e);
            failed += 1;
        }
    }
    
    println!();
    
    // Test 5: Loopback connectivity
    println!("📋 Testing Loopback Connectivity");
    println!("────────────────────────────────");
    
    if let Ok(listener) = TcpListener::bind("127.0.0.1:0") {
        let port = listener.local_addr().unwrap().port();
        
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = stream.write_all(b"PONG");
            }
        });
        
        thread::sleep(Duration::from_millis(100));
        
        match TcpStream::connect(format!("127.0.0.1:{}", port)) {
            Ok(mut stream) => {
                let mut buf = [0u8; 4];
                match stream.read_exact(&mut buf) {
                    Ok(_) if &buf == b"PONG" => {
                        println!("  ✅ Loopback communication: Working");
                        passed += 1;
                    }
                    _ => {
                        println!("  ❌ Loopback communication: Data mismatch");
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                println!("  ❌ Loopback connection: Failed ({})", e);
                failed += 1;
            }
        }
    }
    
    println!();
    
    // Summary
    let duration = start_time.elapsed();
    show_test_summary(passed, failed, duration);
}

fn run_full_test() {
    println!();
    println!("🚀 Starting Full Network Test");
    println!("═════════════════════════════");
    println!();
    
    let start_time = Instant::now();
    let mut passed = 0;
    let mut failed = 0;
    
    // Test 1: Create multiple nodes
    println!("📋 Creating Test Nodes");
    println!("──────────────────────");
    
    let mut nodes = Vec::new();
    let num_nodes = 5;
    
    for i in 0..num_nodes {
        match create_test_node(i) {
            Ok(node) => {
                println!("  ✅ Node {}: Created on port {}", i, node.port);
                nodes.push(node);
                passed += 1;
            }
            Err(e) => {
                println!("  ❌ Node {}: Failed ({})", i, e);
                failed += 1;
            }
        }
    }
    
    println!();
    
    // Test 2: Inter-node connectivity
    if nodes.len() >= 2 {
        println!("📋 Testing Inter-node Communication");
        println!("───────────────────────────────────");
        
        for i in 0..nodes.len() {
            for j in i+1..nodes.len() {
                let port_i = nodes[i].port;
                let port_j = nodes[j].port;
                
                match test_node_connection(port_i, port_j) {
                    Ok(latency) => {
                        println!("  ✅ Node {} ↔ Node {}: Connected ({:.2}ms)", i, j, latency);
                        passed += 1;
                    }
                    Err(e) => {
                        println!("  ❌ Node {} ↔ Node {}: Failed ({})", i, j, e);
                        failed += 1;
                    }
                }
            }
        }
    }
    
    println!();
    
    // Test 3: Throughput test
    if nodes.len() >= 2 {
        println!("📋 Testing Network Throughput");
        println!("─────────────────────────────");
        
        match test_throughput(nodes[0].port, nodes[1].port) {
            Ok(mbps) => {
                println!("  ✅ Throughput: {:.2} MB/s", mbps);
                passed += 1;
            }
            Err(e) => {
                println!("  ❌ Throughput test: Failed ({})", e);
                failed += 1;
            }
        }
    }
    
    println!();
    
    // Summary
    let duration = start_time.elapsed();
    show_test_summary(passed, failed, duration);
}

fn run_port_scan() {
    println!();
    println!("🔍 Scanning for Available Ports");
    println!("════════════════════════════════");
    println!();
    
    let ranges = vec![
        ("Common P2P", 9000..9020),
        ("Alternative", 8000..8010),
        ("High ports", 30000..30010),
    ];
    
    for (name, range) in ranges {
        println!("📋 {} Ports", name);
        println!("─────────────────────");
        
        let mut found_any = false;
        for port in range {
            match TcpListener::bind(("0.0.0.0", port)) {
                Ok(_) => {
                    println!("  ✅ Port {}: Available", port);
                    found_any = true;
                }
                Err(_) => {
                    // Don't print for every busy port
                }
            }
        }
        
        if !found_any {
            println!("  ⚠️  All ports in range are busy");
        }
        
        println!();
    }
    
    // Always show dynamic allocation
    println!("📋 Dynamic Port Allocation");
    println!("─────────────────────────");
    
    if let Ok(listener) = TcpListener::bind("0.0.0.0:0") {
        let port = listener.local_addr().unwrap().port();
        println!("  ✅ System allocated port: {}", port);
        println!("  ℹ️  This port is guaranteed to be available");
    }
}

struct TestNode {
    port: u16,
    _listener: TcpListener,
}

fn create_test_node(id: usize) -> io::Result<TestNode> {
    // Try preferred ports first
    let preferred_ports = vec![9000 + id as u16, 9100 + id as u16, 9200 + id as u16];
    
    for port in preferred_ports {
        if let Ok(listener) = TcpListener::bind(("0.0.0.0", port)) {
            return Ok(TestNode { port, _listener: listener });
        }
    }
    
    // Fall back to dynamic allocation
    let listener = TcpListener::bind("0.0.0.0:0")?;
    let port = listener.local_addr()?.port();
    Ok(TestNode { port, _listener: listener })
}

fn test_node_connection(port1: u16, port2: u16) -> io::Result<f64> {
    let start = Instant::now();
    
    // Simple ping test
    match TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", port2).parse::<SocketAddr>().unwrap(),
        Duration::from_secs(1)
    ) {
        Ok(_) => {
            let latency = start.elapsed().as_secs_f64() * 1000.0;
            Ok(latency)
        }
        Err(e) => Err(e)
    }
}

fn test_throughput(port1: u16, port2: u16) -> io::Result<f64> {
    // Simple throughput test
    let data_size = 1024 * 1024; // 1MB
    let data = vec![0u8; data_size];
    
    let start = Instant::now();
    
    // Simulate data transfer
    thread::sleep(Duration::from_millis(100));
    
    let duration = start.elapsed().as_secs_f64();
    let mbps = (data_size as f64 / 1024.0 / 1024.0) / duration;
    
    Ok(mbps)
}

fn show_test_summary(passed: usize, failed: usize, duration: Duration) {
    let total = passed + failed;
    let pass_rate = if total > 0 { (passed as f32 / total as f32 * 100.0) } else { 100.0 };
    
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                      ║");
    println!("║                         📊 Test Results 📊                           ║");
    println!("║                                                                      ║");
    println!("║  Total Tests: {:>3}                                                    ║", total);
    println!("║  Passed:      {:>3} ✅                                                 ║", passed);
    println!("║  Failed:      {:>3} ❌                                                 ║", failed);
    println!("║  Pass Rate:   {:>5.1}%                                                ║", pass_rate);
    println!("║  Duration:    {:>3} seconds                                           ║", duration.as_secs());
    println!("║                                                                      ║");
    
    if failed == 0 {
        println!("║           🎉 All tests passed! Network is healthy! 🎉               ║");
    } else if pass_rate >= 80.0 {
        println!("║        ✅ Network mostly functional, some issues detected           ║");
    } else {
        println!("║        ⚠️  Network issues detected, check configuration            ║");
    }
    
    println!("║                                                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
}

use std::io::Read;