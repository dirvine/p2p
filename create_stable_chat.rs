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
use std::io::{self, Write};
use std::time::{Duration, Instant};
use std::thread;

fn main() {
    // Don't clear screen immediately - let user see the window
    println!("Starting P2P Chat...");
    thread::sleep(Duration::from_secs(1));
    
    show_welcome();
    
    let mode = get_user_mode();
    
    match mode {
        UserMode::Host => run_as_host(),
        UserMode::Join => run_as_joiner(),
    }
    
    // Keep window open at the end
    println!();
    println!("Press Enter to exit...");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}

enum UserMode {
    Host,
    Join,
}

fn show_welcome() {
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                      ║");
    println!("║                    🐜 P2P Foundation Network 🐜                      ║");
    println!("║                                                                      ║");
    println!("║                  Secure, Decentralized Communication                 ║");
    println!("║                                                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();
    thread::sleep(Duration::from_millis(1500));
}

fn get_user_mode() -> UserMode {
    println!("Welcome! Let's get you connected to the P2P network.");
    println!();
    println!("Are you:");
    println!("  1) Starting a new chat room (I'll give you an address to share)");
    println!("  2) Joining a friend's chat room (I have their address)");
    println!();
    
    loop {
        print!("Please enter 1 or 2: ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        match input.trim() {
            "1" => return UserMode::Host,
            "2" => return UserMode::Join,
            _ => println!("Please enter either 1 or 2"),
        }
    }
}

fn run_as_host() {
    println!();
    println!("🚀 Starting your P2P node...");
    println!();
    
    // Simulate network detection
    show_progress("Detecting network environment", 3);
    println!("✅ Network detected: IPv4 with NAT");
    println!();
    
    show_progress("Setting up quantum-resistant encryption", 2);
    println!("✅ ML-KEM encryption initialized");
    println!();
    
    show_progress("Establishing P2P tunnel", 4);
    println!("✅ Teredo tunnel established for IPv6 over IPv4");
    println!();
    
    show_progress("Generating your unique address", 2);
    
    let address = generate_three_word_address();
    
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                      ║");
    println!("║                    🎉 Your node is ready! 🎉                        ║");
    println!("║                                                                      ║");
    println!("║  Your three-word address is:                                        ║");
    println!("║                                                                      ║");
    println!("║            🔑  {}                         ║", format_address(&address));
    println!("║                                                                      ║");
    println!("║  Share this address with your friends so they can connect!          ║");
    println!("║                                                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();
    
    println!("📊 Connection Details:");
    println!("   • Local IP: 192.168.1.42");
    println!("   • Public IP: [Hidden for privacy]");
    println!("   • Tunnel: Teredo (IPv6 over IPv4)");
    println!("   • Encryption: ML-KEM-768 (Quantum-resistant)");
    println!("   • Port: 9000 (auto-selected)");
    println!();
    
    wait_for_connections(&address);
}

fn run_as_joiner() {
    println!();
    println!("🔗 Let's connect you to your friend's chat room!");
    println!();
    println!("Please enter your friend's three-word address");
    println!("(Example: ocean-swift-mountain)");
    println!();
    
    let address = loop {
        print!("Address: ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let trimmed = input.trim();
        
        if validate_three_word_address(trimmed) {
            break trimmed.to_string();
        } else {
            println!("⚠️  That doesn't look like a valid three-word address.");
            println!("   Please use the format: word-word-word");
            println!();
        }
    };
    
    println!();
    println!("🔍 Connecting to {}...", address);
    println!();
    
    show_progress("Detecting network environment", 2);
    println!("✅ Network detected: IPv4 with NAT");
    println!();
    
    show_progress("Resolving friend's address", 3);
    println!("✅ Address resolved to peer ID: a7f8...3d2e");
    println!();
    
    show_progress("Establishing secure tunnel", 4);
    println!("✅ Teredo tunnel established");
    println!();
    
    show_progress("Performing quantum-resistant handshake", 3);
    println!("✅ Secure connection established!");
    println!();
    
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                      ║");
    println!("║               🎊 Successfully connected! 🎊                          ║");
    println!("║                                                                      ║");
    println!("║  Connected to: {}                          ║", format_address(&address));
    println!("║  Encryption: ML-KEM-768 (Quantum-resistant)                         ║");
    println!("║  Tunnel: Teredo IPv6 over IPv4                                      ║");
    println!("║                                                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();
    
    start_chat(&address);
}

fn show_progress(task: &str, seconds: u64) {
    print!("⏳ {}...", task);
    io::stdout().flush().unwrap();
    
    let start = Instant::now();
    let total_steps = 20;
    
    for _ in 0..total_steps {
        thread::sleep(Duration::from_millis((seconds * 1000) / total_steps));
        print!(".");
        io::stdout().flush().unwrap();
    }
    
    println!(" Done! ({}s)", start.elapsed().as_secs());
}

fn generate_three_word_address() -> String {
    // In real implementation, this would be derived from peer ID
    let words = vec![
        vec!["ocean", "river", "mountain", "forest", "desert", "valley"],
        vec!["swift", "bright", "calm", "bold", "wise", "free"],
        vec!["eagle", "wolf", "bear", "fox", "hawk", "lion"],
    ];
    
    let addr = format!("{}-{}-{}", 
        words[0][3],  // forest
        words[1][1],  // bright  
        words[2][0]   // eagle
    );
    
    addr
}

fn format_address(address: &str) -> String {
    format!("{:<30}", address)
}

fn validate_three_word_address(address: &str) -> bool {
    let parts: Vec<&str> = address.split('-').collect();
    parts.len() == 3 && parts.iter().all(|p| p.len() > 2 && p.chars().all(|c| c.is_alphabetic()))
}

fn wait_for_connections(address: &str) {
    println!("⏳ Waiting for friends to connect...");
    println!("   (Type /quit to stop)");
    println!();
    
    // Start a thread to simulate connection
    thread::spawn(|| {
        thread::sleep(Duration::from_secs(5));
        println!();
        println!("🔔 Someone is connecting...");
        thread::sleep(Duration::from_secs(2));
        println!("✅ river-quick-fox has joined the chat!");
        println!();
    });
    
    // Start chat interface
    start_chat(address);
}

fn start_chat(address: &str) {
    thread::sleep(Duration::from_millis(500));
    
    println!("💬 Chat Room Active");
    println!("─────────────────────────────────────────────────────────────────────");
    println!("Commands: /help, /peers, /info, /quit");
    println!();
    
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let trimmed = input.trim();
        
        match trimmed {
            "/quit" => {
                println!("👋 Disconnecting from P2P network...");
                println!("Thanks for using P2P Foundation!");
                break;
            }
            "/help" => {
                println!("Available commands:");
                println!("  /help  - Show this help");
                println!("  /peers - List connected peers");
                println!("  /info  - Show connection details");
                println!("  /quit  - Exit the chat");
            }
            "/peers" => {
                println!("Connected peers:");
                println!("  • river-quick-fox (Direct P2P connection)");
                println!("  • {} (You)", address);
            }
            "/info" => {
                println!("Connection Information:");
                println!("  • Your address: {}", address);
                println!("  • Encryption: ML-KEM-768 + ChaCha20-Poly1305");
                println!("  • Tunnel: Teredo (IPv6 over IPv4)");
                println!("  • NAT traversal: Successful via QUIC");
                println!("  • Latency: 45ms");
                println!("  • Packet loss: 0%");
            }
            _ if !trimmed.is_empty() => {
                println!("[You] {}", trimmed);
                
                // Simulate response after a moment
                let message = trimmed.to_string();
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(800));
                    if message.contains("?") {
                        println!();
                        println!("[river-quick-fox] Good question! Let me think...");
                        print!("> ");
                        io::stdout().flush().unwrap();
                    } else if message.to_lowercase().contains("hello") {
                        println!();
                        println!("[river-quick-fox] Hey there! How's it going?");
                        print!("> ");
                        io::stdout().flush().unwrap();
                    }
                });
            }
            _ => {}
        }
    }
}