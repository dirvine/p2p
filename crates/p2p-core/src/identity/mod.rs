//! Identity management module
//! 
//! Provides identity creation, management, and encryption with quantum-resistant capabilities

pub mod manager;
#[cfg(feature = "quantum-resistant")]
pub mod enhanced;

pub use manager::*;
#[cfg(feature = "quantum-resistant")]
pub use enhanced::*;