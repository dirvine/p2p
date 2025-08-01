# Task 5: Fix Configuration Hardcoding

## Overview
Replace hardcoded addresses and values with proper configuration management.

## Context
- **Phase**: Security Hardening (Week 2-3)
- **Priority**: HIGH
- **Impact**: Won't work in production environment
- **Issues**: Hardcoded localhost/127.0.0.1 addresses throughout code

## Requirements
1. Replace hardcoded addresses with config
2. Add environment variable support
3. Implement config validation
4. Create config documentation

## Hardcoded Values to Fix
```rust
// Found in multiple files:
"127.0.0.1:9000"
"localhost:8080"
"127.0.0.1:0"
// In bootstrap/cache.rs, bootstrap/contact.rs, etc.
```

## Technical Specification
- Create `Config` struct with serde
- Support layered configuration (env > file > defaults)
- Add validation for network addresses
- Support both IPv4 and IPv6
- Create production and development profiles

## Configuration Structure
```toml
[network]
bootstrap_nodes = ["addr1", "addr2"]
listen_address = "0.0.0.0:9000"
public_address = ""  # Auto-detect if empty

[security]
rate_limit = 1000
connection_limit = 10000

[storage]
path = "./data"
max_size = "10GB"
```

## Acceptance Criteria
- [ ] No hardcoded addresses in src/
- [ ] Config struct with proper defaults
- [ ] Environment variable override support
- [ ] Config validation on startup
- [ ] Example config files provided
- [ ] Documentation complete

## Dependencies
- Task 1: Error Handling (for config errors)

## Testing
- Config loading tests
- Environment override tests
- Invalid config handling
- Default behavior tests