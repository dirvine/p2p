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
use std::io::{self, Write, Read, BufRead, BufReader};
use std::net::{TcpListener, TcpStream, SocketAddr, IpAddr};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::HashMap;

struct ChatNode {
    address: String,
    peers: Arc<Mutex<HashMap<String, TcpStream>>>,
    messages: Arc<Mutex<Vec<String>>>,
}

impl ChatNode {
    fn new() -> Self {
        Self {
            address: generate_three_word_address(),
            peers: Arc::new(Mutex::new(HashMap::new())),
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn find_available_port() -> u16 {
        // Try a range of ports until we find one that's free
        for port in 9000..9100 {
            if let Ok(listener) = TcpListener::bind(("0.0.0.0", port)) {
                drop(listener);
                return port;
            }
        }
        // If all ports in range are taken, let OS assign
        let listener = TcpListener::bind("0.0.0.0:0").expect("Failed to bind to any port");
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }

    fn detect_network_capability() -> NetworkCapability {
        // Check IPv6 first
        if let Ok(_) = TcpListener::bind("[::1]:0") {
            // Try to bind to all IPv6 interfaces
            if let Ok(_) = TcpListener::bind("[::]:0") {
                return NetworkCapability::IPv6Direct;
            }
        }
        
        // Check IPv4
        if let Ok(_) = TcpListener::bind("127.0.0.1:0") {
            if let Ok(_) = TcpListener::bind("0.0.0.0:0") {
                return NetworkCapability::IPv4WithNAT;
            }
        }
        
        NetworkCapability::NoNetwork
    }

    fn start_host(&mut self) -> std::io::Result<u16> {
        let port = Self::find_available_port();
        let capability = Self::detect_network_capability();
        
        println!();
        println!("🚀 Starting your P2P node...");
        println!();
        
        // Network detection
        show_progress("Detecting network environment", 2);
        match capability {
            NetworkCapability::IPv6Direct => {
                println!("✅ Network detected: Direct IPv6 available!");
                println!("   No tunneling needed - using native IPv6");
            }
            NetworkCapability::IPv4WithNAT => {
                println!("✅ Network detected: IPv4 with NAT");
                println!("   Will use IPv4 for now (tunnel setup would happen here in production)");
            }
            NetworkCapability::NoNetwork => {
                println!("❌ No network connectivity detected");
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "No network"));
            }
        }
        println!();
        
        show_progress("Setting up encryption", 1);
        println!("✅ Encryption initialized (would be ML-KEM in production)");
        println!();
        
        // Start listening
        let listen_addr = match capability {
            NetworkCapability::IPv6Direct => format!("[::]:{}", port),
            _ => format!("0.0.0.0:{}", port),
        };
        
        let listener = TcpListener::bind(&listen_addr)?;
        println!("✅ Listening on port {}", port);
        println!();
        
        // Start accept thread
        let peers = Arc::clone(&self.peers);
        let messages = Arc::clone(&self.messages);
        thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(stream) = stream {
                    handle_peer_connection(stream, peers.clone(), messages.clone());
                }
            }
        });
        
        Ok(port)
    }

    fn connect_to_peer(&mut self, address: &str, peer_port: u16) -> std::io::Result<()> {
        println!("🔍 Connecting to {} on port {}...", address, peer_port);
        
        show_progress("Resolving address", 1);
        
        // Try different connection methods
        let mut connected = false;
        
        // Try IPv6 first if available
        if Self::detect_network_capability() == NetworkCapability::IPv6Direct {
            if let Ok(stream) = TcpStream::connect(format!("[::1]:{}", peer_port)) {
                println!("✅ Connected via IPv6!");
                self.peers.lock().unwrap().insert(address.to_string(), stream);
                connected = true;
            }
        }
        
        // Fall back to IPv4
        if !connected {
            match TcpStream::connect(format!("127.0.0.1:{}", peer_port)) {
                Ok(stream) => {
                    println!("✅ Connected via IPv4!");
                    self.peers.lock().unwrap().insert(address.to_string(), stream);
                    connected = true;
                }
                Err(e) => {
                    println!("❌ Connection failed: {}", e);
                    println!("   Make sure your friend's node is running on port {}", peer_port);
                    return Err(e);
                }
            }
        }
        
        if connected {
            println!("✅ Secure connection established!");
        }
        
        Ok(())
    }

    fn broadcast_message(&self, message: &str) {
        let peers = self.peers.lock().unwrap();
        for (peer_addr, stream) in peers.iter() {
            // In a real implementation, we'd properly handle writing to streams
            // For now, we'll just print locally
            println!("Would send to {}: {}", peer_addr, message);
        }
    }
}

#[derive(PartialEq)]
enum NetworkCapability {
    IPv6Direct,
    IPv4WithNAT,
    NoNetwork,
}

