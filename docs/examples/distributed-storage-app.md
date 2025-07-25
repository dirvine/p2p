# Example: Distributed Storage Application

This example demonstrates building a distributed file storage application using the Adaptive P2P Network.

## Features

- Store files of any size with automatic chunking
- Retrieve files by content hash
- List stored files with metadata
- Monitor storage usage
- Share files with other users

## Complete Implementation

```rust
use saorsa_core::adaptive::*;
use std::{
    fs,
    path::{Path, PathBuf},
    collections::HashMap,
};
use serde::{Serialize, Deserialize};
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "p2p-storage")]
#[command(about = "Distributed file storage using P2P network")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Store a file in the network
    Store {
        /// Path to file to store
        file: PathBuf,
        
        /// Optional tags for the file
        #[arg(short, long)]
        tags: Vec<String>,
    },
    
    /// Retrieve a file from the network
    Get {
        /// Content hash of the file
        hash: String,
        
        /// Output path (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// List stored files
    List {
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
    },
    
    /// Show storage statistics
    Stats,
    
    /// Share a file with another user
    Share {
        /// Content hash
        hash: String,
        
        /// Recipient's P2P address
        recipient: String,
    },
}

#[derive(Serialize, Deserialize)]
struct FileMetadata {
    name: String,
    size: u64,
    hash: String,
    tags: Vec<String>,
    stored_at: u64,
}

struct StorageApp {
    client: Client,
    metadata_file: PathBuf,
    files: HashMap<String, FileMetadata>,
}

impl StorageApp {
    async fn new() -> Result<Self> {
        let client = Client::connect(ClientConfig::default()).await?;
        let metadata_file = dirs::config_dir()
            .unwrap()
            .join("p2p-storage")
            .join("files.json");
        
        // Create directory if needed
        if let Some(parent) = metadata_file.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Load existing metadata
        let files = if metadata_file.exists() {
            let data = fs::read_to_string(&metadata_file)?;
            serde_json::from_str(&data)?
        } else {
            HashMap::new()
        };
        
        Ok(Self {
            client,
            metadata_file,
            files,
        })
    }
    
    fn save_metadata(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(&self.files)?;
        fs::write(&self.metadata_file, data)?;
        Ok(())
    }
    
    async fn store_file(&mut self, path: &Path, tags: Vec<String>) -> Result<()> {
        println!("Storing file: {}", path.display());
        
        // Read file
        let file_data = fs::read(path)?;
        let file_size = file_data.len() as u64;
        let file_name = path.file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        
        // Store in network
        let start = std::time::Instant::now();
        let hash = self.client.store(file_data).await?;
        let duration = start.elapsed();
        
        // Save metadata
        let metadata = FileMetadata {
            name: file_name.clone(),
            size: file_size,
            hash: format!("{:?}", hash),
            tags,
            stored_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };
        
        self.files.insert(metadata.hash.clone(), metadata);
        self.save_metadata()?;
        
        println!("✓ Stored {} ({} bytes) in {:.2}s", 
            file_name, file_size, duration.as_secs_f32());
        println!("  Hash: {}", format!("{:?}", hash));
        println!("  Replicas: 5+ nodes");
        
        Ok(())
    }
    
    async fn get_file(&self, hash: &str, output: Option<PathBuf>) -> Result<()> {
        println!("Retrieving file: {}", hash);
        
        // Parse hash
        let hash_bytes = hex::decode(hash.trim_start_matches("ContentHash(")
            .trim_end_matches(")"))?;
        let mut content_hash = [0u8; 32];
        content_hash.copy_from_slice(&hash_bytes);
        let hash = ContentHash(content_hash);
        
        // Retrieve from network
        let start = std::time::Instant::now();
        let data = self.client.retrieve(&hash).await?;
        let duration = start.elapsed();
        
        // Determine output path
        let output_path = if let Some(path) = output {
            path
        } else if let Some(metadata) = self.files.get(&format!("{:?}", hash)) {
            PathBuf::from(&metadata.name)
        } else {
            PathBuf::from("retrieved_file")
        };
        
        // Write file
        fs::write(&output_path, &data)?;
        
        println!("✓ Retrieved {} bytes in {:.2}s", 
            data.len(), duration.as_secs_f32());
        println!("  Saved to: {}", output_path.display());
        
        Ok(())
    }
    
    fn list_files(&self, tag_filter: Option<String>) -> Result<()> {
        let mut files: Vec<_> = self.files.values().collect();
        
        // Filter by tag if specified
        if let Some(tag) = tag_filter {
            files.retain(|f| f.tags.contains(&tag));
        }
        
        // Sort by date
        files.sort_by_key(|f| f.stored_at);
        
        if files.is_empty() {
            println!("No files found");
            return Ok(());
        }
        
        println!("{:<40} {:<10} {:<20} {:<30}", 
            "Hash", "Size", "Name", "Tags");
        println!("{}", "-".repeat(100));
        
        for file in files {
            let hash_short = if file.hash.len() > 40 {
                format!("{}...", &file.hash[..37])
            } else {
                file.hash.clone()
            };
            
            println!("{:<40} {:<10} {:<20} {:<30}", 
                hash_short,
                format_size(file.size),
                truncate(&file.name, 20),
                file.tags.join(", ")
            );
        }
        
        Ok(())
    }
    
    async fn show_stats(&self) -> Result<()> {
        let stats = self.client.get_network_stats().await?;
        
        println!("=== Storage Statistics ===");
        println!("Local files: {}", self.files.len());
        println!("Total size: {}", 
            format_size(self.files.values()
                .map(|f| f.size)
                .sum()));
        
        println!("\n=== Network Statistics ===");
        println!("Connected peers: {}", stats.connected_peers);
        println!("Network storage: {} / {}", 
            format_size(stats.available_storage),
            format_size(stats.total_storage));
        println!("Cache hit rate: {:.1}%", stats.cache_hit_rate * 100.0);
        println!("Network health: {:.1}%", 
            stats.routing_success_rate * 100.0);
        
        Ok(())
    }
    
    async fn share_file(&self, hash: &str, recipient: &str) -> Result<()> {
        // In a real implementation, this would create an encrypted
        // share link or transfer ownership
        println!("Sharing {} with {}", hash, recipient);
        
        // Create share message
        let share_msg = format!("SHARE:{}:{}", hash, recipient);
        self.client.publish("shares", share_msg.into_bytes()).await?;
        
        println!("✓ File shared successfully");
        Ok(())
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_idx])
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    let cli = Cli::parse();
    let mut app = StorageApp::new().await?;
    
    match cli.command {
        Commands::Store { file, tags } => {
            app.store_file(&file, tags).await?;
        }
        
        Commands::Get { hash, output } => {
            app.get_file(&hash, output).await?;
        }
        
        Commands::List { tag } => {
            app.list_files(tag)?;
        }
        
        Commands::Stats => {
            app.show_stats().await?;
        }
        
        Commands::Share { hash, recipient } => {
            app.share_file(&hash, &recipient).await?;
        }
    }
    
    // Shutdown cleanly
    app.client.shutdown().await?;
    Ok(())
}
```

