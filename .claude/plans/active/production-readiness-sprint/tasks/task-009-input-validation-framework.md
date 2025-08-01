# Task 009: Comprehensive Input Validation

## Overview
Implement input validation across all public APIs and network endpoints to prevent malformed data from causing errors or security issues. This includes validating network messages, API parameters, and configuration values.

## Acceptance Criteria
- [ ] All public APIs have input validation
- [ ] Network message validation implemented
- [ ] Configuration validation on load
- [ ] Clear validation error messages
- [ ] No performance regression

## Technical Details

### 1. Validation Framework Setup

#### Core Validation Types
Location: `crates/p2p-core/src/validation/mod.rs`

```rust
use validator::{Validate, ValidationError};
use regex::Regex;

lazy_static! {
    static ref THREE_WORDS_REGEX: Regex = 
        Regex::new(r"^[a-z]+-[a-z]+-[a-z]+$").unwrap();
    static ref PEER_ID_REGEX: Regex = 
        Regex::new(r"^[0-9a-f]{64}$").unwrap();
}

#[derive(Debug, Validate)]
pub struct ValidatedPeerAddress {
    #[validate(length(min = 1, max = 255))]
    #[validate(custom = "validate_ip_or_domain")]
    pub host: String,
    
    #[validate(range(min = 1, max = 65535))]
    pub port: u16,
}

#[derive(Debug, Validate)]
pub struct ValidatedThreeWords {
    #[validate(regex = "THREE_WORDS_REGEX")]
    pub words: String,
}

#[derive(Debug, Validate)]
pub struct ValidatedDhtKey {
    #[validate(length(equal = 32))]
    pub bytes: Vec<u8>,
}

#[derive(Debug, Validate)]
pub struct ValidatedMessage {
    #[validate(length(min = 1, max = 1_048_576))] // 1MB max
    pub payload: Vec<u8>,
    
    #[validate(regex = "PEER_ID_REGEX")]
    pub sender_id: String,
    
    #[validate(range(min = 0, max = 255))]
    pub message_type: u8,
}
```

### 2. Network Message Validation

```rust
impl NetworkMessage {
    pub fn validate_incoming(&self) -> Result<(), ValidationError> {
        // Size limits
        if self.payload.len() > MAX_MESSAGE_SIZE {
            return Err(ValidationError::new("message_too_large"));
        }
        
        // Message type validation
        if !MessageType::is_valid(self.msg_type) {
            return Err(ValidationError::new("invalid_message_type"));
        }
        
        // Sender validation
        if self.sender.as_bytes().len() != 32 {
            return Err(ValidationError::new("invalid_sender_id"));
        }
        
        // Protocol version
        if self.version > CURRENT_PROTOCOL_VERSION {
            return Err(ValidationError::new("unsupported_protocol_version"));
        }
        
        Ok(())
    }
}
```

### 3. API Parameter Validation

```rust
// For Tauri commands
#[tauri::command]
pub async fn connect_to_peer(
    address: String,
    port: u16,
    network: State<'_, Network>,
) -> Result<String, String> {
    // Validate inputs
    let validated_address = ValidatedPeerAddress {
        host: address,
        port,
    };
    
    validated_address.validate()
        .map_err(|e| format!("Invalid address: {}", e))?;
    
    // Proceed with connection
    network
        .connect(&validated_address.host, validated_address.port)
        .await
        .map_err(|e| e.to_string())
}
```

### 4. Configuration Validation

```rust
#[derive(Debug, Deserialize, Validate)]
pub struct NetworkConfig {
    #[validate(length(min = 1, max = 100))]
    pub node_name: String,
    
    #[validate(range(min = 1024, max = 65535))]
    pub listen_port: u16,
    
    #[validate(range(min = 1, max = 1000))]
    pub max_connections: usize,
    
    #[validate(range(min = 1, max = 300))]
    pub connection_timeout_secs: u64,
    
    #[validate(custom = "validate_log_level")]
    pub log_level: String,
}

impl NetworkConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read config file")?;
        
        let config: Self = toml::from_str(&content)
            .context("Failed to parse config file")?;
        
        config.validate()
            .context("Invalid configuration")?;
        
        Ok(config)
    }
}
```

### 5. DHT Operation Validation

```rust
impl DhtClient {
    pub async fn store(&self, key: &[u8], value: &[u8]) -> Result<()> {
        // Validate key
        if key.len() != 32 {
            return Err(DhtError::InvalidKeyLength { 
                expected: 32, 
                actual: key.len() 
            });
        }
        
        // Validate value
        if value.is_empty() {
            return Err(DhtError::EmptyValue);
        }
        
        if value.len() > MAX_DHT_VALUE_SIZE {
            return Err(DhtError::ValueTooLarge { 
                size: value.len(), 
                max: MAX_DHT_VALUE_SIZE 
            });
        }
        
        // Proceed with storage
        self.store_internal(key, value).await
    }
}
```

### 6. Custom Validators

```rust
fn validate_ip_or_domain(host: &str) -> Result<(), ValidationError> {
    // Try parsing as IP first
    if host.parse::<IpAddr>().is_ok() {
        return Ok(());
    }
    
    // Validate as domain name
    if host.len() > 253 {
        return Err(ValidationError::new("domain_too_long"));
    }
    
    let labels: Vec<&str> = host.split('.').collect();
    if labels.is_empty() || labels.len() > 127 {
        return Err(ValidationError::new("invalid_domain_structure"));
    }
    
    for label in labels {
        if label.is_empty() || label.len() > 63 {
            return Err(ValidationError::new("invalid_domain_label"));
        }
        if !label.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(ValidationError::new("invalid_domain_characters"));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(ValidationError::new("invalid_domain_label_format"));
        }
    }
    
    Ok(())
}
```

## Testing Requirements
- Unit tests for each validator
- Fuzzing tests with random inputs
- Edge case testing (empty, max size, special chars)
- Performance benchmarks
- Integration tests with real data

## Dependencies
- Previous: Task 001 (Error Framework)
- External: validator crate

## Time Estimate
- Implementation: 8 hours
- Testing: 4 hours
- Integration: 2 hours
- Total: 14 hours

## Definition of Done
- [ ] All public APIs validated
- [ ] Network messages validated
- [ ] Configuration validation complete
- [ ] Tests comprehensive
- [ ] No performance impact