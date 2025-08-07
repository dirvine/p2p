# Runbook: P2P Network Down

## Alert: P2PNetworkDown
**Severity**: Critical  
**Response Time**: Immediate (< 5 minutes)

## Summary
All P2P network components are reporting as unhealthy, indicating complete network failure.

## Impact
- Users cannot connect to the P2P network
- No data can be stored or retrieved
- All network operations are failing
- Business operations are completely disrupted

## Immediate Actions (0-5 minutes)

1. **Acknowledge the alert** in PagerDuty
2. **Check system status**:
   ```bash
   curl -f http://localhost:8080/health || echo "Health endpoint down"
   curl -f http://localhost:8080/metrics || echo "Metrics endpoint down"
   ```
3. **Verify process is running**:
   ```bash
   ps aux | grep p2p-node
   systemctl status p2p-node
   ```
4. **Check recent logs**:
   ```bash
   journalctl -u p2p-node --since "5 minutes ago" -n 50
   ```

## Investigation (5-15 minutes)

1. **System resources**:
   ```bash
   df -h  # Disk space
   free -h  # Memory
   top -bn1 | head -20  # CPU usage
   ```

2. **Network connectivity**:
   ```bash
   netstat -tulpn | grep :8080  # Check if port is bound
   ping 8.8.8.8  # Basic connectivity
   ```

3. **Check configuration**:
   ```bash
   p2p-node --validate-config
   ```

4. **Database/Storage check**:
   ```bash
   ls -la /var/lib/p2p/  # Check data directory
   du -sh /var/lib/p2p/*  # Storage usage
   ```

## Resolution Steps

### If process is not running:
```bash
systemctl start p2p-node
systemctl status p2p-node
journalctl -u p2p-node -f  # Monitor startup
```

### If process is running but unhealthy:
```bash
systemctl restart p2p-node
# Wait 2 minutes for startup
curl http://localhost:8080/health
```

### If configuration issues:
```bash
# Backup current config
cp /etc/p2p/config.toml /tmp/config.toml.backup

# Restore known good configuration
cp /etc/p2p/config.toml.backup /etc/p2p/config.toml

# Restart service
systemctl restart p2p-node
```

### If disk space issues:
```bash
# Clean old logs
journalctl --vacuum-time=7d

# Clean old data (CAUTION: May cause data loss)
find /var/lib/p2p/cache -mtime +7 -delete

# Restart service
systemctl restart p2p-node
```

## Recovery Verification

1. **Check health endpoint**:
   ```bash
   curl http://localhost:8080/health | jq .
   # Should show all components as healthy
   ```

2. **Verify metrics**:
   ```bash
   curl http://localhost:8080/metrics | grep p2p_healthy_components
   # Should show healthy_components > 0
   ```

3. **Test basic functionality**:
   ```bash
   p2p-cli peer list  # Should show connected peers
   p2p-cli storage test  # Should succeed
   ```

## Post-Incident Actions

1. **Document the incident** in the incident tracker
2. **Update monitoring** if gaps were identified
3. **Schedule post-mortem** if downtime > 15 minutes
4. **Review and update** this runbook based on learnings

## Prevention

1. **Monitoring**: Ensure all health checks are properly configured
2. **Alerting**: Set up predictive alerts for resource exhaustion
3. **Automation**: Consider auto-restart for certain failure modes
4. **Capacity**: Monitor growth trends and plan capacity
5. **Testing**: Regular disaster recovery drills

## Escalation

Escalate to Level 2 if:
- Resolution steps don't work after 30 minutes
- Multiple nodes are affected
- Data corruption is suspected
- Root cause is unclear

**Level 2 Contact**: platform-team-lead@example.com
