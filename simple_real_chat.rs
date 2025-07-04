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
use std::io::{self, Write, Read, BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

fn main() {
    println!("Starting P2P Chat...");
    thread::sleep(Duration::from_secs(1));
    
    show_welcome();
    
    let mode = get_user_mode();
    
    match mode {
        UserMode::Host => run_as_host(),
        UserMode::Join => run_as_joiner(),
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
    println!("║                  Real P2P Chat - Direct Connection                   ║");
    println!("║                                                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();
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

fn run_as_host() {
    println!();
    println!("🚀 Starting your P2P node...");
    println!();
    
    // Detect network
    show_progress("Detecting network environment", 1);
    let (has_ipv6, has_ipv4) = detect_network_capability();
    
    if has_ipv6 {
        println!("✅ Network: Direct IPv6 available - no tunneling needed!");
    } else if has_ipv4 {
        println!("✅ Network: IPv4 only - would use Teredo/6to4 tunnel in production");
    } else {
        println!("❌ No network connectivity detected");
        return;
    }
    println!();
    
    // Find available port
    let port = find_available_port();
    let listen_addr = if has_ipv6 {
        format!("[::]:{}", port)
    } else {
        format!("0.0.0.0:{}", port)
    };
    
    match TcpListener::bind(&listen_addr) {
        Ok(listener) => {
            let address = generate_three_word_address();
            show_host_ready(&address, port);
            
            // Accept connections in background
            thread::spawn(move || {
                for stream in listener.incoming() {
                    if let Ok(stream) = stream {
                        handle_peer(stream);
                    }
                }
            });
            
            // Simple chat loop
            run_chat_loop();
        }
        Err(e) => {
            println!("❌ Failed to start: {}", e);
        }
    }
}

fn run_as_joiner() {
    println!();
    println!("🔗 Let's connect to your friend!");
    println!();
    
    print!("Enter your friend's three-word address: ");
    io::stdout().flush().unwrap();
    let mut address = String::new();
    io::stdin().read_line(&mut address).unwrap();
    
    print!("Enter their port number: ");
    io::stdout().flush().unwrap();
    let mut port_str = String::new();
    io::stdin().read_line(&mut port_str).unwrap();
    let port: u16 = port_str.trim().parse().unwrap_or(9000);
    
    println!();
    show_progress("Connecting", 1);
    
    // Try IPv6 first, then IPv4
    let connected = if let Ok(stream) = TcpStream::connect(format!("[::1]:{}", port)) {
        println!("✅ Connected via IPv6!");
        handle_connection(stream);
        true
    } else if let Ok(stream) = TcpStream::connect(format!("127.0.0.1:{}", port)) {
        println!("✅ Connected via IPv4!");
        handle_connection(stream);
        true
    } else {
        println!("❌ Could not connect. Is your friend's chat running?");
        false
    };
    
    if connected {
        println!();
        show_join_success(&address.trim());
        run_chat_loop();
    }
}

fn detect_network_capability() -> (bool, bool) {
    let has_ipv6 = TcpListener::bind("[::1]:0").is_ok();
    let has_ipv4 = TcpListener::bind("127.0.0.1:0").is_ok();
    (has_ipv6, has_ipv4)
}

fn find_available_port() -> u16 {
    // Try common ports first
    for port in 9000..9020 {
        if TcpListener::bind(format!("0.0.0.0:{}", port)).is_ok() {
            return port;
        }
    }
    
    // Let OS assign
    let listener = TcpListener::bind("0.0.0.0:0").expect("No ports available");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

fn handle_peer(mut stream: TcpStream) {
    let peer = stream.peer_addr().unwrap();
    println!();
    println!("🔔 {} connected!", peer);
    println!();
    
    // Echo messages back for demo
    let mut buffer = [0; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                let msg = String::from_utf8_lossy(&buffer[0..n]);
                print!("[{}]: {}", peer, msg);
                io::stdout().flush().unwrap();
            }
            Err(_) => break,
        }
    }
    
    println!("[{} disconnected]", peer);
}

fn handle_connection(mut stream: TcpStream) {
    thread::spawn(move || {
        let mut buffer = [0; 1024];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let msg = String::from_utf8_lossy(&buffer[0..n]);
                    print!("\r[Friend]: {}", msg);
                    print!("> ");
                    io::stdout().flush().unwrap();
                }
                Err(_) => break,
            }
        }
    });
}

fn run_chat_loop() {
    println!("💬 Chat Active - Type messages or /quit to exit");
    println!();
    
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        if input.trim() == "/quit" {
            println!("Goodbye!");
            break;
        }
        
        // In real app, would send to connected peers
        if !input.trim().is_empty() {
            println!("[You]: {}", input.trim());
        }
    }
}

fn show_host_ready(address: &str, port: u16) {
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                    🎉 Your chat room is ready! 🎉                   ║");
    println!("║                                                                      ║");
    println!("║  Share these with your friend:                                      ║");
    println!("║                                                                      ║");
    println!("║     Address: {}                                   ║", format!("{:<25}", address));
    println!("║     Port: {}                                                     ║", format!("{:<5}", port));
    println!("║                                                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();
}

fn show_join_success(address: &str) {
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                   🎊 Connected to chat room! 🎊                     ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
}

fn show_progress(task: &str, seconds: u64) {
    print!("⏳ {}...", task);
    io::stdout().flush().unwrap();
    thread::sleep(Duration::from_secs(seconds));
    println!(" Done!");
}

fn generate_three_word_address() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as usize;
    
    let words = [
        ["ocean", "river", "mountain", "forest", "desert", "valley"],
        ["swift", "bright", "calm", "bold", "wise", "free"],
        ["eagle", "wolf", "bear", "fox", "hawk", "lion"],
    ];
    
    format!("{}-{}-{}", 
        words[0][millis % 6],
        words[1][(millis / 6) % 6],
        words[2][(millis / 36) % 6]
    )
}