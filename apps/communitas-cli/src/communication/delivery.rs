// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Message delivery mechanisms and confirmation system

use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;
use crate::identity::FourWordAddress;
use super::message::{Message, StoredMessage};

/// Message delivery manager
#[derive(Debug)]
pub struct MessageDelivery {
    /// Outbound message queue
    outbound_queue: VecDeque<StoredMessage>,
    /// Messages awaiting delivery confirmation
    pending_confirmations: HashMap<Uuid, StoredMessage>,
    /// Offline message queue per recipient
    offline_queue: HashMap<FourWordAddress, VecDeque<StoredMessage>>,
    /// Maximum queue size per recipient
    max_queue_size: usize,
    /// Maximum retry attempts
    max_retry_attempts: u32,
}

/// Delivery result information
#[derive(Debug, Clone, PartialEq)]
pub enum DeliveryResult {
    Success,
    RecipientOffline,
    NetworkError(String),
    EncryptionError(String),
    InvalidRecipient,
}

impl MessageDelivery {
    /// Create new message delivery manager
    pub fn new() -> Self {
        MessageDelivery {
            outbound_queue: VecDeque::new(),
            pending_confirmations: HashMap::new(),
            offline_queue: HashMap::new(),
            max_queue_size: 1000,
            max_retry_attempts: 3,
        }
    }
    
    /// Queue message for delivery
    pub fn queue_message(&mut self, message: Message) -> Result<()> {
        let stored = StoredMessage::new(message);
        self.outbound_queue.push_back(stored);
        Ok(())
    }
    
    /// Attempt to deliver next message in queue
    pub async fn deliver_next_message(&mut self) -> Result<Option<DeliveryResult>> {
        if let Some(mut stored_message) = self.outbound_queue.pop_front() {
            // Simulate delivery attempt (will be replaced with actual network code)
            let result = self.attempt_delivery(&stored_message.message).await;
            
            match result {
                DeliveryResult::Success => {
                    stored_message.mark_sent();
                    // Store for confirmation tracking
                    self.pending_confirmations.insert(
                        stored_message.message.id, 
                        stored_message
                    );
                    Ok(Some(DeliveryResult::Success))
                }
                DeliveryResult::RecipientOffline => {
                    // Queue for offline delivery
                    self.queue_for_offline_delivery(stored_message)?;
                    Ok(Some(DeliveryResult::RecipientOffline))
                }
                other => {
                    stored_message.mark_failed(format!("{:?}", other));
                    
                    // Retry if under limit
                    if stored_message.attempts < self.max_retry_attempts {
                        self.outbound_queue.push_back(stored_message);
                    }
                    
                    Ok(Some(other))
                }
            }
        } else {
            Ok(None)
        }
    }
    
