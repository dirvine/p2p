# Saorsa Desktop Application

**🕊️ The flagship P2P messaging application powered by Ant Core**

Saorsa (pronounced "SEER-sha", Irish for "freedom") is a revolutionary desktop messaging application that demonstrates the full potential of the P2P Foundation ecosystem. Built with Tauri, it provides native desktop performance with a modern web UI while delivering true peer-to-peer communication with zero-friction onboarding.

## ✨ Revolutionary Features

### 🤖 AI-Powered Cryptocurrency Management
- **Invisible Wallet Operations**: AI handles all cryptocurrency operations behind the scenes
- **Zero Crypto Knowledge Required**: Users never see wallets, private keys, or token balances
- **Automatic Economic Participation**: Earn ANT tokens by contributing storage, spend automatically for features
- **Seamless Fiat Integration**: Simple "Add Credits" button when needed - AI handles exchange setup

### 🔒 Privacy-First Architecture
- **Encrypted by Default**: All profile data encrypted with AES-256-GCM
- **Friend-Based Sharing**: Granular control over what friends can see
- **Local AI Processing**: All sensitive operations happen on your device
- **Zero Data Collection**: No tracking, analytics, or external data harvesting

### 🌐 True Peer-to-Peer Communication
- **No Central Servers**: Direct encrypted communication between devices
- **Three-Word Addresses**: Share `alice.secure.network` instead of complex addresses
- **Universal Connectivity**: Works on any network through intelligent tunneling
- **Cross-Device Sync**: Access your data from any device worldwide

### 👥 Comprehensive Contact Management (v0.2.0)
- **Smart Contact Organization**: Categorize contacts (Friends, Family, Work)
- **Enhanced Privacy Controls**: Per-contact permissions for profile visibility
- **Contact Blocking**: Block unwanted contacts with visual indicators
- **Rich Contact Profiles**: Add nicknames, notes, and trust levels
- **Right-Click Context Menu**: Quick access to contact actions
- **Bulk Operations**: Manage multiple contacts efficiently

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────┐
│         Saorsa User Interface          │  ← Modern chat interface
│  ┌─────────────────────────────────────┐ │
│  │ • Real-time messaging UI           │ │
│  │ • Contact management               │ │  
│  │ • Profile settings                 │ │
│  │ • "Network Credits" status         │ │  ← No crypto terminology
│  └─────────────────────────────────────┘ │
├─────────────────────────────────────────┤
│           Local AI Model               │  ← Revolutionary crypto UX
│  ┌─────────────────────────────────────┐ │
│  │ • Ed25519 Wallet Management        │ │  ← 100% local, secure
│  │ • ANT Token Earning/Spending       │ │  ← Automatic optimization
│  │ • Fiat-to-Crypto Integration       │ │  ← Seamless purchases
│  │ • Economic Decision Making         │ │  ← AI optimizes usage
│  └─────────────────────────────────────┘ │
├─────────────────────────────────────────┤
│          Tauri Application Layer       │  ← Rust backend
│  ┌─────────────────────────────────────┐ │
│  │ • P2P Node Management              │ │
│  │ • Identity & Profile Management    │ │
│  │ • Message Encryption/Decryption    │ │
│  │ • DHT Storage Operations           │ │
│  └─────────────────────────────────────┘ │
├─────────────────────────────────────────┤
│           Ant Core Library             │  ← P2P Foundation
│  ┌─────────────────────────────────────┐ │
│  │ • QUIC Transport & NAT Traversal   │ │
│  │ • Kademlia DHT for Storage         │ │
│  │ • Privacy-First Identity System    │ │
│  │ • Three-Word Address Resolution    │ │
│  └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

## 🚀 Getting Started

### Prerequisites
- **Rust** 1.75+ with Tauri CLI: `cargo install tauri-cli`
- **Node.js** 18+ for frontend development
- **Platform**: macOS 11+, Windows 10+, or Linux (Ubuntu 20.04+)

### Development Setup

```bash
# Clone the repository
git clone https://github.com/dirvine/p2p.git
cd p2p/apps/desktop-tauri

# Install frontend dependencies
npm install

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

### First Launch Experience

1. **Download AI Model**: App downloads personalized AI model (1-10GB)
2. **Create Profile**: Choose display name and three-word address
3. **Start Chatting**: Full functionality available immediately
4. **Invisible Setup**: AI creates wallet and begins earning tokens automatically

## 🧠 AI Integration Architecture

### Local AI Model Responsibilities

```rust
/// AI-powered wallet that manages all cryptocurrency operations invisibly
pub struct LocalAIWallet {
    // Cryptographic identity (never leaves device)
    private_key: [u8; 32],           // Ed25519 private key
    public_key: [u8; 32],            // Ed25519 public key
    
