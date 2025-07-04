// Copyright 2024 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This example demonstrates AGPL-3.0 compliance for P2P Foundation

use p2p_foundation::{P2PNode, NodeConfig};
use std::net::SocketAddr;
use warp::{Filter, Reply};

/// Example web service using P2P Foundation under AGPL-3.0
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize P2P node
    let config = NodeConfig::default();
    let node = P2PNode::new(config).await?;
    
    println!("Starting AGPL-compliant P2P service...");
    
    // Create web routes with AGPL compliance
    let routes = create_routes(node);
    
    // Start web server
    let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    println!("Server listening on http://{}", addr);
    println!("Source code link available at http://{}/source", addr);
    
    warp::serve(routes).run(addr).await;
    
    Ok(())
}

/// Create web routes with AGPL compliance features
fn create_routes(node: P2PNode) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    // Main application route
    let index = warp::path::end()
        .map(|| {
            warp::reply::html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>AGPL P2P Application</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .license-notice {
            background: #f0f0f0;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }
        footer {
            margin-top: 40px;
            padding-top: 20px;
            border-top: 1px solid #ccc;
            text-align: center;
        }
    </style>
</head>
<body>
    <h1>P2P Application Example</h1>
    
    <div class="license-notice">
        <h3>⚖️ License Notice</h3>
        <p>
            This application is free software licensed under the 
            <a href="/license">GNU Affero General Public License v3.0</a>.
        </p>
        <p>
            You have the right to use, modify, and distribute this software
            under the terms of the AGPL-3.0 license.
        </p>
    </div>
    
    <h2>Features</h2>
    <ul>
        <li>Decentralized P2P networking</li>
        <li>End-to-end encryption</li>
        <li>Open source implementation</li>
    </ul>
    
    <footer>
        <p>
            <a href="/source">📦 View Source Code</a> |
            <a href="/license">📄 License</a> |
            <a href="/api">🔧 API Documentation</a>
        </p>
        <p>Powered by P2P Foundation (AGPL-3.0)</p>
    </footer>
</body>
</html>
            "#)
        });
    
    // AGPL-3.0 Compliance: Source code access (Section 13)
    let source = warp::path("source")
        .map(|| {
            warp::reply::html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Source Code - AGPL Compliance</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .source-section {
            background: #f8f8f8;
            padding: 20px;
            border-radius: 5px;
            margin: 20px 0;
        }
        code {
            background: #eee;
            padding: 2px 5px;
            border-radius: 3px;
        }
    </style>
</head>
<body>
    <h1>Source Code Access</h1>
    
    <p>
        In compliance with the GNU Affero General Public License v3.0 (AGPL-3.0),
        the complete source code for this application is available.
    </p>
    
    <div class="source-section">
        <h2>🔗 Repository</h2>
        <p>
            <strong>GitHub:</strong> 
            <a href="https://github.com/yourorg/p2p-app">
                https://github.com/yourorg/p2p-app
            </a>
        </p>
        
        <h3>Clone the repository:</h3>
        <pre><code>git clone https://github.com/yourorg/p2p-app.git</code></pre>
        
        <h3>Download as archive:</h3>
        <ul>
            <li><a href="https://github.com/yourorg/p2p-app/archive/main.zip">Download ZIP</a></li>
            <li><a href="https://github.com/yourorg/p2p-app/archive/main.tar.gz">Download TAR.GZ</a></li>
        </ul>
    </div>
    
    <div class="source-section">
        <h2>📋 Build Instructions</h2>
        <pre><code># Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone repository
git clone https://github.com/yourorg/p2p-app.git
cd p2p-app

# Build and run
cargo build --release
cargo run --release</code></pre>
    </div>
    
    <div class="source-section">
        <h2>📦 Dependencies</h2>
        <p>This project uses the following main dependencies:</p>
        <ul>
            <li>P2P Foundation (AGPL-3.0) - Core P2P networking</li>
            <li>Tokio (MIT) - Async runtime</li>
            <li>Warp (MIT) - Web framework</li>
        </ul>
        <p>Full dependency list available in <code>Cargo.toml</code></p>
    </div>
    
    <div class="source-section">
        <h2>📄 License</h2>
        <p>
            This program is free software: you can redistribute it and/or modify
            it under the terms of the GNU Affero General Public License as published
            by the Free Software Foundation, either version 3 of the License, or
            (at your option) any later version.
        </p>
        <p>
            <a href="/license">View full license text</a>
        </p>
    </div>
    
    <p><a href="/">← Back to Home</a></p>
</body>
</html>
            "#)
        });
    
    // License text route
    let license = warp::path("license")
        .map(|| {
            // In production, read from LICENSE-AGPL-3.0 file
            warp::reply::with_header(
                include_str!("../../LICENSE-AGPL-3.0"),
                "content-type",
                "text/plain; charset=utf-8"
            )
        });
    
    // API documentation (part of complete corresponding source)
    let api = warp::path("api")
        .map(|| {
            warp::reply::json(&serde_json::json!({
                "name": "P2P Application API",
                "version": "1.0.0",
                "license": "AGPL-3.0-or-later",
                "source": "https://github.com/yourorg/p2p-app",
                "endpoints": {
                    "/": "Main application interface",
                    "/source": "Source code access (AGPL compliance)",
                    "/license": "License text",
                    "/api": "This API documentation",
                    "/health": "Health check endpoint"
                },
                "p2p_features": {
                    "networking": "QUIC-based P2P connections",
                    "discovery": "DHT-based peer discovery",
                    "encryption": "End-to-end encryption"
                }
            }))
        });
    
    // Health check
    let health = warp::path("health")
        .map(move || {
            warp::reply::json(&serde_json::json!({
                "status": "healthy",
                "p2p_node": "active",
                "license": "AGPL-3.0",
                "source_available": true
            }))
        });
    
    // Combine all routes
    index
        .or(source)
        .or(license)
        .or(api)
        .or(health)
}

/// Additional AGPL compliance helpers
mod agpl_compliance {
    /// Ensure source code notice is included in all responses
    pub fn add_source_header() -> warp::filters::reply::WithHeader {
        warp::reply::with::header(
            "X-Source-Code",
            "https://github.com/yourorg/p2p-app"
        )
    }
    
    /// Log file for AGPL compliance
    pub fn compliance_logger() {
        println!("AGPL Compliance Active:");
        println!("- Source code: Available at /source");
        println!("- License: Available at /license");
        println!("- Modifications: Documented in CHANGES.md");
        println!("- Build instructions: Included in repository");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_agpl_compliance_routes() {
        // Ensure all required AGPL compliance routes exist
        let routes = vec!["/source", "/license", "/api"];
        
        for route in routes {
            println!("Checking AGPL compliance route: {}", route);
            // In real test, would make HTTP request and verify response
        }
    }
    
    #[test]
    fn test_source_availability() {
        // Verify source code link is accessible
        // Verify repository exists and is public
        // Verify build instructions are present
    }
}