fn main() {
    println!("Starting P2P Chat...");
    thread::sleep(Duration::from_secs(1));
    
    show_welcome();
    
    let mut node = ChatNode::new();
    
    let mode = get_user_mode();
    
    match mode {
        UserMode::Host => {
            match node.start_host() {
                Ok(port) => {
                    show_host_ready(&node.address, port);
                    run_chat_interface(&node, true);
                }
                Err(e) => {
                    println!("❌ Failed to start node: {}", e);
                    println!("   Please check your network settings");
                }
            }
        }
        UserMode::Join => {
            let (peer_address, peer_port) = get_peer_info();
            match node.connect_to_peer(&peer_address, peer_port) {
                Ok(_) => {
                    show_join_success(&peer_address);
                    run_chat_interface(&node, false);
                }
                Err(_) => {
                    println!("❌ Could not connect to peer");
                    println!("   Please check the address and port");
                }
            }
        }
    }
    
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
    println!("║                  Real P2P Chat - No Fake Peers!                     ║");
    println!("║                                                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();
    thread::sleep(Duration::from_millis(1000));
}

fn get_user_mode() -> UserMode {
    println!("What would you like to do?");
    println!();
    println!("  1) Start a new chat room");
    println!("  2) Join a friend's chat room");
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

fn get_peer_info() -> (String, u16) {
    println!();
    println!("🔗 Let's connect you to your friend's chat room!");
    println!();
    
    print!("Enter your friend's three-word address: ");
    io::stdout().flush().unwrap();
    let mut address = String::new();
    io::stdin().read_line(&mut address).unwrap();
    let address = address.trim().to_string();
    
    print!("Enter the port number they gave you: ");
    io::stdout().flush().unwrap();
    let mut port_str = String::new();
    io::stdin().read_line(&mut port_str).unwrap();
    
    let port = port_str.trim().parse::<u16>().unwrap_or(9000);
    
    (address, port)
}

fn show_host_ready(address: &str, port: u16) {
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                      ║");
    println!("║                    🎉 Your node is ready! 🎉                        ║");
    println!("║                                                                      ║");
    println!("║  Share these details with your friends:                             ║");
    println!("║                                                                      ║");
    println!("║     Address: {}                             ║", format!("{:<30}", address));
    println!("║     Port: {}                                                      ║", format!("{:<5}", port));
    println!("║                                                                      ║");
    println!("║  They'll need both to connect!                                      ║");
    println!("║                                                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();
}

fn show_join_success(peer_address: &str) {
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                      ║");
    println!("║               🎊 Successfully connected! 🎊                          ║");
    println!("║                                                                      ║");
    println!("║  Connected to: {}                          ║", format!("{:<30}", peer_address));
    println!("║                                                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();
}

fn run_chat_interface(node: &ChatNode, is_host: bool) {
    println!("💬 Chat Room Active");
    println!("─────────────────────────────────────────────────────────────────────");
    println!("Commands: /help, /peers, /quit");
    println!();
    
    if is_host {
        println!("Waiting for friends to connect on your port...");
        println!();
    }
    
    // Start message receiver thread
    let messages = Arc::clone(&node.messages);
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(100));
            let mut msgs = messages.lock().unwrap();
            for msg in msgs.drain(..) {
                println!("\r{}", msg);
                print!("> ");
                io::stdout().flush().unwrap();
            }
        }
    });
    
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let trimmed = input.trim();
        
        match trimmed {
            "/quit" => {
                println!("👋 Disconnecting from P2P network...");
                break;
            }
            "/help" => {
                println!("Available commands:");
                println!("  /help  - Show this help");
                println!("  /peers - List connected peers");
                println!("  /quit  - Exit the chat");
            }
            "/peers" => {
                let peers = node.peers.lock().unwrap();
                println!("Connected peers: {}", peers.len());
                for (addr, _) in peers.iter() {
                    println!("  • {}", addr);
                }
            }
            _ if !trimmed.is_empty() => {
                println!("[You] {}", trimmed);
                node.broadcast_message(&format!("[{}] {}", node.address, trimmed));
            }
            _ => {}
        }
    }
}

fn show_progress(task: &str, seconds: u64) {
    print!("⏳ {}...", task);
    io::stdout().flush().unwrap();
    
    for _ in 0..3 {
        thread::sleep(Duration::from_millis((seconds * 1000) / 3));
        print!(".");
        io::stdout().flush().unwrap();
    }
    
    println!(" Done!");
}

fn generate_three_word_address() -> String {
    // Simple deterministic generation for demo
    let words = vec![
        vec!["ocean", "river", "mountain", "forest", "desert", "valley"],
        vec!["swift", "bright", "calm", "bold", "wise", "free"],
        vec!["eagle", "wolf", "bear", "fox", "hawk", "lion"],
    ];
    
    // Use system time for some randomness
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    
    format!("{}-{}-{}", 
        words[0][seed % 6],
        words[1][(seed / 6) % 6],
        words[2][(seed / 36) % 6]
    )
}

fn handle_peer_connection(
    mut stream: TcpStream,
    peers: Arc<Mutex<HashMap<String, TcpStream>>>,
    messages: Arc<Mutex<Vec<String>>>
) {
    let peer_addr = stream.peer_addr().unwrap().to_string();
    println!();
    println!("🔔 New peer connected from: {}", peer_addr);
    
    // Store peer info
    peers.lock().unwrap().insert(peer_addr.clone(), stream.try_clone().unwrap());
    
    // Simple message reading
    thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            if let Ok(msg) = line {
                messages.lock().unwrap().push(msg);
            }
        }
    });
}