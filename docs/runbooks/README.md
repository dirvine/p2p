# P2P Network Operational Runbooks

This directory contains operational runbooks for responding to P2P network alerts and incidents.

## Quick Reference

| Alert | Severity | Runbook | Response Time |
|-------|----------|---------|---------------|
| P2PNetworkDown | Critical | [Network Down](./network-down.md) | Immediate |
| P2PHighErrorRate | Critical | [High Error Rate](./high-error-rate.md) | 5 minutes |
| P2PHighLatency | Warning | [High Latency](./high-latency.md) | 15 minutes |
| P2PLowPeerCount | Warning | [Low Peer Count](./low-peer-count.md) | 30 minutes |
| P2PHighMemoryUsage | Warning | [High Memory Usage](./high-memory-usage.md) | 15 minutes |

## Runbook Structure

Each runbook follows this standard structure:
1. **Severity & Impact**: Alert severity and business impact
2. **Immediate Actions**: Steps to take within first 5 minutes
3. **Investigation**: How to diagnose the root cause
4. **Resolution**: Step-by-step fix instructions
5. **Prevention**: How to prevent recurrence
6. **Escalation**: When and how to escalate

## Monitoring Dashboards

- [P2P Overview Dashboard](http://grafana.example.com/d/p2p-overview)
- [System Health Dashboard](http://grafana.example.com/d/p2p-health)  
- [Performance Dashboard](http://grafana.example.com/d/p2p-performance)

## On-Call Procedures

### Escalation Path
1. **Level 1**: On-call engineer (PagerDuty)
2. **Level 2**: Platform team lead
3. **Level 3**: Engineering manager
4. **Level 4**: VP Engineering / CTO

### Contact Information
- **PagerDuty**: +1-800-XXX-XXXX
- **Slack**: #p2p-alerts
- **Email**: platform-oncall@example.com
