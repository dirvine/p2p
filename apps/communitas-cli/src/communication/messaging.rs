// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Main messaging logic with encryption and persistence

use anyhow::{Result, Context};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;
use crate::identity::{FourWordAddress, EnhancedIdentityManager};
use super::message::{Message, MessageEnvelope, StoredMessage, MessageType};
use super::delivery::{MessageDelivery, DeliveryResult};

/// Secure messaging system
#[derive(Debug)]
pub struct SecureMessaging {
    /// Message delivery manager
    delivery: MessageDelivery,
    /// Message storage
    storage: MessageStorage,
    /// Identity manager for encryption/decryption
    identity_manager: Option<EnhancedIdentityManager>,
}

/// Message storage system
#[derive(Debug)]
pub struct MessageStorage {
    /// Storage path
    storage_path: PathBuf,
    /// In-memory message history (will be persisted)
    message_history: HashMap<FourWordAddress, Vec<StoredMessage>>,
    /// Maximum messages to keep per conversation
    max_messages_per_conversation: usize,
}

/// Conversation summary
#[derive(Debug, Clone)]
pub struct ConversationSummary {
    pub peer: FourWordAddress,
    pub message_count: usize,
    pub last_message_time: Option<u64>,
    pub unread_count: usize,
}

impl SecureMessaging {
    /// Create new secure messaging system
    pub fn new(storage_path: PathBuf) -> Self {
        SecureMessaging {
            delivery: MessageDelivery::new(),
            storage: MessageStorage::new(storage_path),
            identity_manager: None,
        }
    }
    
    /// Initialize with identity manager
    pub fn with_identity_manager(mut self, identity_manager: EnhancedIdentityManager) -> Self {
        self.identity_manager = Some(identity_manager);
        self
    }
    
    /// Send a text message
    pub async fn send_message(
        &mut self, 
        to: FourWordAddress, 
        content: String
    ) -> Result<Uuid> {
        // Get sender address from identity manager
        let from = self.get_own_address()
            .context("No identity available for sending messages")?;
        
        let message = Message::new_text(from.clone(), to.clone(), content);
        let message_id = message.id;
        
        // Store message locally
        self.storage.store_message(&from, StoredMessage::new(message.clone()));
        
        // Queue for delivery
        self.delivery.queue_message(message)?;
        
        Ok(message_id)
    }
    
    /// Receive and decrypt a message envelope
    pub async fn receive_message_envelope(
        &mut self, 
        envelope: MessageEnvelope
    ) -> Result<Option<Message>> {
        // Decrypt the envelope (placeholder implementation)
        let message = self.decrypt_envelope(envelope)
            .context("Failed to decrypt message envelope")?;
        
        if let Some(msg) = message {
            // Store received message
            let stored = StoredMessage::new(msg.clone());
            self.storage.store_message(&msg.from, stored);
            
            // Send delivery confirmation
            self.send_delivery_confirmation(&msg).await?;
            
            Ok(Some(msg))
        } else {
            Ok(None)
        }
    }
    
    /// Process delivery confirmation
    pub fn process_delivery_confirmation(&mut self, confirmation: &Message) -> Result<()> {
        if let MessageType::DeliveryConfirmation { original_id } = &confirmation.message_type {
            self.delivery.process_delivery_confirmation(*original_id)?;
        }
        Ok(())
    }
    
    /// Get conversation history with a peer
    pub fn get_conversation_history(&self, peer: &FourWordAddress) -> Vec<StoredMessage> {
        self.storage.get_conversation_history(peer)
    }
    
    /// Get all conversations summary
    pub fn get_conversations_summary(&self) -> Vec<ConversationSummary> {
        self.storage.get_conversations_summary()
    }
    