    // Economic management
    ant_balance: u64,                // Current ANT token balance
    earning_rate: f64,               // Tokens earned per hour
    spending_patterns: Vec<Transaction>,
    
    // Fiat integration
    exchange_accounts: Vec<ExchangeConfig>,
    preferred_payment_methods: Vec<PaymentMethod>,
    
    // Decision making
    economic_optimizer: EconomicOptimizer,
    usage_predictor: UsagePredictor,
}

impl LocalAIWallet {
    /// Generate secure wallet automatically
    async fn create_secure_wallet() -> Result<Self>;
    
    /// Earn tokens for providing storage to network
    async fn earn_tokens_for_storage(&mut self, bytes: u64) -> Result<()>;
    
    /// Spend tokens for data replication and features
    async fn spend_tokens_for_features(&mut self, feature: Feature) -> Result<()>;
    
    /// Setup fiat purchasing when needed
    async fn setup_fiat_integration(&mut self, config: FiatConfig) -> Result<()>;
    
    /// Make economic decisions automatically
    async fn optimize_token_usage(&mut self) -> Result<OptimizationResult>;
    
    /// Handle insufficient tokens scenario
    async fn handle_low_credits(&mut self) -> Result<CreditSolution>;
}
```

### Invisible User Experience

```javascript
// Frontend JavaScript - No crypto complexity exposed
class SaorsaApp {
    async sendMessage(friendId, message) {
        // AI automatically handles any token costs
        const result = await invoke('send_encrypted_message', {
            friendId,
            message,
        });
        
        if (result.needsCredits) {
            // Show simple "Add Credits" dialog
            this.showAddCreditsDialog(result.estimatedCost);
        } else {
            // Message sent successfully
            this.displayMessage(message, 'sent');
        }
    }
    
    async enableCrossDevice() {
        // AI evaluates cost and manages payment
        const result = await invoke('enable_cross_device_sync');
        
        switch (result.status) {
            case 'enabled':
                this.showSuccess('Your data is now available on all devices!');
                break;
            case 'needsCredits':
                this.showAddCreditsDialog(result.cost);
                break;
            case 'processing':
                this.showProcessing('Setting up cross-device access...');
                break;
        }
    }
    
    async addCredits(amountUSD) {
        // AI handles entire fiat-to-crypto flow
        const result = await invoke('purchase_network_credits', { amountUSD });
        this.showCreditsAdded(result.creditsReceived);
    }
}
```

## 🎨 User Interface Design

### Modern Chat Interface
```html
<!-- Real-time messaging with emoji support -->
<div class="chat-interface">
    <div class="message-list" id="messageList">
        <!-- Messages populated dynamically -->
    </div>
    
    <div class="message-input">
        <input type="text" id="messageInput" placeholder="Type your message...">
        <button id="emojiButton">😊</button>
        <button id="sendButton">Send</button>
    </div>
</div>

<!-- Network status (no crypto terminology) -->
<div class="network-status">
    <span class="status-indicator connected"></span>
    <span>Network Credits: Sufficient</span>
    <button class="add-credits" style="display: none;">Add Credits</button>
</div>
```

### Profile Management with Privacy Controls
```html
<!-- Profile management with granular privacy -->
<div class="profile-management">
    <h2>👤 Your Profile</h2>
    
    <div class="profile-field">
        <label>Display Name</label>
        <input type="text" id="displayName" value="Alice">
    </div>
    
    <div class="profile-field">
        <label>Three-Word Address</label>
        <input type="text" id="threeWordAddress" value="alice.secure.network" readonly>
        <button id="copyAddress">Copy</button>
        <button id="shareQR">QR Code</button>
    </div>
    
    <h3>🔒 Privacy Settings</h3>
    <div class="privacy-controls">
        <div class="privacy-item">
            <label>Profile Photo</label>
            <select id="photoVisibility">
                <option value="private">Private</option>
                <option value="friends">Friends Only</option>
                <option value="public">Public</option>
            </select>
        </div>
        
        <div class="privacy-item">
            <label>Online Status</label>
            <select id="statusVisibility">
                <option value="nobody">Nobody</option>
                <option value="friends">Friends Only</option>
            </select>
        </div>
    </div>
