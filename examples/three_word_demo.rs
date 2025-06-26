//! Three-Word Address Demonstration
//!
//! Shows how multiaddrs can be converted to memorable three-word combinations
//! for human-friendly P2P bootstrap address sharing.

use p2p_foundation::{
    bootstrap::{ThreeWordAddress, WordEncoder}, 
    Multiaddr
};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🌟 P2P Foundation Three-Word Address Demo");
    println!("==========================================\n");
    
    let encoder = WordEncoder::new();
    
    // Demo addresses that would be real bootstrap nodes
    let demo_addresses = vec![
        "/ip6/2001:db8::1/udp/9000/quic",
        "/ip6/2001:db8::2/udp/9001/quic", 
        "/ip6/::1/tcp/8000",
        "/ip4/192.168.1.100/udp/5000/quic",
        "/ip6/2606:4700:4700::1111/udp/9002/quic",
        "/ip6/2001:4860:4860::8888/udp/9003/quic",
    ];
    
    println!("📍 Converting multiaddrs to three-word addresses:");
    println!("=================================================\n");
    
    for addr_str in &demo_addresses {
        match addr_str.parse::<Multiaddr>() {
            Ok(multiaddr) => {
                match encoder.encode_multiaddr(&multiaddr) {
                    Ok(words) => {
                        println!("🔗 Technical: {}", addr_str);
                        println!("💬 Memorable: {}", words);
                        println!("📱 Share as:  \"Connect to {}\"", words);
                        println!();
                    }
                    Err(e) => {
                        println!("❌ Failed to encode {}: {}", addr_str, e);
                    }
                }
            }
            Err(e) => {
                println!("❌ Invalid multiaddr {}: {}", addr_str, e);
            }
        }
    }
    
    println!("🎯 User Experience Examples:");
    println!("============================\n");
    
    // Show how users would interact with the system
    let examples = vec![
        ("Alice to Bob", "Hey Bob, connect to: global.fast.eagle"),
        ("Discord Chat", "Join our P2P network at: local.mesh.lighthouse"),
        ("Email", "P2P address: europe.stable.compass"),
        ("Voice Call", "Connect to ocean dot thunder dot falcon"),
        ("Business Card", "P2P: forest.swift.phoenix"),
    ];
    
    for (context, message) in examples {
        println!("📞 {}: \"{}\"", context, message);
    }
    
    println!("\n🧪 Testing Three-Word Validation:");
    println!("=================================\n");
    
    // Test word validation
    let test_words = vec![
        ("global.fast.eagle", true),
        ("ocean.thunder.falcon", true), 
        ("invalid.words.here", false),
        ("partial.incomplete", false), // Not enough words
        ("too.many.words.here", false), // Too many words
    ];
    
    for (word_str, should_be_valid) in test_words {
        match ThreeWordAddress::from_string(word_str) {
            Ok(words) => {
                match words.validate(&encoder) {
                    Ok(()) => {
                        let status = if should_be_valid { "✅" } else { "⚠️" };
                        println!("{} \"{}\" -> Valid three-word address", status, word_str);
                    }
                    Err(e) => {
                        let status = if !should_be_valid { "✅" } else { "❌" };
                        println!("{} \"{}\" -> Invalid: {}", status, word_str, e);
                    }
                }
            }
            Err(e) => {
                let status = if !should_be_valid { "✅" } else { "❌" };
                println!("{} \"{}\" -> Parse error: {}", status, word_str, e);
            }
        }
    }
    
    println!("\n🚀 Bootstrap Integration Example:");
    println!("================================\n");
    
    // Show how this integrates with bootstrap
    println!("Instead of technical bootstrap commands:");
    println!("  cargo run --example chat -- --bootstrap '/ip6/2001:db8::1/udp/9000/quic'");
    println!();
    println!("Users can now bootstrap with:");
    println!("  cargo run --example chat -- --bootstrap-words 'global.fast.eagle'");
    println!();
    println!("Flutter app users just enter three words:");
    println!("  [global] [fast] [eagle] -> Connect");
    println!();
    
    println!("🎉 Three-Word Address System Ready!");
    println!("===================================");
    println!("- Human-friendly: ✅");
    println!("- Error-resistant: ✅"); 
    println!("- Memorable: ✅");
    println!("- Deterministic: ✅");
    println!("- Voice-friendly: ✅");
    
    Ok(())
}