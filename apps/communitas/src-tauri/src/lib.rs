//! Communitas P2P Collaboration Platform Library
//! 
//! This library provides the core functionality for the Communitas platform,
//! including bootstrap node capabilities, contact management, and P2P networking.

pub mod bootstrap;
pub mod contacts;
pub mod contact_commands;
pub mod identity;
pub mod groups;
pub mod files;

// Re-export main components
pub use bootstrap::{BootstrapNode, BootstrapConfig, NodeStats, run_bootstrap_node};
pub use contacts::{ContactManager, ContactProfile, ContactInvitation, ContactStatus, ContactPreferences};
pub use contact_commands::{ContactManagerState, init_contact_manager};
