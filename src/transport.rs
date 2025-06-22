//! Transport module placeholder
//!
//! This module will contain transport layer functionality.

/// Placeholder transport types
pub mod transport_types {
    /// Transport protocol types
    #[derive(Debug, Clone, PartialEq)]
    pub enum TransportType {
        QUIC,
        TCP,
    }
}