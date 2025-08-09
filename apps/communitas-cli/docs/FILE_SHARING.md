# File Sharing Infrastructure

## Overview

The Communitas CLI includes a comprehensive file sharing infrastructure that enables secure peer-to-peer file transfers with trust-based permissions, chunking for large files, and robust integrity verification.

## Key Features

### 🔒 Security First
- **BLAKE3 Hashing**: All files and chunks verified with quantum-resistant BLAKE3 cryptographic hashing
- **Trust-Based Permissions**: Fine-grained access control with owner, trusted peers, and public sharing modes
- **Integrity Verification**: Every chunk verified on transfer with cryptographic guarantees

### 📦 Efficient Chunking
- **Configurable Chunk Sizes**: Default 1MB chunks, configurable up to 10MB
- **Resume Capability**: Interrupted transfers can be resumed from last successful chunk
- **Progress Tracking**: Real-time transfer progress with ETA calculations

### 🤝 Trust-Based Sharing
- **Owner Control**: File owners have complete access control
- **Trusted Peers**: Share with explicitly trusted network participants
- **Public Sharing**: Optional public access for community resources
- **Expiration Support**: Time-limited sharing permissions

## Architecture

### Core Components

```
File Sharing Infrastructure
├── FileMetadata        # File information and chunking
├── FileChunker        # File chunking and integrity
├── TransferSession    # Transfer state management
├── FileTransferManager # Transfer orchestration  
└── TransferProtocolHandler # Network protocol
```

### File Metadata System

The `FileMetadata` structure contains comprehensive information about shared files:

```rust
pub struct FileMetadata {
    pub id: Uuid,                    // Unique file identifier
    pub name: String,                // Original filename
    pub size: u64,                   // File size in bytes
    pub mime_type: String,           // MIME type detection
    pub blake3_hash: String,         // Full file BLAKE3 hash
    pub chunk_size: usize,           // Chunk size used
    pub chunk_count: usize,          // Total number of chunks
    pub chunk_hashes: Vec<String>,   // BLAKE3 hash of each chunk
    pub created_at: u64,             // Creation timestamp
    pub owner: FourWordAddress,      // File owner identity
    pub permissions: FilePermissions, // Access control settings
}
```

### Chunking System

Files are divided into chunks for efficient transfer:

- **Default Chunk Size**: 1MB (1,048,576 bytes)
- **Maximum Chunk Size**: 10MB (configurable)
- **Integrity**: Each chunk has individual BLAKE3 hash
- **Resume Support**: Track completion status per chunk

### Trust-Based Permissions

```rust
pub struct FilePermissions {
    pub public: bool,                        // Public access
    pub trusted_peers_only: bool,           // Restrict to trusted peers
    pub allowed_peers: Vec<FourWordAddress>, // Explicit allow list
    pub expires_at: Option<u64>,            // Optional expiration
}
```

Permission evaluation priority:
1. **Owner** - Always has full access
2. **Expiration** - Check if permissions expired
3. **Public** - Allow if public flag set
4. **Allow List** - Check explicit peer permissions
5. **Trusted Peers** - Allow if peer is trusted

## Usage Examples

### Basic File Sharing

```rust
use communitas_cli::communication::{
    FileTransferManager, FileMetadata, FourWordAddress
};

// Initialize transfer manager
let manager = FileTransferManager::new();

// Share a file
let owner = FourWordAddress::generate()?;
let peer = FourWordAddress::parse("alice-bob-charlie-dave")?;
let file_path = PathBuf::from("document.pdf");

// Start upload to peer
let session_id = manager.start_upload(file_path, peer, owner).await?;

// Monitor progress
let progress = manager.get_progress(session_id);
if let Some(progress) = progress {
    println!("Progress: {:.1}% ({}/{})", 
        progress.percentage, 
        progress.chunks_completed, 
        progress.chunks_total
    );
}
```

### Advanced Permission Control

```rust
// Create file metadata with custom permissions
let mut metadata = FileMetadata::from_file(file_path, owner).await?;

// Configure trust-based sharing
metadata.permissions = FilePermissions {
    public: false,                    // Not public
    trusted_peers_only: false,       // Allow specific peers
    allowed_peers: vec![
        peer1, peer2, peer3          // Explicit allow list
    ],
    expires_at: Some(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 3600
    ), // Expires in 1 hour
};

// Verify permissions before sharing
if metadata.has_permission(&requesting_peer, false) {
    // Proceed with transfer
}
```

### Transfer Session Management

```rust
// Pause active transfer
manager.pause_transfer(session_id)?;

// Resume paused transfer
manager.resume_transfer(session_id)?;

// Cancel transfer
manager.cancel_transfer(session_id)?;

// Get all active transfers
let active = manager.get_active_transfers();
for session in active {
    println!("Transfer {} to {}: {:?}", 
        session.file_metadata.name,
        session.peer,
        session.status
    );
}
```

## Protocol Messages

The transfer protocol defines several message types for coordination:

### Transfer Request
```rust
TransferMessage::TransferRequest {
    file_metadata: FileMetadata,
    request_id: Uuid,
}
```

