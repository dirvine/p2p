# P2P Network Monitoring Setup Guide

## Overview
Complete monitoring solution for P2P network operations using Prometheus, Grafana, and AlertManager.

## Architecture

```
P2P Node → Prometheus → Grafana Dashboards
     ↓         ↓
Health     AlertManager → PagerDuty/Slack
Metrics         ↓
            Runbooks
```

## Quick Start

### 1. Start P2P Node with Health Endpoints
```bash
# The health server starts automatically on port 8080
cargo run --bin p2p-node

# Verify health endpoint
curl http://localhost:8080/health
curl http://localhost:8080/metrics
```

### 2. Configure Prometheus
```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'p2p-nodes'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 10s
```

### 3. Start Prometheus
```bash
docker run -p 9090:9090 \
  -v $(pwd)/monitoring/prometheus:/etc/prometheus \
  prom/prometheus
```

### 4. Configure Grafana
```bash
# Start Grafana
docker run -p 3000:3000 grafana/grafana

# Import dashboard: monitoring/grafana/dashboards/p2p-overview.json
```

### 5. Setup AlertManager
```bash
# Configure alerts
docker run -p 9093:9093 \
  -v $(pwd)/monitoring/alertmanager:/etc/alertmanager \
  prom/alertmanager
```

## Key Metrics

### System Health
- `p2p_healthy_components` - Number of healthy components
- `p2p_unhealthy_components` - Number of unhealthy components  
- `p2p_health_check_latency_ms` - Health check response time

### Business Metrics
- `p2p_active_peers` - Currently connected peers
- `p2p_operations_per_second` - Network operations rate
- `p2p_dht_success_rate` - DHT operation success rate (0.0-1.0)
- `p2p_storage_success_rate` - Storage operation success rate (0.0-1.0)
- `p2p_average_response_time_ms` - Average operation latency

### System Resources
- `p2p_system_memory_total_bytes` - Total system memory
- `p2p_system_memory_available_bytes` - Available system memory
- `p2p_runtime_threads` - Number of runtime threads

## Alert Thresholds

| Alert | Condition | Threshold | Action |
|-------|-----------|-----------|--------|
| Network Down | All components unhealthy | 1 minute | Page immediately |
| High Error Rate | Success rate < 99% | 5 minutes | Investigate |
| High Latency | Response time > 500ms | 5 minutes | Check performance |
| Low Peers | Peer count < 10 | 10 minutes | Check connectivity |
| Memory Usage | Memory > 80% | 5 minutes | Check for leaks |

## Dashboard Panels

1. **Network Status** - Health overview with component status
2. **Active Peers** - Real-time peer count with trends  
3. **System Resources** - Memory, CPU, and thread usage
4. **Operations Performance** - Throughput and latency metrics
5. **Success Rates** - DHT and storage operation success rates
6. **Data Transfer** - Storage and retrieval rate trends

## Runbook Integration

Each alert links to specific runbooks:
- Network failures → [Network Down Runbook](./runbooks/network-down.md)
- Performance issues → [High Latency Runbook](./runbooks/high-latency.md)  
- Error conditions → [High Error Rate Runbook](./runbooks/high-error-rate.md)

## Team Training

### On-Call Engineers
1. **Dashboard Navigation** - How to read the Grafana dashboards
2. **Alert Response** - Standard procedures for each alert type
3. **Runbook Execution** - Step-by-step incident response
4. **Escalation** - When and how to escalate issues

### Platform Team
1. **Metrics Design** - How to add new metrics
2. **Alert Tuning** - Adjusting thresholds and conditions
3. **Dashboard Updates** - Modifying Grafana panels
4. **Runbook Maintenance** - Keeping procedures current

## Production Checklist

- [ ] Health endpoints accessible
- [ ] Prometheus scraping metrics  
- [ ] Grafana dashboards imported
- [ ] AlertManager configured
- [ ] PagerDuty integration tested
- [ ] Runbooks reviewed and tested
- [ ] Team trained on procedures
- [ ] Escalation paths verified
- [ ] Monitoring coverage validated

## Maintenance

### Weekly
- Review alert noise and tune thresholds
- Check dashboard accuracy
- Verify runbook currency

### Monthly  
- Conduct tabletop incident exercises
- Review and update escalation procedures
- Analyze monitoring coverage gaps

### Quarterly
- Full disaster recovery drill
- Monitor tool version updates
- Review and improve automation