</div>
```

## 📦 Installation

### Option 1: Pre-built Binaries (Recommended)
Download the latest release for your platform from:
- 🔗 [GitHub Releases](https://github.com/dirvine/p2p/releases)

### Option 2: Build from Source
```bash
# Clone the repository
git clone https://github.com/dirvine/p2p
cd p2p/apps/desktop-tauri

# Install dependencies and build
npm install
npm run tauri build

# The binary will be in src-tauri/target/release/
```

### Option 3: From crates.io (v0.2.2+)
```bash
# Install with bundled frontend
cargo install saorsa --features bundle-frontend

# Run the app
saorsa
```

### Option 4: As a Rust Library
```toml
# In your Cargo.toml
[dependencies]
saorsa = "0.2.2"
```

## 🔧 Technical Implementation

### Tauri Commands for Frontend-Backend Communication
```rust
// Core Tauri commands that power the UI
#[tauri::command]
async fn create_profile(
    name: String,
    three_word_address: String,
) -> Result<UserProfile, String> {
    // AI wallet automatically created behind the scenes
    let identity = IDENTITY_MANAGER
        .create_identity(name, three_word_address, None, None)
        .await
        .map_err(|e| e.to_string())?;
    
    // Start contributing storage and earning tokens
    AI_WALLET
        .begin_storage_contribution(50_000_000_000) // 50GB
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(identity.into())
}

#[tauri::command]
async fn send_encrypted_message(
    friend_id: String,
    message: String,
) -> Result<MessageResult, String> {
    // AI automatically handles token spending for message storage
    let result = MESSAGE_HANDLER
        .send_encrypted_message(friend_id, message)
        .await
        .map_err(|e| e.to_string())?;
    
    // Check if user needs more credits
    if AI_WALLET.needs_credits_for_operation().await? {
        return Ok(MessageResult::NeedsCredits {
            estimated_cost: AI_WALLET.estimate_credit_cost().await?,
        });
    }
    
    Ok(MessageResult::Sent)
}

#[tauri::command]
async fn get_network_status() -> Result<NetworkStatus, String> {
    let wallet_status = AI_WALLET.get_status().await.map_err(|e| e.to_string())?;
    
    Ok(NetworkStatus {
        connected: P2P_NODE.is_connected().await,
        credit_status: match wallet_status.credits_sufficient {
            true => "Sufficient".to_string(),
            false => "Low - Add Credits".to_string(),
        },
        peer_count: P2P_NODE.peer_count().await,
        storage_contributed: wallet_status.bytes_contributed,
    })
}

#[tauri::command]
async fn purchase_network_credits(amount_usd: f64) -> Result<PurchaseResult, String> {
    // AI handles entire fiat-to-crypto conversion
    let result = AI_WALLET
        .purchase_credits_with_fiat(amount_usd)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(PurchaseResult {
        credits_received: result.ant_tokens_received,
        transaction_id: result.transaction_id,
        estimated_coverage: result.estimated_months_coverage,
    })
}
```

### Message Encryption and Storage
```rust
impl MessageHandler {
    /// Send encrypted message to friend
    pub async fn send_encrypted_message(
        &self,
        friend_id: String,
        message: String,
    ) -> Result<()> {
        let friend_user_id = UserId::from_string(&friend_id)?;
        
        // Create encrypted message
        let chat_message = ChatMessage {
            id: Uuid::new_v4(),
            sender: self.current_user_id,
            recipient: friend_user_id,
            content: message,
            timestamp: SystemTime::now(),
            message_type: MessageType::Text,
        };
        
        // Generate DHT key for this conversation
        let dht_key = KeyGenerator::friend_data_key(
            &self.current_user_id,
            &friend_user_id,
            "message"
        );
        
        // Create access grants (only sender and recipient)
        let access_grants = vec![
            AccessGrant::new(friend_user_id, AccessLevel::Read),
        ];
        
        // Store in DHT with automatic token payment
        self.storage.store_encrypted(
            dht_key,
            &chat_message.serialize()?,
            access_grants
        ).await?;
        
        // Send notification to friend
        self.send_message_notification(&friend_user_id, &chat_message.id).await?;
        
        Ok(())
    }
}
```

## 📱 Cross-Platform Support

### Current Status
- ✅ **macOS**: Native Apple Silicon and Intel support with DMG installer
- 🔄 **Windows**: Coming in v0.2.0 with MSI installer
- 🔄 **Linux**: Coming in v0.2.0 with AppImage and .deb packages

### Platform-Specific Features
```rust
#[cfg(target_os = "macos")]
mod macos_integration {
    /// macOS-specific features
    pub async fn integrate_with_keychain() -> Result<()> {
        // Store wallet keys in macOS Keychain
        todo!()
    }
    
