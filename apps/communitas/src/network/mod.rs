//! Network integration with P2P Foundation

mod integration;

pub use integration::NetworkIntegration;

/// Network-related errors
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    /// Bootstrap connection failed
    #[error("Failed to connect to bootstrap node: {0}")]
    BootstrapFailed(String),

    /// DHT operation failed
    #[error("DHT operation failed: {0}")]
    DhtError(String),

    /// Transport error
    #[error("Transport error: {0}")]
    TransportError(String),
}
