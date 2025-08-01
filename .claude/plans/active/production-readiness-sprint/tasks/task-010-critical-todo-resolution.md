# Task 010: Critical TODO/FIXME Resolution

## Overview
Resolve the most critical items from the 77 TODO/FIXME comments in the codebase, focusing on security, data integrity, and production readiness issues.

## Acceptance Criteria
- [ ] All security-related TODOs resolved
- [ ] Data integrity TODOs addressed
- [ ] Performance-critical TODOs completed
- [ ] Remaining TODOs documented with timeline
- [ ] No new technical debt introduced

## Technical Details

### 1. TODO Scanning and Prioritization

First, scan and categorize all TODOs:
```bash
# Find all TODOs with context
rg "TODO|FIXME" --type rust -B 2 -A 2 > todo_list.txt
```

Priority Categories:
1. **CRITICAL**: Security, data loss, panic potential
2. **HIGH**: Performance, missing features
3. **MEDIUM**: Code cleanup, optimizations
4. **LOW**: Nice-to-have improvements

### 2. Security-Critical TODOs

#### Example: Rate Limiting (from production.rs)
```rust
// TODO: Implement rate limiting
// CRITICAL: Without this, nodes are vulnerable to DoS

// Resolution:
use governor::{Quota, RateLimiter};

pub struct RateLimitedEndpoint {
    limiter: RateLimiter<String, DefaultKeyedStateStore<String>>,
}

impl RateLimitedEndpoint {
    pub fn new(requests_per_second: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap());
        Self {
            limiter: RateLimiter::keyed(quota),
        }
    }
    
    pub async fn check_rate_limit(&self, peer_id: &str) -> Result<()> {
        match self.limiter.check_key(&peer_id.to_string()) {
            Ok(_) => Ok(()),
            Err(_) => Err(NetworkError::RateLimitExceeded),
        }
    }
}
```

#### Example: Secure Key Deletion
```rust
// TODO: Implement secure key wiping
// CRITICAL: Keys may remain in memory

// Resolution:
use zeroize::Zeroize;

impl Drop for SecretKey {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}
```

### 3. Data Integrity TODOs

#### Example: Atomic File Operations
```rust
// TODO: Make file writes atomic
// HIGH: Corruption possible on crash

// Resolution:
pub async fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let temp_path = path.with_extension("tmp");
    
    // Write to temporary file
    tokio::fs::write(&temp_path, data).await?;
    
    // Sync to disk
    let file = tokio::fs::File::open(&temp_path).await?;
    file.sync_all().await?;
    
    // Atomic rename
    tokio::fs::rename(&temp_path, path).await?;
    
    Ok(())
}
```

### 4. Performance TODOs

#### Example: Connection Pool Sizing
```rust
// TODO: Dynamic connection pool sizing
// MEDIUM: Fixed pool may be suboptimal

// Resolution:
pub struct DynamicConnectionPool {
    min_size: usize,
    max_size: usize,
    current_load: Arc<AtomicUsize>,
}

impl DynamicConnectionPool {
    pub async fn adjust_pool_size(&self) {
        let load = self.current_load.load(Ordering::Relaxed);
        let current_size = self.get_current_size();
        
        let target_size = if load > 80 {
            (current_size * 1.5).min(self.max_size)
        } else if load < 20 {
            (current_size * 0.8).max(self.min_size)
        } else {
            current_size
        };
        
        self.resize_pool(target_size).await;
    }
}
```

### 5. Missing Feature TODOs

#### Example: Metric Collection
```rust
// TODO: Add connection metrics
// HIGH: No visibility into connection health

// Resolution:
#[derive(Debug)]
pub struct ConnectionMetrics {
    pub bytes_sent: Counter,
    pub bytes_received: Counter,
    pub messages_sent: Counter,
    pub messages_received: Counter,
    pub errors: Counter,
    pub latency: Histogram,
}

impl Connection {
    pub async fn send_with_metrics(&self, msg: Message) -> Result<()> {
        let start = Instant::now();
        
        match self.send_internal(msg).await {
            Ok(()) => {
                self.metrics.messages_sent.inc();
                self.metrics.bytes_sent.inc_by(msg.len() as u64);
                self.metrics.latency.observe(start.elapsed().as_secs_f64());
                Ok(())
            }
            Err(e) => {
                self.metrics.errors.inc();
                Err(e)
            }
        }
    }
}
```

### 6. Documentation for Deferred TODOs

Create `docs/TECHNICAL_DEBT.md`:
```markdown
# Technical Debt Register

## Deferred TODOs

### Low Priority Items

1. **Optimize message serialization** (network/message.rs:142)
   - Current: Using bincode
   - Proposed: Custom zero-copy serialization
   - Impact: 10-15% performance improvement
   - Timeline: Post-v1.0

2. **Implement connection multiplexing** (transport/quic.rs:89)
   - Current: One stream per message
   - Proposed: Stream pooling
   - Impact: Reduced latency
   - Timeline: v1.1

## Tracking
- Total TODOs at sprint start: 77
- Resolved in this sprint: 45
- Deferred with justification: 32
```

## Testing Requirements
- Unit tests for each resolved TODO
- Integration tests for critical fixes
- Performance benchmarks where applicable
- Security audit for crypto changes

## Dependencies
- Previous: All core module tasks
- May impact: All modules

## Time Estimate
- Analysis and prioritization: 2 hours
- Critical TODOs: 8 hours
- High priority TODOs: 6 hours
- Testing: 4 hours
- Documentation: 2 hours
- Total: 22 hours

## Definition of Done
- [ ] All critical TODOs resolved
- [ ] High priority TODOs completed
- [ ] Remaining TODOs documented
- [ ] Tests for all changes
- [ ] Technical debt register updated