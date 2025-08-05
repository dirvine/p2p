//! Network diagnostics and monitoring

mod engine;
mod metrics;

pub use engine::DiagnosticsEngine;
pub use metrics::{NetworkHealth, NetworkMetrics, StorageMetrics, PeerMetrics, NetworkStats};


/// NAT type detected
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, Default)]
pub enum NatType {
    /// No NAT - direct connection
    None,
    /// Full cone NAT
    FullCone,
    /// Restricted cone NAT
    RestrictedCone,
    /// Port restricted cone NAT
    PortRestrictedCone,
    /// Symmetric NAT
    Symmetric,
    /// Unknown/detecting
    #[default]
    Unknown,
}