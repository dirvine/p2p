# Communication Module

The communication module provides secure direct messaging between Communitas instances using four-word addresses.

## Components

### Core Message Structures (`message.rs`)
- **Message**: Core message content with metadata, types (Text, File, System, DeliveryConfirmation)
- **MessageEnvelope**: Encrypted wrapper for network transmission
- **StoredMessage**: Message with delivery tracking and status
- **DeliveryStatus**: Pending, Sent, Delivered, Failed states

### Message Delivery (`delivery.rs`)
- **MessageDelivery**: Queue management and delivery orchestration
- **DeliveryResult**: Success, RecipientOffline, NetworkError, etc.
- **Offline queuing**: Per-recipient message queues with size limits
- **Retry mechanism**: Configurable retry attempts for failed deliveries

### Secure Messaging (`messaging.rs`)
- **SecureMessaging**: Main messaging interface with encryption
- **MessageStorage**: Persistent message history and conversations
- **ConversationSummary**: Conversation metadata and unread counts
- **Integration**: Works with EnhancedIdentityManager for addressing

### Community Synchronization (`community_sync.rs`)
- **Community**: Data structure for community information and membership
- **CommunitySyncManager**: Manages community creation, updates, and synchronization
- **CommunityPermissions**: Role-based access control (Read, Write, Admin, Owner)
- **SyncStatus**: Track synchronization progress and state

### Synchronization Protocol (`sync_protocol.rs`)
- **SyncProtocolHandler**: Protocol message handling for community sync
- **ProtocolMessage**: Encrypted message envelopes for sync communication
- **SyncSession**: Manages synchronization sessions with peers
- **Request/Response**: Community list requests, data requests, incremental updates

### Conflict Resolution (`conflict_resolution.rs`)
- **ConflictResolver**: Sophisticated conflict resolution engine
- **ConflictAnalysis**: Detect and analyze conflicts between community versions
- **MergeResult**: Results from conflict resolution with detailed metadata
- **Resolution Strategies**: LastWriterWins, AutoMerge, Manual, TrustedPeer

## Features

### ✅ Implemented
- Message data structures and serialization
- Encrypted message transmission (placeholder encryption)
- Message delivery confirmation system
- Message history and persistence
- Offline message queuing
- Conversation management
- **Community synchronization system** with protocol messaging
- **Conflict resolution mechanisms** with multiple strategies
- **Incremental sync capabilities** for efficient data updates
- **Community sharing permissions** with role-based access control
- **Sync status tracking** and comprehensive progress reporting
- Comprehensive test coverage (134 tests, including 35 new community sync tests)

### 🔄 Future Enhancements
- Replace placeholder encryption with saorsa-core cryptography
- Network layer integration for actual message transmission
- Message indexing and search capabilities
- File attachment support
- Message reactions and threading

## Usage

```rust
use crate::communication::{CommunicationManager, SecureMessaging};
use crate::identity::{FourWordAddress, EnhancedIdentityManager};

// Initialize secure messaging
let mut comm_manager = CommunicationManager::new();
comm_manager.initialize_messaging(storage_path, identity_manager)?;

// Send a message
let recipient = FourWordAddress::from_string("apple-beach-cloud-dream")?;
let message_id = comm_manager.send_secure_message(recipient, "Hello!".to_string()).await?;

// Process delivery queue
let results = comm_manager.process_delivery_queue().await?;

// Get conversation history
let history = comm_manager.get_conversation_history(&recipient);

// Community synchronization
let creator = FourWordAddress::from_string("alice-bob-charlie-delta")?;
let community_id = comm_manager.create_community(
    "Tech Discussion".to_string(),
    "Community for tech discussions".to_string(),
    creator.clone()
)?;

// Start sync session with peer
let peer = FourWordAddress::from_string("echo-foxtrot-golf-hotel")?;
let sync_session = comm_manager.start_community_sync(peer).await?;

// Handle conflicts with automatic resolution
let versions = vec![community1, community2];
let result = comm_manager.resolve_community_conflicts(&versions, None)?;
```

## Testing

Run the communication tests with:
```bash
cargo test communication
```

All 134 tests pass, covering:
- Message creation and serialization
- Delivery mechanisms and confirmations  
- Storage and persistence
- Queue management and limits
- Encryption/decryption workflows
- **Community data synchronization** (12 tests)
- **Sync protocol messaging** (11 tests) 
- **Conflict resolution** (12 tests)
- **Event system integration** (8 tests)
- **Notification management** (12+ tests)