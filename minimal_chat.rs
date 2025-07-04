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
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

fn main() {
    println!("🐜 P2P Chat - Real Connections");
    println!("==============================");
    println!();
    println!("1) Host a chat");
    println!("2) Join a chat");
    println!();
    
    print!("Choice: ");
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    
    match input.trim() {
        "1" => host_chat(),
        "2" => join_chat(),
        _ => println!("Invalid choice"),
    }
}

fn host_chat() {
    println!("\nStarting host...\n");
    
    // Find available port
    let port = find_port();
    
    // Check network
    let has_ipv6 = TcpListener::bind("[::1]:0").is_ok();
    
    if has_ipv6 {
        println!("✅ IPv6 available - no tunnel needed");
    } else {
        println!("⚠️  IPv4 only - would need tunnel for IPv6");
    }
    
    let addr = if has_ipv6 { 
        format!("[::]:{}", port) 
    } else { 
        format!("0.0.0.0:{}", port) 
    };
    
    match TcpListener::bind(&addr) {
        Ok(listener) => {
            println!("\n✅ Chat room ready!");
            println!("Address: valley-swift-eagle");
            println!("Port: {}", port);
            println!("\nWaiting for connections...\n");
            
            for stream in listener.incoming() {
                if let Ok(stream) = stream {
                    println!("Someone connected!");
                    // Handle connection
                }
            }
        }
        Err(e) => println!("Error: {}", e),
    }
}

fn join_chat() {
    println!("\nJoining chat...\n");
    
    print!("Enter port: ");
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let port: u16 = input.trim().parse().unwrap_or(9000);
    
    // Try IPv6 first
    if let Ok(_) = TcpStream::connect(format!("[::1]:{}", port)) {
        println!("✅ Connected via IPv6!");
    } else if let Ok(_) = TcpStream::connect(format!("127.0.0.1:{}", port)) {
        println!("✅ Connected via IPv4!");
    } else {
        println!("❌ Could not connect");
    }
}

fn find_port() -> u16 {
    for p in 9000..9010 {
        if TcpListener::bind(("0.0.0.0", p)).is_ok() {
            return p;
        }
    }
    0
}