#!/bin/bash
# Create a simple standalone chat application

echo "🔨 Creating simple P2P chat application..."

cat > simple_p2p_chat.rs << 'EOF'
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
EOF

# Compile the simple chat
echo "📦 Compiling simple chat..."
if rustc simple_p2p_chat.rs -o p2p-chat-simple 2>/dev/null; then
    echo "✅ Simple chat compiled successfully!"
    echo "   Binary: ./p2p-chat-simple"
    echo ""
    echo "Usage:"
    echo "  ./p2p-chat-simple                                    # Start with your address"
    echo "  ./p2p-chat-simple --bootstrap-words ocean.swift.mountain  # Connect to friend"
else
    echo "❌ Failed to compile simple chat"
fi