    /// Process delivery confirmation
    pub fn process_delivery_confirmation(&mut self, message_id: Uuid) -> Result<bool> {
        if let Some(mut stored_message) = self.pending_confirmations.remove(&message_id) {
            stored_message.mark_delivered();
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Queue message for offline delivery
    fn queue_for_offline_delivery(&mut self, stored_message: StoredMessage) -> Result<()> {
        let recipient = stored_message.message.to.clone();
        let queue = self.offline_queue.entry(recipient).or_insert_with(VecDeque::new);
        
        // Respect queue size limits
        if queue.len() >= self.max_queue_size {
            queue.pop_front(); // Remove oldest message
        }
        
        queue.push_back(stored_message);
        Ok(())
    }
    
    /// Get offline messages for recipient (when they come online)
    pub fn get_offline_messages(&mut self, recipient: &FourWordAddress) -> Vec<StoredMessage> {
        if let Some(queue) = self.offline_queue.remove(recipient) {
            queue.into_iter().collect()
        } else {
            Vec::new()
        }
    }
    
    /// Attempt delivery of a message (placeholder implementation)
    async fn attempt_delivery(&self, _message: &Message) -> DeliveryResult {
        // TODO: Replace with actual network delivery implementation
        // This is a placeholder that simulates various delivery outcomes
        
        // For now, randomly simulate different outcomes for testing
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        match rng.gen_range(0..4) {
            0 => DeliveryResult::Success,
            1 => DeliveryResult::RecipientOffline,
            2 => DeliveryResult::NetworkError("Connection timeout".to_string()),
            _ => DeliveryResult::Success, // Bias towards success for testing
        }
    }
    
    /// Get queue statistics
    pub fn get_queue_stats(&self) -> QueueStats {
        QueueStats {
            outbound_count: self.outbound_queue.len(),
            pending_confirmations: self.pending_confirmations.len(),
            offline_recipients: self.offline_queue.len(),
            total_offline_messages: self.offline_queue.values()
                .map(|queue| queue.len())
                .sum(),
        }
    }
    
    /// Clear all queues (for testing)
    pub fn clear_all_queues(&mut self) {
        self.outbound_queue.clear();
        self.pending_confirmations.clear();
        self.offline_queue.clear();
    }
}

/// Queue statistics
#[derive(Debug, Clone, PartialEq)]
pub struct QueueStats {
    pub outbound_count: usize,
    pub pending_confirmations: usize,
    pub offline_recipients: usize,
    pub total_offline_messages: usize,
}

impl Default for MessageDelivery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_delivery_creation() {
        let delivery = MessageDelivery::new();
        assert_eq!(delivery.outbound_queue.len(), 0);
        assert_eq!(delivery.pending_confirmations.len(), 0);
        assert_eq!(delivery.offline_queue.len(), 0);
    }
    
    #[tokio::test]
    async fn test_queue_message() {
        let mut delivery = MessageDelivery::new();
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        let message = Message::new_text(from, to, "Test".to_string());
        
        delivery.queue_message(message).unwrap();
        assert_eq!(delivery.outbound_queue.len(), 1);
    }
    
    #[tokio::test]
    async fn test_deliver_next_message() {
        let mut delivery = MessageDelivery::new();
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        let message = Message::new_text(from, to, "Test".to_string());
        
        delivery.queue_message(message).unwrap();
        
        // Attempt delivery
        let result = delivery.deliver_next_message().await.unwrap();
        assert!(result.is_some());
        
        // Message should be processed (queue might be empty or contain retry)
        // Due to random delivery simulation, queue could be empty (success) 
        // or contain the message again (failed delivery with retry)
        assert!(delivery.outbound_queue.len() <= 1);
    }
    
    #[tokio::test]
    async fn test_delivery_confirmation() {
        let mut delivery = MessageDelivery::new();
        let message_id = Uuid::new_v4();
        
        // Add a pending confirmation manually for testing
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        let mut message = Message::new_text(from, to, "Test".to_string());
        message.id = message_id; // Set specific ID for testing
        
        let mut stored = StoredMessage::new(message);
        stored.mark_sent();
        delivery.pending_confirmations.insert(message_id, stored);
        
        // Process confirmation
        let confirmed = delivery.process_delivery_confirmation(message_id).unwrap();
        assert!(confirmed);
        assert!(!delivery.pending_confirmations.contains_key(&message_id));
    }
    
    #[tokio::test]
    async fn test_offline_message_queueing() {
        let mut delivery = MessageDelivery::new();
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        let message = Message::new_text(from, to.clone(), "Offline test".to_string());
        let stored = StoredMessage::new(message);
        
        delivery.queue_for_offline_delivery(stored).unwrap();
        
        let stats = delivery.get_queue_stats();
        assert_eq!(stats.offline_recipients, 1);
        assert_eq!(stats.total_offline_messages, 1);
        
        // Retrieve offline messages
        let offline_messages = delivery.get_offline_messages(&to);
        assert_eq!(offline_messages.len(), 1);
        
        // Should be empty after retrieval
        let empty_messages = delivery.get_offline_messages(&to);
        assert_eq!(empty_messages.len(), 0);
    }
    
    #[tokio::test]
    async fn test_queue_size_limits() {
        let mut delivery = MessageDelivery::new();
        delivery.max_queue_size = 2; // Set small limit for testing
        
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        
        // Add messages beyond limit
        for i in 0..3 {
            let message = Message::new_text(
                from.clone(), 
                to.clone(), 
                format!("Message {}", i)
            );
            let stored = StoredMessage::new(message);
            delivery.queue_for_offline_delivery(stored).unwrap();
        }
        
        let stats = delivery.get_queue_stats();
        assert_eq!(stats.total_offline_messages, 2); // Should respect limit
    }
    
    #[tokio::test]
    async fn test_queue_stats() {
        let mut delivery = MessageDelivery::new();
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        
        // Add outbound message
        let message1 = Message::new_text(from.clone(), to.clone(), "Outbound".to_string());
        delivery.queue_message(message1).unwrap();
        
        // Add offline message
        let message2 = Message::new_text(from.clone(), to.clone(), "Offline".to_string());
        let stored = StoredMessage::new(message2);
        delivery.queue_for_offline_delivery(stored).unwrap();
        
        let stats = delivery.get_queue_stats();
        assert_eq!(stats.outbound_count, 1);
        assert_eq!(stats.offline_recipients, 1);
        assert_eq!(stats.total_offline_messages, 1);
    }
    
    #[tokio::test]
    async fn test_clear_all_queues() {
        let mut delivery = MessageDelivery::new();
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        
        // Add messages to all queues
        let message1 = Message::new_text(from.clone(), to.clone(), "Test1".to_string());
        delivery.queue_message(message1).unwrap();
        
        let message2 = Message::new_text(from.clone(), to.clone(), "Test2".to_string());
        let stored = StoredMessage::new(message2);
        delivery.queue_for_offline_delivery(stored).unwrap();
        
        delivery.clear_all_queues();
        
        let stats = delivery.get_queue_stats();
        assert_eq!(stats.outbound_count, 0);
        assert_eq!(stats.offline_recipients, 0);
        assert_eq!(stats.total_offline_messages, 0);
    }
}