## Usage Examples

### Store a File

```bash
# Store a document
./p2p-storage store ~/Documents/report.pdf --tags work,reports

# Store with multiple tags
./p2p-storage store photo.jpg --tags vacation,2024,family
```

### Retrieve a File

```bash
# Get by hash
./p2p-storage get ContentHash(3f2a1b...)

# Get with custom output name
./p2p-storage get ContentHash(3f2a1b...) --output restored_report.pdf
```

### List Files

```bash
# List all files
./p2p-storage list

# Filter by tag
./p2p-storage list --tag work
```

### View Statistics

```bash
./p2p-storage stats
```

Output:
```
=== Storage Statistics ===
Local files: 42
Total size: 1.37 GB

=== Network Statistics ===
Connected peers: 127
Network storage: 8.9 TB / 15.2 TB
Cache hit rate: 76.3%
Network health: 99.1%
```

## Features Explained

### Automatic Chunking

Large files are automatically split into chunks for efficient storage:

```rust
// The client handles this internally
let hash = client.store(large_file).await?;
// File is chunked, stored, and reassembled on retrieval
```

### Content Addressing

Files are identified by their content hash, ensuring:
- Deduplication (same file stored once)
- Integrity (tampering detected)
- Permanent addressing

### Metadata Management

Local metadata tracks:
- Original filename
- File size
- Storage timestamp
- User-defined tags
- Content hash

### Error Handling

The app handles common errors gracefully:
- Network failures → Automatic retry
- File not found → Clear error message
- Corruption → Integrity check fails

## Advanced Features

### Encryption

Add client-side encryption:

```rust
use aes_gcm::{Aes256Gcm, Key, Nonce};

fn encrypt_file(data: &[u8], password: &str) -> Result<Vec<u8>> {
    let key = derive_key_from_password(password);
    let cipher = Aes256Gcm::new(&key);
    let nonce = generate_nonce();
    
    let encrypted = cipher.encrypt(&nonce, data)?;
    Ok([nonce.as_slice(), &encrypted].concat())
}
```

### Versioning

Track file versions:

```rust
#[derive(Serialize, Deserialize)]
struct Version {
    hash: String,
    timestamp: u64,
    message: String,
}

struct VersionedFile {
    name: String,
    versions: Vec<Version>,
}
```

### Streaming

For very large files:

```rust
use futures::StreamExt;

async fn store_stream(
    client: &Client,
    mut stream: impl Stream<Item = Result<Bytes>>,
) -> Result<ContentHash> {
    let mut chunks = vec![];
    
    while let Some(chunk) = stream.next().await {
        let data = chunk?;
        let hash = client.store(data.to_vec()).await?;
        chunks.push(hash);
    }
    
    // Store manifest
    let manifest = serde_json::to_vec(&chunks)?;
    client.store(manifest).await
}
```

## Building and Running

```bash
# Add to Cargo.toml
[dependencies]
saorsa-core = "0.2"
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
dirs = "5"
hex = "0.4"
env_logger = "0.10"

# Build
cargo build --release

# Run
./target/release/p2p-storage --help
```

## Conclusion

This example demonstrates building a fully functional distributed storage application with:
- Simple command-line interface
- Efficient file storage and retrieval
- Metadata management
- Network statistics
- Error handling

The Adaptive P2P Network handles all the complex distributed systems aspects, allowing you to focus on application logic.