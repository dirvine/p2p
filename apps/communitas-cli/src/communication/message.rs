// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Message data structures and serialization

use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::identity::FourWordAddress;

/// Message content with metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub id: Uuid,
    pub from: FourWordAddress,
    pub to: FourWordAddress,
    pub content: String,
    pub timestamp: u64,
    pub message_type: MessageType,
}

/// Types of messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    Text,
    File,
    System,
    DeliveryConfirmation { original_id: Uuid },
}

/// Encrypted message envelope for transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub id: Uuid,
    pub to: FourWordAddress,
    pub encrypted_payload: Vec<u8>,
    pub signature: Vec<u8>,
    pub timestamp: u64,
}

/// Message delivery status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeliveryStatus {
    Pending,
    Sent,
    Delivered,
    Failed { reason: String },
}

/// Stored message with delivery tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    pub message: Message,
    pub status: DeliveryStatus,
    pub attempts: u32,
    pub last_attempt: Option<u64>,
}

impl Message {
    /// Create a new text message
    pub fn new_text(
        from: FourWordAddress, 
        to: FourWordAddress, 
        content: String
    ) -> Self {
        Message {
            id: Uuid::new_v4(),
            from,
            to,
            content,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            message_type: MessageType::Text,
        }
    }
    
    /// Create a delivery confirmation message
    pub fn new_delivery_confirmation(
        from: FourWordAddress,
        to: FourWordAddress,
        original_id: Uuid,
    ) -> Self {
        Message {
            id: Uuid::new_v4(),
            from,
            to,
            content: format!("Message {} delivered", original_id),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            message_type: MessageType::DeliveryConfirmation { original_id },
        }
    }
    
    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(Into::into)
    }
    
    /// Deserialize from JSON
    pub fn from_json(data: &str) -> Result<Self> {
        serde_json::from_str(data).map_err(Into::into)
    }
}

impl MessageEnvelope {
    /// Create new envelope with encrypted payload
    pub fn new(
        to: FourWordAddress,
        encrypted_payload: Vec<u8>,
        signature: Vec<u8>,
    ) -> Self {
        MessageEnvelope {
            id: Uuid::new_v4(),
            to,
            encrypted_payload,
            signature,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

impl StoredMessage {
    /// Create new stored message
    pub fn new(message: Message) -> Self {
        StoredMessage {
            message,
            status: DeliveryStatus::Pending,
            attempts: 0,
            last_attempt: None,
        }
    }
    
    /// Mark message as sent
    pub fn mark_sent(&mut self) {
        self.status = DeliveryStatus::Sent;
        self.attempts += 1;
        self.last_attempt = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
    }
    
    /// Mark message as delivered
    pub fn mark_delivered(&mut self) {
        self.status = DeliveryStatus::Delivered;
    }
    
    /// Mark message as failed
    pub fn mark_failed(&mut self, reason: String) {
        self.status = DeliveryStatus::Failed { reason };
        self.attempts += 1;
        self.last_attempt = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        let message = Message::new_text(from.clone(), to.clone(), "Hello!".to_string());
        
        assert_eq!(message.from, from);
        assert_eq!(message.to, to);
        assert_eq!(message.content, "Hello!");
        assert!(matches!(message.message_type, MessageType::Text));
    }
    
    #[test]
    fn test_delivery_confirmation_message() {
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        let original_id = Uuid::new_v4();
        
        let message = Message::new_delivery_confirmation(from.clone(), to.clone(), original_id);
        
        assert_eq!(message.from, from);
        assert_eq!(message.to, to);
        assert!(message.content.contains(&original_id.to_string()));
        assert!(matches!(message.message_type, MessageType::DeliveryConfirmation { .. }));
    }
    
    #[test]
    fn test_message_serialization() {
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        let message = Message::new_text(from, to, "Test message".to_string());
        
        let json = message.to_json().unwrap();
        let deserialized = Message::from_json(&json).unwrap();
        
        assert_eq!(message, deserialized);
    }
    
    #[test]
    fn test_message_envelope_creation() {
        let to = FourWordAddress::generate().unwrap();
        let payload = b"encrypted data".to_vec();
        let signature = b"signature".to_vec();
        
        let envelope = MessageEnvelope::new(to.clone(), payload.clone(), signature.clone());
        
        assert_eq!(envelope.to, to);
        assert_eq!(envelope.encrypted_payload, payload);
        assert_eq!(envelope.signature, signature);
    }
    
    #[test]
    fn test_stored_message_lifecycle() {
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        let message = Message::new_text(from, to, "Test".to_string());
        let mut stored = StoredMessage::new(message);
        
        // Initial state
        assert!(matches!(stored.status, DeliveryStatus::Pending));
        assert_eq!(stored.attempts, 0);
        
        // Mark as sent
        stored.mark_sent();
        assert!(matches!(stored.status, DeliveryStatus::Sent));
        assert_eq!(stored.attempts, 1);
        assert!(stored.last_attempt.is_some());
        
        // Mark as delivered
        stored.mark_delivered();
        assert!(matches!(stored.status, DeliveryStatus::Delivered));
        
        // Test failure case
        let mut stored2 = StoredMessage::new(stored.message);
        stored2.mark_failed("Network error".to_string());
        assert!(matches!(stored2.status, DeliveryStatus::Failed { .. }));
        assert_eq!(stored2.attempts, 1);
    }
    
    #[test]
    fn test_message_types() {
        // Test different message types can be created and identified
        assert!(matches!(MessageType::Text, MessageType::Text));
        assert!(matches!(MessageType::File, MessageType::File));
        assert!(matches!(MessageType::System, MessageType::System));
        
        let delivery_conf = MessageType::DeliveryConfirmation { 
            original_id: Uuid::new_v4() 
        };
        assert!(matches!(delivery_conf, MessageType::DeliveryConfirmation { .. }));
    }
}