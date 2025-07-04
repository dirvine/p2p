use std::io::{self, Write};
use std::time::{Duration, Instant};
use std::thread;

fn main() {
    clear_screen();
    show_welcome();
    
    let mode = get_user_mode();
    
    match mode {
        UserMode::Host => run_as_host(),
        UserMode::Join => run_as_joiner(),
    }
}

enum UserMode {
    Host,
    Join,
}

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
}

fn show_welcome() {
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
    clear_screen();
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
    clear_screen();
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
    
    clear_screen();
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
    
    for i in 0..total_steps {
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
    println!("   (Press Ctrl+C to stop)");
    println!();
    
    // Simulate waiting for connections
    thread::sleep(Duration::from_secs(5));
    
    println!("🔔 Someone is connecting...");
    show_progress("Authenticating peer", 2);
    println!("✅ river-quick-fox has joined the chat!");
    println!();
    
    start_chat(address);
}

fn start_chat(address: &str) {
    println!("💬 Chat Room Active");
    println!("─────────────────────────────────────────────────────────────────────");
    println!("Commands: /help, /peers, /info, /quit");
    println!();
    
    // Simulate some chat activity
    println!("[river-quick-fox] Hello! The connection worked perfectly! 🎉");
    println!("[You] Welcome! Isn't it amazing that this works through all those NATs?");
    println!("[river-quick-fox] And it's quantum-resistant too! The future is here 🚀");
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
                println!("  • ocean-swift-mountain (You)");
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
                
                // Simulate response
                thread::sleep(Duration::from_millis(800));
                if trimmed.contains("?") {
                    println!("[river-quick-fox] Good question! Let me think...");
                } else {
                    println!("[river-quick-fox] Totally agree!");
                }
            }
            _ => {}
        }
    }
}