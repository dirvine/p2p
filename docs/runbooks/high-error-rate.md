# Runbook: High Error Rate

## Alert: P2PHighErrorRate
**Severity**: Critical  
**Response Time**: 5 minutes

## Summary
P2P network operations (DHT or storage) are failing at a rate above 1%.

## Impact
- Degraded user experience
- Potential data loss or unavailability
- Reduced network reliability

## Immediate Actions (0-5 minutes)

1. **Check current error rate**:
   ```bash
   curl -s http://localhost:8080/metrics | grep -E "(dht|storage)_success_rate"
   ```

2. **Identify failing operations**:
   ```bash
   journalctl -u p2p-node --since "10 minutes ago" | grep -i error | tail -20
   ```

3. **Check system health**:
   ```bash
   curl http://localhost:8080/health
   ```

## Investigation

1. **Analyze error patterns**:
   ```bash
   # Check for network timeouts
   journalctl -u p2p-node | grep -i timeout | tail -10
   
   # Check for authentication errors
   journalctl -u p2p-node | grep -i "auth\|permission" | tail -10
   
   # Check for storage errors
   journalctl -u p2p-node | grep -i "storage\|disk\|write" | tail -10
   ```

2. **Network connectivity**:
   ```bash
   # Test peer connectivity
   p2p-cli peer list --unhealthy
   p2p-cli network test-connectivity
   ```

3. **Resource constraints**:
   ```bash
   # Check if we're hitting limits
   curl -s http://localhost:8080/metrics | grep -E "(memory|cpu|disk)"
   ```

## Resolution Steps

### For DHT errors:
```bash
# Check DHT health
p2p-cli dht status
p2p-cli dht repair --dry-run

# If repair needed
p2p-cli dht repair
```

### For storage errors:
```bash
# Check storage health  
p2p-cli storage fsck
p2p-cli storage status

# Clean up if needed
p2p-cli storage cleanup
```

### For network errors:
```bash
# Refresh peer connections
p2p-cli peer refresh
p2p-cli peer bootstrap
```

## Recovery Verification

1. Monitor success rates for 10 minutes
2. Verify error rate drops below 0.5%
3. Check that operations complete successfully

## Prevention

- Implement circuit breakers
- Add retry logic with exponential backoff  
- Monitor peer health proactively
- Set up capacity alerts

## Escalation

Escalate if error rate doesn't improve within 20 minutes.