    /// Encrypt message into envelope (placeholder implementation)
    pub fn encrypt_message(&self, message: &Message) -> Result<MessageEnvelope> {
        // TODO: Replace with actual encryption using saorsa-core
        let payload = serde_json::to_vec(message)
            .context("Failed to serialize message")?;
        
        // Placeholder "encryption" - just the serialized data
        let encrypted_payload = payload;
        
        // Placeholder signature
        let signature = b"placeholder_signature".to_vec();
        
        Ok(MessageEnvelope::new(
            message.to.clone(),
            encrypted_payload,
            signature
        ))
    }
    
    /// Decrypt envelope into message (placeholder implementation)
    fn decrypt_envelope(&self, envelope: MessageEnvelope) -> Result<Option<Message>> {
        // TODO: Replace with actual decryption using saorsa-core
        
        // For now, just deserialize directly (placeholder)
        let message: Message = serde_json::from_slice(&envelope.encrypted_payload)
            .context("Failed to decrypt/deserialize message")?;
        
        Ok(Some(message))
    }
    
    /// Send delivery confirmation
    async fn send_delivery_confirmation(&mut self, original_message: &Message) -> Result<()> {
        let from = self.get_own_address()
            .context("No identity available for sending confirmation")?;
        
        let confirmation = Message::new_delivery_confirmation(
            from,
            original_message.from.clone(),
            original_message.id,
        );
        
        self.delivery.queue_message(confirmation)?;
        Ok(())
    }
    
    /// Get own address from identity manager
    fn get_own_address(&self) -> Option<FourWordAddress> {
        self.identity_manager.as_ref()
            .and_then(|im| im.get_address())
            .cloned()
    }
    
    /// Process delivery queue
    pub async fn process_delivery_queue(&mut self) -> Result<Vec<DeliveryResult>> {
        let mut results = Vec::new();
        
        while let Some(result) = self.delivery.deliver_next_message().await? {
            let is_success = matches!(result, DeliveryResult::Success);
            results.push(result);
            
            // Break if we hit an error to avoid infinite loops
            if !is_success {
                break;
            }
        }
        
        Ok(results)
    }
    
    /// Save all data to persistent storage
    pub async fn save(&self) -> Result<()> {
        self.storage.save().await
    }
    
    /// Load data from persistent storage
    pub async fn load(&mut self) -> Result<()> {
        self.storage.load().await
    }
}

impl MessageStorage {
    /// Create new message storage
    pub fn new(storage_path: PathBuf) -> Self {
        MessageStorage {
            storage_path,
            message_history: HashMap::new(),
            max_messages_per_conversation: 10000,
        }
    }
    
    /// Store a message
    pub fn store_message(&mut self, peer: &FourWordAddress, message: StoredMessage) {
        let conversation = self.message_history
            .entry(peer.clone())
            .or_insert_with(Vec::new);
        
        conversation.push(message);
        
        // Respect message limits
        if conversation.len() > self.max_messages_per_conversation {
            conversation.remove(0); // Remove oldest
        }
    }
    