### Chunk Transfer
```rust
TransferMessage::ChunkRequest {
    file_id: Uuid,
    chunk_index: usize, 
    request_id: Uuid,
}

TransferMessage::ChunkResponse {
    request_id: Uuid,
    chunk_data: Vec<u8>,
    chunk_metadata: ChunkMetadata,
}
```

### Transfer Control
```rust
TransferMessage::TransferCancel {
    file_id: Uuid,
    reason: String,
}

TransferMessage::Heartbeat {
    session_id: Uuid,
}
```

## Configuration

### Transfer Manager Settings

```rust
let mut manager = FileTransferManager::new();

// Configure concurrent transfers
manager.set_max_concurrent_transfers(10);

// Set chunk timeout
manager.set_chunk_timeout(Duration::from_secs(60));
```

### Protocol Handler Settings

```rust
let mut handler = TransferProtocolHandler::new();

// Configure maximum chunk size
handler.set_max_chunk_size(5 * 1024 * 1024); // 5MB

// Set session timeout
handler.set_session_timeout(600); // 10 minutes
```

## Integration Points

### With Communication System

File sharing integrates with the existing communication infrastructure:

```rust
// In CommunicationManager
pub struct CommunicationManager {
    // ... existing fields
    file_transfer_manager: Option<FileTransferManager>,
    transfer_protocol_handler: Option<TransferProtocolHandler>,
}
```

### With Identity System

File sharing uses the four-word address system:

- **Owner Identity**: Files are owned by specific four-word addresses
- **Peer Authentication**: Transfer permissions verified against peer identity
- **Trust Networks**: Integration with existing peer trust relationships

### With Storage System

File metadata can be persisted:

- **Session Recovery**: Resume transfers after restarts
- **Permission Caching**: Cache permission decisions for performance
- **Transfer History**: Track completed and failed transfers

## Error Handling

The system includes comprehensive error handling:

```rust
// File operations
match FileMetadata::from_file(path, owner).await {
    Ok(metadata) => { /* proceed */ },
    Err(e) => eprintln!("Failed to create metadata: {}", e),
}

// Transfer operations  
match manager.start_upload(file, peer, owner).await {
    Ok(session_id) => { /* monitor progress */ },
    Err(e) => eprintln!("Transfer failed to start: {}", e),
}

// Permission checks
if !metadata.has_permission(&peer, is_trusted) {
    return Err(anyhow::anyhow!("Permission denied for peer {}", peer));
}
```

## Security Considerations

### Cryptographic Security
- **BLAKE3 Hashing**: Quantum-resistant cryptographic integrity
- **Hash Verification**: Every chunk verified before acceptance
- **File Integrity**: Complete file hash verified on completion

### Access Control
- **Owner Authority**: File owners have complete control
- **Expiration Enforcement**: Time-limited permissions respected  
- **Trust Boundaries**: Clear distinction between trusted and untrusted peers

### Network Security
- **Protocol Messages**: All messages include request tracking
- **Session Management**: Transfer sessions have configurable timeouts
- **Resource Limits**: Configurable limits on concurrent transfers and chunk sizes

## Performance Characteristics

### Scalability
- **Concurrent Transfers**: Support for multiple simultaneous transfers
- **Chunk Parallelization**: Individual chunks can be requested in parallel
- **Memory Efficiency**: Streaming chunk processing without loading entire files

### Network Efficiency
- **Resume Capability**: Avoid re-transferring completed chunks
- **Progress Tracking**: Efficient progress calculation without scanning
- **Connection Reuse**: Protocol designed for connection pooling

### Storage Efficiency
- **Streaming Processing**: Process chunks without temporary storage
- **Metadata Caching**: Efficient metadata lookups and validation
- **Cleanup Management**: Automatic cleanup of completed transfers

## Testing

The file sharing system includes comprehensive test coverage:

- **Unit Tests**: 42 tests covering all components
- **Integration Tests**: Protocol message flows and session management
- **Error Scenarios**: Permission denial, transfer cancellation, timeout handling
- **Performance Tests**: Large file handling and concurrent transfer scenarios

### Test Categories

1. **File Metadata** (11 tests)
   - File analysis and chunking
   - Permission evaluation
   - Hash verification

2. **Transfer Management** (13 tests)
   - Session creation and control
   - Progress tracking
   - Transfer lifecycle

3. **Protocol Handling** (18 tests)
   - Message processing
   - Session management
   - Network protocol flows

## Future Enhancements

### Planned Features
- **Bandwidth Limiting**: Configurable transfer rate limits
- **Priority Queuing**: Priority-based transfer scheduling
- **Compression**: Optional chunk compression for text files
- **Encryption**: End-to-end encryption for sensitive files

### Protocol Extensions
- **Batch Operations**: Transfer multiple files in single session
- **Delta Sync**: Incremental updates for changed files
- **Multi-Source**: Download chunks from multiple peers simultaneously
- **Merkle Trees**: Hierarchical integrity verification for large files

---

*This documentation covers the complete file sharing infrastructure implementation in Communitas CLI v0.1.0. For implementation details, see the source code in `src/communication/file_*` modules.*