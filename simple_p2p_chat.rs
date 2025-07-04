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
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    println!("🐜 P2P Foundation - Simple Chat");
    println!("================================");
    println!();
    
    // Check if bootstrap words were provided
    if args.len() > 2 && args[1] == "--bootstrap-words" {
        println!("🔗 Connecting to: {}", args[2]);
        println!("   (In a real implementation, this would connect to the P2P network)");
    } else {
        println!("🎯 Your three-word address: ocean.swift.mountain");
        println!("   Share this with friends so they can connect!");
    }
    
    println!();
    println!("💬 Chat commands:");
    println!("   /help   - Show help");
    println!("   /peers  - List connected peers");
    println!("   /quit   - Exit chat");
    println!("   Any other text will be sent as a message");
    println!();
    
    // Simple chat loop
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        let trimmed = input.trim();
        match trimmed {
            "/quit" => {
                println!("👋 Goodbye!");
                break;
            }
            "/help" => {
                println!("Commands:");
                println!("  /help  - Show this help");
                println!("  /peers - List connected peers");
                println!("  /quit  - Exit chat");
            }
            "/peers" => {
                println!("Connected peers:");
                println!("  • bootstrap.node (ocean.swift.mountain)");
                println!("  • peer1.node (river.quick.forest)");
                println!("  (In real implementation, would show actual peers)");
            }
            _ if !trimmed.is_empty() => {
                println!("[You]: {}", trimmed);
                println!("(Message would be sent to all connected peers)");
            }
            _ => {}
        }
    }
}
