//! Communitas P2P Collaboration Platform Library
//!
//! This library provides the core functionality for the Communitas platform,
//! including bootstrap node capabilities, contact management, and P2P networking.

pub mod bootstrap;
pub mod contact_commands;
pub mod contacts;
pub mod files;
pub mod groups;
pub mod identity;
pub mod stores;

// Re-export main components
pub use bootstrap::{run_bootstrap_node, BootstrapConfig, BootstrapNode, NodeStats};
pub use contact_commands::{init_contact_manager, ContactManagerState};
pub use contacts::{
    ContactInvitation, ContactManager, ContactPreferences, ContactProfile, ContactStatus,
};
// Store API re-exports
pub use stores::{
    init_local_stores, get_metadata, create_organization, create_group_local as create_group, create_project,
    add_contact_local, list_markdown, read_markdown_file, write_markdown_file,
    create_markdown, Metadata, ScopePath, MarkdownFileInfo,
};
