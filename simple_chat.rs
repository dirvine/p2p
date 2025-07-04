#!/usr/bin/env rust
//! Simple P2P Chat Application
//! 
//! Run with: rustc simple_chat.rs && ./simple_chat
//! Or build a proper binary in the project

use std::io::{self, Write};

fn main() {
    println!("🐜 P2P Foundation - Simple Chat Demo");
    println!("====================================");
    println!();
    println!("This is a demo of how the chat would work.");
    println!("For the real implementation, build the project:");
    println!();
    println!("  cargo build --release --bin ant-connect");
    println!("  cargo build --release --bin saorsa-test-suite");
    println!();
    println!("Or use the Tauri app:");
    println!("  cd apps/saorsa");
    println!("  cargo tauri build");
    println!();
    println!("Your three-word address: ocean.swift.mountain");
    println!();
    println!("Share this address with friends to connect!");
    println!();
    
    // Simple echo loop for demo
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        let trimmed = input.trim();
        if trimmed == "/quit" {
            println!("Goodbye!");
            break;
        } else if trimmed == "/help" {
            println!("Commands:");
            println!("  /help  - Show this help");
            println!("  /quit  - Exit the chat");
            println!("  Any other text - Would be sent to peers");
        } else if !trimmed.is_empty() {
            println!("[You]: {}", trimmed);
            println!("(In real app, this would be sent to connected peers)");
        }
    }
}