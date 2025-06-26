
use p2p_foundation::{P2PNode, P2PEvent};
use p2p_foundation::bootstrap::{BootstrapManager, ThreeWordAddress};
use anyhow::Result;
use tokio::io::{self, AsyncBufReadExt};
use tokio::sync::mpsc;
use clap::Parser;

const CHAT_TOPIC: &str = "p2p-chat/1.0.0";

/// A simple P2P chat application.
#[derive(Parser, Debug)]
#[clap(name = "p2p-chat")]
struct Args {
    /// The address to listen on for incoming connections.
    #[clap(long, default_value = "/ip6/::/udp/0/quic")]
    listen_address: String,

    /// A peer to bootstrap from. Can be specified multiple times.
    #[clap(long)]
    bootstrap: Vec<String>,
    
    /// Bootstrap using three-word addresses (e.g., "global.fast.eagle").
    /// Much easier to share and remember than complex multiaddrs!
    #[clap(long)]
    bootstrap_words: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut builder = P2PNode::builder()
        .listen_on(&args.listen_address);

    // Handle traditional multiaddr bootstrap peers
    for peer in &args.bootstrap {
        builder = builder.with_bootstrap_peer(peer);
    }
    
    // Handle three-word bootstrap addresses
    if !args.bootstrap_words.is_empty() {
        let bootstrap_manager = BootstrapManager::new().await?;
        
        println!("[System] Processing three-word bootstrap addresses...");
        for word_addr in &args.bootstrap_words {
            match ThreeWordAddress::from_string(word_addr) {
                Ok(words) => {
                    match bootstrap_manager.validate_words(&words) {
                        Ok(()) => {
                            println!("[System] Valid three-word address: {}", words);
                            // Note: In a full implementation, we'd resolve this to a multiaddr
                            // For now, we'll use well-known addresses as examples
                            if word_addr == "global.fast.eagle" {
                                builder = builder.with_bootstrap_peer("/ip6/::1/tcp/9000");
                                println!("[System] Resolved {} to /ip6/::1/tcp/9000", words);
                            } else if word_addr == "local.mesh.lighthouse" {
                                builder = builder.with_bootstrap_peer("/ip6/::1/udp/9001/quic");
                                println!("[System] Resolved {} to /ip6/::1/udp/9001/quic", words);
                            } else {
                                println!("[System] ⚠️  {} is valid but not in our demo registry", words);
                                println!("[System] In production, this would be resolved via DHT lookup");
                            }
                        }
                        Err(e) => {
                            println!("[System] ❌ Invalid three-word address '{}': {}", word_addr, e);
                        }
                    }
                }
                Err(e) => {
                    println!("[System] ❌ Failed to parse three-word address '{}': {}", word_addr, e);
                }
            }
        }
    }
    
    let node = builder.build().await?;
    node.start().await?;

    // Subscribe to chat topic
    node.subscribe(CHAT_TOPIC).await?;

    // Channel to handle stdin
    let (tx, mut rx) = mpsc::channel::<String>(32);

    // Spawn a task to read from stdin
    tokio::spawn(async move {
        let mut stdin = io::BufReader::new(io::stdin()).lines();
        loop {
            match stdin.next_line().await {
                Ok(Some(line)) => {
                    if tx.send(line).await.is_err() {
                        println!("[System] Error sending message.");
                        break;
                    }
                }
                _ => break,
            }
        }
    });

    println!("[System] Welcome to P2P Chat!");
    println!("[System] Your Peer ID is: {}", node.peer_id());
    if let Some(addr) = node.local_addr() {
        println!("[System] Listening on: {}", addr);
        
        // Show three-word address for easy sharing
        if let Ok(multiaddr) = addr.parse() {
            let bootstrap_manager = BootstrapManager::new().await?;
            if let Ok(words) = bootstrap_manager.encode_address(&multiaddr) {
                println!("[System] 💬 Share-friendly address: {}", words);
                println!("[System] 📱 Tell friends: \"Connect to {}\"", words);
            }
        }
    }
    println!("[System] Type a message and press Enter to send.");
    println!("[System] Connecting to the network...");

    let mut events = node.subscribe_events();

    loop {
        tokio::select! {
            // Handle user input from stdin
            Some(line) = rx.recv() => {
                if !line.trim().is_empty() {
                    if let Err(e) = node.publish(CHAT_TOPIC, line.as_bytes()).await {
                        println!("[System] Failed to send message: {:?}", e);
                    }
                }
            }
            // Handle network events
            Ok(event) = events.recv() => {
                match event {
                    P2PEvent::Message { topic, source, data } => {
                        if topic == CHAT_TOPIC {
                            println!("[{}] {}", source, String::from_utf8_lossy(&data));
                        }
                    }
                    P2PEvent::PeerConnected(peer_id) => {
                        println!("[System] Peer connected: {}", peer_id);
                    }
                    P2PEvent::PeerDisconnected(peer_id) => {
                        println!("[System] Peer disconnected: {}", peer_id);
                    }
                }
            }
            // Break loop if both channels are closed
            else => {
                break;
            }
        }
    }

    Ok(())
}
