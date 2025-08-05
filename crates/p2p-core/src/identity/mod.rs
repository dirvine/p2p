// Copyright 2024 Saorsa Labs Limited
//
// This software is dual-licensed under:
// - GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later)
// - Commercial License
//
// For AGPL-3.0 license, see LICENSE-AGPL-3.0
// For commercial licensing, contact: saorsalabs@gmail.com
//
// Unless required by applicable law or agreed to in writing, software
// distributed under these licenses is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.

//! Identity management module
//! 
//! Provides identity creation, management, and encryption with quantum-resistant capabilities

pub mod manager;
pub mod enhanced;
pub mod node_identity;
pub mod four_words;
pub mod cli;
pub mod encryption;
pub mod secure_node_identity;

#[cfg(test)]
mod node_identity_extensions;
#[cfg(test)]
mod four_words_extensions;
#[cfg(test)]
mod cli_handler;
#[cfg(test)]
mod four_words_error_tests;

pub use manager::*;
pub use enhanced::*;
pub use node_identity::{NodeIdentity, NodeId, ProofOfWork, IdentityData};
pub use four_words::{FourWordAddress, WordEncoder};
pub use secure_node_identity::SecureNodeIdentity;

#[cfg(test)]
pub use node_identity_extensions::*;
// #[cfg(test)]
// pub use four_words_extensions::*; // Commented out - module doesn't exist yet
#[cfg(test)]
pub use cli_handler::{IdentityCommand, IdentityCliHandler, MessageInput, ExportFormat};