    /// Get conversation history
    pub fn get_conversation_history(&self, peer: &FourWordAddress) -> Vec<StoredMessage> {
        self.message_history
            .get(peer)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Get conversations summary
    pub fn get_conversations_summary(&self) -> Vec<ConversationSummary> {
        self.message_history
            .iter()
            .map(|(peer, messages)| {
                let message_count = messages.len();
                let last_message_time = messages
                    .last()
                    .map(|m| m.message.timestamp);
                
                // Count unread messages (placeholder logic)
                let unread_count = messages
                    .iter()
                    .filter(|m| matches!(m.status, super::message::DeliveryStatus::Pending))
                    .count();
                
                ConversationSummary {
                    peer: peer.clone(),
                    message_count,
                    last_message_time,
                    unread_count,
                }
            })
            .collect()
    }
    
    /// Save to persistent storage (placeholder)
    pub async fn save(&self) -> Result<()> {
        // TODO: Implement actual persistence
        // For now, just ensure directory exists
        tokio::fs::create_dir_all(&self.storage_path).await
            .context("Failed to create storage directory")?;
        
        Ok(())
    }
    
    /// Load from persistent storage (placeholder)
    pub async fn load(&mut self) -> Result<()> {
        // TODO: Implement actual loading
        // For now, just ensure directory exists
        tokio::fs::create_dir_all(&self.storage_path).await
            .context("Failed to create storage directory")?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_secure_messaging_creation() {
        let temp_dir = TempDir::new().unwrap();
        let messaging = SecureMessaging::new(temp_dir.path().to_path_buf());
        
        // Should be created successfully
        assert!(messaging.identity_manager.is_none());
    }
    
    #[tokio::test]
    async fn test_message_encryption_decryption() {
        let temp_dir = TempDir::new().unwrap();
        let messaging = SecureMessaging::new(temp_dir.path().to_path_buf());
        
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        let message = Message::new_text(from, to, "Test message".to_string());
        
        // Encrypt message
        let envelope = messaging.encrypt_message(&message).unwrap();
        assert!(!envelope.encrypted_payload.is_empty());
        assert!(!envelope.signature.is_empty());
        
        // Decrypt envelope
        let decrypted = messaging.decrypt_envelope(envelope).unwrap();
        assert!(decrypted.is_some());
        
        let decrypted_message = decrypted.unwrap();
        assert_eq!(decrypted_message.content, message.content);
        assert_eq!(decrypted_message.from, message.from);
        assert_eq!(decrypted_message.to, message.to);
    }
    
    #[tokio::test]
    async fn test_message_storage() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = MessageStorage::new(temp_dir.path().to_path_buf());
        
        let peer = FourWordAddress::generate().unwrap();
        let from = FourWordAddress::generate().unwrap();
        let message = Message::new_text(from, peer.clone(), "Test".to_string());
        let stored = StoredMessage::new(message);
        
        // Store message
        storage.store_message(&peer, stored.clone());
        
        // Retrieve conversation history
        let history = storage.get_conversation_history(&peer);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].message.content, stored.message.content);
    }
    
    #[tokio::test]
    async fn test_conversation_summary() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = MessageStorage::new(temp_dir.path().to_path_buf());
        
        let peer = FourWordAddress::generate().unwrap();
        let from = FourWordAddress::generate().unwrap();
        
        // Add multiple messages
        for i in 0..3 {
            let message = Message::new_text(
                from.clone(), 
                peer.clone(), 
                format!("Message {}", i)
            );
            let stored = StoredMessage::new(message);
            storage.store_message(&peer, stored);
        }
        
        let summaries = storage.get_conversations_summary();
        assert_eq!(summaries.len(), 1);
        
        let summary = &summaries[0];
        assert_eq!(summary.peer, peer);
        assert_eq!(summary.message_count, 3);
        assert!(summary.last_message_time.is_some());
    }
    
    #[tokio::test]
    async fn test_message_limit_enforcement() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = MessageStorage::new(temp_dir.path().to_path_buf());
        storage.max_messages_per_conversation = 2; // Set small limit for testing
        
        let peer = FourWordAddress::generate().unwrap();
        let from = FourWordAddress::generate().unwrap();
        
        // Add messages beyond limit
        for i in 0..4 {
            let message = Message::new_text(
                from.clone(), 
                peer.clone(), 
                format!("Message {}", i)
            );
            let stored = StoredMessage::new(message);
            storage.store_message(&peer, stored);
        }
        
        let history = storage.get_conversation_history(&peer);
        assert_eq!(history.len(), 2); // Should respect limit
        
        // Should have the latest messages
        assert!(history[0].message.content.contains("Message 2"));
        assert!(history[1].message.content.contains("Message 3"));
    }
    
    #[tokio::test]
    async fn test_storage_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = MessageStorage::new(temp_dir.path().to_path_buf());
        
        // Save and load should not fail
        storage.save().await.unwrap();
        storage.load().await.unwrap();
        
        // Directory should exist
        assert!(temp_dir.path().exists());
    }
}