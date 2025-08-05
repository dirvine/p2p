//! Chat service implementation

mod service;
mod message;
mod group;

pub use service::ChatService;
pub use message::{Message, MessageContent, MessageId};
pub use group::{Group, GroupId};


/// Chat-related errors
#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    /// Group not found
    #[error("Group not found: {0}")]
    GroupNotFound(String),
    
    /// Message delivery failed
    #[error("Message delivery failed: {0}")]
    DeliveryFailed(String),
    
    /// Invalid group size
    #[error("Group size exceeds maximum of 20 participants")]
    GroupSizeLimitExceeded,
    
    /// Storage error
    #[error("Storage error: {0}")]
    Storage(#[from] sled::Error),
}