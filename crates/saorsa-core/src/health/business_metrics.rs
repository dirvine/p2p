// Copyright (c) 2025 Saorsa Labs Limited

// This file is part of the Saorsa P2P network.

// Licensed under the AGPL-3.0 license:
// <https://www.gnu.org/licenses/agpl-3.0.html>

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.


//! Business metrics for P2P network operations

use crate::Result;
use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Business metrics collector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessMetrics {
    pub active_peers: u64,
    pub total_data_stored: u64,
    pub total_data_retrieved: u64,
    pub operations_per_second: f64,
    pub dht_success_rate: f64,
    pub storage_success_rate: f64,
    pub network_growth_rate: f64,
    pub average_response_time_ms: f64,
    pub timestamp: u64,
}

impl BusinessMetrics {
    pub fn new() -> Self {
        Self {
            active_peers: 0,
            total_data_stored: 0,
            total_data_retrieved: 0,
            operations_per_second: 0.0,
            dht_success_rate: 1.0,
            storage_success_rate: 1.0,
            network_growth_rate: 0.0,
            average_response_time_ms: 0.0,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

/// Business metrics collector with thread-safe operations
pub struct BusinessMetricsCollector {
    metrics: Arc<RwLock<BusinessMetrics>>,
    operation_history: Arc<RwLock<Vec<(u64, String, bool, u64)>>>,
}

impl BusinessMetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(BusinessMetrics::new())),
            operation_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn record_peer_connected(&self) -> Result<()> {
        let mut metrics = self.metrics.write().map_err(|e| 
            crate::P2PError::Internal(format!("Failed to acquire metrics lock: {}", e).into()))?;
        metrics.active_peers += 1;
        Ok(())
    }

    pub fn record_peer_disconnected(&self) -> Result<()> {
        let mut metrics = self.metrics.write().map_err(|e| 
            crate::P2PError::Internal(format!("Failed to acquire metrics lock: {}", e).into()))?;
        if metrics.active_peers > 0 {
            metrics.active_peers -= 1;
        }
        Ok(())
    }

    pub fn record_data_stored(&self, bytes: u64, success: bool, latency_ms: u64) -> Result<()> {
        {
            let mut metrics = self.metrics.write().map_err(|e| 
                crate::P2PError::Internal(format!("Failed to acquire metrics lock: {}", e).into()))?;
            if success {
                metrics.total_data_stored += bytes;
            }
        }

        {
            let mut history = self.operation_history.write().map_err(|e| 
                crate::P2PError::Internal(format!("Failed to acquire history lock: {}", e).into()))?;
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            history.push((timestamp, "storage".to_string(), success, latency_ms));
            
            if history.len() > 1000 {
                history.remove(0);
            }
        }
        
        self.update_success_rates()?;
        Ok(())
    }

    pub fn record_data_retrieved(&self, bytes: u64, success: bool, latency_ms: u64) -> Result<()> {
        {
            let mut metrics = self.metrics.write().map_err(|e| 
                crate::P2PError::Internal(format!("Failed to acquire metrics lock: {}", e).into()))?;
            if success {
                metrics.total_data_retrieved += bytes;
            }
        }

        {
            let mut history = self.operation_history.write().map_err(|e| 
                crate::P2PError::Internal(format!("Failed to acquire history lock: {}", e).into()))?;
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            history.push((timestamp, "retrieval".to_string(), success, latency_ms));
            
            if history.len() > 1000 {
                history.remove(0);
            }
        }
        
        self.update_success_rates()?;
        Ok(())
    }

    pub fn get_metrics(&self) -> Result<BusinessMetrics> {
        let metrics = self.metrics.read().map_err(|e| 
            crate::P2PError::Internal(format!("Failed to acquire metrics lock: {}", e).into()))?;
        Ok(metrics.clone())
    }

    fn update_success_rates(&self) -> Result<()> {
        let history = self.operation_history.read().map_err(|e| 
            crate::P2PError::Internal(format!("Failed to acquire history lock: {}", e).into()))?;
        
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let recent_operations: Vec<_> = history.iter()
            .filter(|(timestamp, _, _, _)| now - timestamp < 300)
            .collect();

        if recent_operations.is_empty() {
            return Ok(());
        }

        let storage_ops: Vec<_> = recent_operations.iter()
            .filter(|(_, op_type, _, _)| op_type == "storage")
            .collect();
        
        let dht_ops: Vec<_> = recent_operations.iter()
            .filter(|(_, op_type, _, _)| op_type == "retrieval")
            .collect();

        let mut metrics = self.metrics.write().map_err(|e| 
            crate::P2PError::Internal(format!("Failed to acquire metrics lock: {}", e).into()))?;

        if !storage_ops.is_empty() {
            let successful_storage = storage_ops.iter().filter(|(_, _, success, _)| *success).count();
            metrics.storage_success_rate = successful_storage as f64 / storage_ops.len() as f64;
        }

        if !dht_ops.is_empty() {
            let successful_dht = dht_ops.iter().filter(|(_, _, success, _)| *success).count();
            metrics.dht_success_rate = successful_dht as f64 / dht_ops.len() as f64;
        }

        if !recent_operations.is_empty() {
            let total_latency: u64 = recent_operations.iter().map(|(_, _, _, latency)| *latency).sum();
            metrics.average_response_time_ms = total_latency as f64 / recent_operations.len() as f64;
        }

        let ops_in_last_minute = recent_operations.iter()
            .filter(|(timestamp, _, _, _)| now - timestamp < 60)
            .count();
        metrics.operations_per_second = ops_in_last_minute as f64 / 60.0;

        metrics.timestamp = now;
        Ok(())
    }

    pub fn to_prometheus(&self) -> Result<String> {
        let metrics = self.get_metrics()?;
        let mut output = String::new();

        output.push_str("# HELP p2p_active_peers Number of currently connected peers\n");
        output.push_str("# TYPE p2p_active_peers gauge\n");
        output.push_str(&format!("p2p_active_peers {}\n\n", metrics.active_peers));

        output.push_str("# HELP p2p_total_data_stored_bytes Total bytes stored in the network\n");
        output.push_str("# TYPE p2p_total_data_stored_bytes counter\n");
        output.push_str(&format!("p2p_total_data_stored_bytes {}\n\n", metrics.total_data_stored));

        output.push_str("# HELP p2p_total_data_retrieved_bytes Total bytes retrieved from the network\n");
        output.push_str("# TYPE p2p_total_data_retrieved_bytes counter\n");
        output.push_str(&format!("p2p_total_data_retrieved_bytes {}\n\n", metrics.total_data_retrieved));

        output.push_str("# HELP p2p_dht_success_rate DHT operation success rate (0.0 to 1.0)\n");
        output.push_str("# TYPE p2p_dht_success_rate gauge\n");
        output.push_str(&format!("p2p_dht_success_rate {}\n\n", metrics.dht_success_rate));

        output.push_str("# HELP p2p_storage_success_rate Storage operation success rate (0.0 to 1.0)\n");
        output.push_str("# TYPE p2p_storage_success_rate gauge\n");
        output.push_str(&format!("p2p_storage_success_rate {}\n\n", metrics.storage_success_rate));

        output.push_str("# HELP p2p_operations_per_second Network operations per second\n");
        output.push_str("# TYPE p2p_operations_per_second gauge\n");
        output.push_str(&format!("p2p_operations_per_second {}\n\n", metrics.operations_per_second));

        output.push_str("# HELP p2p_average_response_time_ms Average response time in milliseconds\n");
        output.push_str("# TYPE p2p_average_response_time_ms gauge\n");
        output.push_str(&format!("p2p_average_response_time_ms {}\n\n", metrics.average_response_time_ms));

        output.push_str("# HELP p2p_network_growth_rate Network growth rate in peers per hour\n");
        output.push_str("# TYPE p2p_network_growth_rate gauge\n");
        output.push_str(&format!("p2p_network_growth_rate {}\n\n", metrics.network_growth_rate));

        Ok(output)
    }
}

impl Default for BusinessMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_business_metrics_creation() {
        let collector = BusinessMetricsCollector::new();
        let metrics = collector.get_metrics().unwrap();
        assert_eq!(metrics.active_peers, 0);
        assert_eq!(metrics.total_data_stored, 0);
    }

    #[test]
    fn test_peer_tracking() {
        let collector = BusinessMetricsCollector::new();
        
        collector.record_peer_connected().unwrap();
        collector.record_peer_connected().unwrap();
        
        let metrics = collector.get_metrics().unwrap();
        assert_eq!(metrics.active_peers, 2);
        
        collector.record_peer_disconnected().unwrap();
        let metrics = collector.get_metrics().unwrap();
        assert_eq!(metrics.active_peers, 1);
    }

    #[test]
    fn test_prometheus_export() {
        let collector = BusinessMetricsCollector::new();
        collector.record_peer_connected().unwrap();
        
        let prometheus = collector.to_prometheus().unwrap();
        assert!(prometheus.contains("p2p_active_peers 1"));
        assert!(prometheus.contains("p2p_storage_success_rate 1"));
    }
}