    pub async fn setup_notification_center() -> Result<()> {
        // Native macOS notifications for messages
        todo!()
    }
}

#[cfg(target_os = "windows")]
mod windows_integration {
    /// Windows-specific features
    pub async fn integrate_with_credential_manager() -> Result<()> {
        // Store wallet keys in Windows Credential Manager
        todo!()
    }
}
```

## 🧪 Testing Strategy

### Multi-Node Testing
```bash
# Run multiple instances for testing
npm run test:multi-node

# Test with different network conditions
npm run test:nat-traversal

# Performance testing
npm run test:load
```

### AI Wallet Testing
```rust
#[cfg(test)]
mod ai_wallet_tests {
    #[tokio::test]
    async fn test_invisible_wallet_creation() {
        let wallet = LocalAIWallet::create_for_new_user().await.unwrap();
        assert!(wallet.has_valid_keypair());
        assert_eq!(wallet.initial_balance(), 0);
    }
    
    #[tokio::test]
    async fn test_automatic_token_earning() {
        let mut wallet = LocalAIWallet::create_for_new_user().await.unwrap();
        
        // Simulate providing storage
        wallet.earn_tokens_for_storage(1_000_000_000).await.unwrap(); // 1GB
        
        assert!(wallet.balance() > 0);
    }
    
    #[tokio::test]
    async fn test_fiat_integration() {
        let mut wallet = LocalAIWallet::create_for_new_user().await.unwrap();
        
        // Test credit purchase flow
        let result = wallet.simulate_credit_purchase(10.0).await.unwrap();
        assert!(result.credits_received > 0);
    }
}
```

## 🚀 Deployment

### Building for Release
```bash
# Build optimized release
cargo tauri build --release

# Create platform-specific installers
cargo tauri build --target universal-apple-darwin  # macOS Universal
cargo tauri build --target x86_64-pc-windows-msvc  # Windows x64
cargo tauri build --target x86_64-unknown-linux-gnu # Linux x64
```

### Distribution
- **macOS**: DMG installer signed with Apple Developer ID
- **Windows**: MSI installer with Authenticode signing
- **Linux**: AppImage, .deb, and .rpm packages
- **Auto-Updates**: Secure update delivery via P2P network

## 🔮 Roadmap

### v0.2.0 - Multi-Platform & AI Enhancement (Q1 2025)
- ✅ Windows and Linux support
- ✅ Enhanced AI wallet integration
- ✅ Improved fiat-to-crypto flow
- ✅ Advanced privacy controls

### v0.3.0 - Advanced Features (Q2 2025)
- 📋 Voice and video calling
- 📋 File sharing and synchronization
- 📋 Group chat functionality
- 📋 Mobile companion app

### v0.4.0 - Enterprise & AI (Q3 2025)
- 📋 Enterprise security features
- 📋 Advanced AI model hosting
- 📋 Plugin system for extensibility
- 📋 Advanced analytics dashboard

## 🤝 Contributing

We welcome contributions to Saorsa! Please see our [contributing guidelines](../../CONTRIBUTING.md) for details.

### Development Guidelines
1. Follow the [CLAUDE.md](../../CLAUDE.md) development standards
2. Ensure all crypto operations remain invisible to users
3. Maintain privacy-first design principles
4. Test on multiple platforms before submitting PRs

### Getting Help
- 🐛 **Bug Reports**: [GitHub Issues](https://github.com/dirvine/p2p/issues)
- 💬 **Discussions**: [GitHub Discussions](https://github.com/dirvine/p2p/discussions)
- 📧 **Security Issues**: security@p2pfoundation.org

---

**🕊️ Saorsa - Freedom through decentralized communication** 

*Building the future of private, peer-to-peer messaging with invisible complexity and maximum user empowerment.*