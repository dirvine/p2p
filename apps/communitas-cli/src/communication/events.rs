// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Event system for notification architecture

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use crate::identity::FourWordAddress;
use super::message::Message;

/// Event types that can trigger notifications
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    /// New message received
    MessageReceived {
        from: FourWordAddress,
        message_id: Uuid,
        content_preview: String,
    },
    /// Message sent successfully
    MessageSent {
        to: FourWordAddress,
        message_id: Uuid,
    },
    /// Message delivery confirmed
    MessageDelivered {
        to: FourWordAddress,
        message_id: Uuid,
    },
    /// Network peer connected
    PeerConnected {
        peer_address: FourWordAddress,
        connection_quality: ConnectionQuality,
    },
    /// Network peer disconnected
    PeerDisconnected {
        peer_address: FourWordAddress,
        reason: String,
    },
    /// Network status changed
    NetworkStatusChanged {
        status: NetworkStatus,
        peer_count: u32,
    },
    /// System notification
    SystemNotification {
        level: NotificationLevel,
        message: String,
    },
}

/// Connection quality indicators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionQuality {
    Excellent,
    Good,
    Fair,
    Poor,
}

/// Network status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NetworkStatus {
    Online,
    Offline,
    Connecting,
    Reconnecting,
    Error(String),
}

/// Notification severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub event_type: EventType,
    pub timestamp: u64,
    pub source: String,
}

/// Event handler trait
pub trait EventHandler: Send + Sync {
    fn handle_event(&self, event: &Event) -> Result<()>;
    fn can_handle(&self, event_type: &EventType) -> bool;
}

/// Event subscriber callback type
type EventCallback = Arc<dyn Fn(&Event) -> Result<()> + Send + Sync>;

/// Event publisher for managing subscriptions and publishing events
pub struct EventPublisher {
    subscribers: Arc<Mutex<HashMap<String, Vec<EventCallback>>>>,
    handlers: Arc<Mutex<Vec<Arc<dyn EventHandler>>>>,
}

impl std::fmt::Debug for EventPublisher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventPublisher")
            .field("subscriber_count", &self.subscribers.lock().unwrap().len())
            .field("handler_count", &self.handlers.lock().unwrap().len())
            .finish()
    }
}

impl Event {
    /// Create a new event
    pub fn new(event_type: EventType, source: String) -> Self {
        Event {
            id: Uuid::new_v4(),
            event_type,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            source,
        }
    }
    
    /// Create a message received event
    pub fn message_received(from: FourWordAddress, message: &Message) -> Self {
        let content_preview = if message.content.len() > 50 {
            format!("{}...", &message.content[..50])
        } else {
            message.content.clone()
        };
        
        Event::new(
            EventType::MessageReceived {
                from,
                message_id: message.id,
                content_preview,
            },
            "messaging".to_string(),
        )
    }
    
    /// Create a message sent event
    pub fn message_sent(to: FourWordAddress, message_id: Uuid) -> Self {
        Event::new(
            EventType::MessageSent { to, message_id },
            "messaging".to_string(),
        )
    }
    
    /// Create a message delivered event
    pub fn message_delivered(to: FourWordAddress, message_id: Uuid) -> Self {
        Event::new(
            EventType::MessageDelivered { to, message_id },
            "messaging".to_string(),
        )
    }
    
    /// Create a peer connected event
    pub fn peer_connected(peer_address: FourWordAddress, quality: ConnectionQuality) -> Self {
        Event::new(
            EventType::PeerConnected {
                peer_address,
                connection_quality: quality,
            },
            "network".to_string(),
        )
    }
    
    /// Create a peer disconnected event
    pub fn peer_disconnected(peer_address: FourWordAddress, reason: String) -> Self {
        Event::new(
            EventType::PeerDisconnected {
                peer_address,
                reason,
            },
            "network".to_string(),
        )
    }
    
    /// Create a network status changed event
    pub fn network_status_changed(status: NetworkStatus, peer_count: u32) -> Self {
        Event::new(
            EventType::NetworkStatusChanged { status, peer_count },
            "network".to_string(),
        )
    }
    
    /// Create a system notification event
    pub fn system_notification(level: NotificationLevel, message: String) -> Self {
        Event::new(
            EventType::SystemNotification { level, message },
            "system".to_string(),
        )
    }
    
    /// Get event category for filtering
    pub fn category(&self) -> &str {
        match &self.event_type {
            EventType::MessageReceived { .. } 
            | EventType::MessageSent { .. }
            | EventType::MessageDelivered { .. } => "messaging",
            EventType::PeerConnected { .. }
            | EventType::PeerDisconnected { .. }
            | EventType::NetworkStatusChanged { .. } => "network",
            EventType::SystemNotification { .. } => "system",
        }
    }
}

impl EventPublisher {
    /// Create a new event publisher
    pub fn new() -> Self {
        EventPublisher {
            subscribers: Arc::new(Mutex::new(HashMap::new())),
            handlers: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Subscribe to events by category
    pub fn subscribe<F>(&self, category: &str, callback: F) -> Result<String>
    where
        F: Fn(&Event) -> Result<()> + Send + Sync + 'static,
    {
        let subscriber_id = Uuid::new_v4().to_string();
        let mut subscribers = self.subscribers.lock().unwrap();
        
        let callbacks = subscribers.entry(category.to_string()).or_insert_with(Vec::new);
        callbacks.push(Arc::new(callback));
        
        Ok(subscriber_id)
    }
    
    /// Subscribe to all events
    pub fn subscribe_all<F>(&self, callback: F) -> Result<String>
    where
        F: Fn(&Event) -> Result<()> + Send + Sync + 'static,
    {
        self.subscribe("*", callback)
    }
    
    /// Add an event handler
    pub fn add_handler(&self, handler: Arc<dyn EventHandler>) {
        let mut handlers = self.handlers.lock().unwrap();
        handlers.push(handler);
    }
    
    /// Publish an event to all subscribers
    pub fn publish(&self, event: Event) -> Result<()> {
        let subscribers = self.subscribers.lock().unwrap();
        let handlers = self.handlers.lock().unwrap();
        
        // Send to category-specific subscribers
        if let Some(callbacks) = subscribers.get(event.category()) {
            for callback in callbacks {
                if let Err(e) = callback(&event) {
                    eprintln!("Error in event callback: {}", e);
                }
            }
        }
        
        // Send to universal subscribers
        if let Some(callbacks) = subscribers.get("*") {
            for callback in callbacks {
                if let Err(e) = callback(&event) {
                    eprintln!("Error in universal event callback: {}", e);
                }
            }
        }
        
        // Send to registered handlers
        for handler in handlers.iter() {
            if handler.can_handle(&event.event_type) {
                if let Err(e) = handler.handle_event(&event) {
                    eprintln!("Error in event handler: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Get subscriber count for a category
    pub fn subscriber_count(&self, category: &str) -> usize {
        let subscribers = self.subscribers.lock().unwrap();
        subscribers.get(category).map(|v| v.len()).unwrap_or(0)
    }
    
    /// Clear all subscribers (for testing)
    pub fn clear_subscribers(&self) {
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers.clear();
        
        let mut handlers = self.handlers.lock().unwrap();
        handlers.clear();
    }
}

impl Default for EventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_event_creation() {
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        let message = Message::new_text(from.clone(), to, "Test message".to_string());
        
        let event = Event::message_received(from.clone(), &message);
        
        assert_eq!(event.category(), "messaging");
        assert_eq!(event.source, "messaging");
        
        if let EventType::MessageReceived { from: event_from, message_id, content_preview } = event.event_type {
            assert_eq!(event_from, from);
            assert_eq!(message_id, message.id);
            assert_eq!(content_preview, "Test message");
        } else {
            panic!("Expected MessageReceived event type");
        }
    }
    
    #[test]
    fn test_content_preview_truncation() {
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        let long_content = "a".repeat(100);
        let message = Message::new_text(from.clone(), to, long_content);
        
        let event = Event::message_received(from, &message);
        
        if let EventType::MessageReceived { content_preview, .. } = event.event_type {
            assert_eq!(content_preview.len(), 53); // 50 + "..."
            assert!(content_preview.ends_with("..."));
        } else {
            panic!("Expected MessageReceived event type");
        }
    }
    
    #[test]
    fn test_event_types() {
        let addr = FourWordAddress::generate().unwrap();
        let msg_id = Uuid::new_v4();
        
        // Test all event types
        let events = vec![
            Event::message_sent(addr.clone(), msg_id),
            Event::message_delivered(addr.clone(), msg_id),
            Event::peer_connected(addr.clone(), ConnectionQuality::Good),
            Event::peer_disconnected(addr.clone(), "Timeout".to_string()),
            Event::network_status_changed(NetworkStatus::Online, 5),
            Event::system_notification(NotificationLevel::Info, "System ready".to_string()),
        ];
        
        for event in events {
            assert!(!event.id.to_string().is_empty());
            assert!(event.timestamp > 0);
        }
    }
    
    #[test]
    fn test_event_categories() {
        let addr = FourWordAddress::generate().unwrap();
        let msg_id = Uuid::new_v4();
        
        let messaging_event = Event::message_sent(addr.clone(), msg_id);
        assert_eq!(messaging_event.category(), "messaging");
        
        let network_event = Event::peer_connected(addr, ConnectionQuality::Good);
        assert_eq!(network_event.category(), "network");
        
        let system_event = Event::system_notification(NotificationLevel::Info, "Test".to_string());
        assert_eq!(system_event.category(), "system");
    }
    
    #[test]
    fn test_event_publisher_creation() {
        let publisher = EventPublisher::new();
        assert_eq!(publisher.subscriber_count("messaging"), 0);
        assert_eq!(publisher.subscriber_count("network"), 0);
    }
    
    #[test]
    fn test_event_subscription() {
        let publisher = EventPublisher::new();
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();
        
        let _subscription = publisher.subscribe("messaging", move |_event| {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }).unwrap();
        
        assert_eq!(publisher.subscriber_count("messaging"), 1);
    }
    
    #[test]
    fn test_event_publishing() {
        let publisher = EventPublisher::new();
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();
        
        publisher.subscribe("messaging", move |_event| {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }).unwrap();
        
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        let message = Message::new_text(from.clone(), to, "Test".to_string());
        let event = Event::message_received(from, &message);
        
        publisher.publish(event).unwrap();
        
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
    
    #[test]
    fn test_universal_subscription() {
        let publisher = EventPublisher::new();
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();
        
        publisher.subscribe_all(move |_event| {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }).unwrap();
        
        // Publish different types of events
        let addr = FourWordAddress::generate().unwrap();
        publisher.publish(Event::message_sent(addr.clone(), Uuid::new_v4())).unwrap();
        publisher.publish(Event::peer_connected(addr, ConnectionQuality::Good)).unwrap();
        publisher.publish(Event::system_notification(NotificationLevel::Info, "Test".to_string())).unwrap();
        
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }
    
    #[test]
    fn test_multiple_subscribers() {
        let publisher = EventPublisher::new();
        let call_count1 = Arc::new(AtomicU32::new(0));
        let call_count2 = Arc::new(AtomicU32::new(0));
        let call_count1_clone = call_count1.clone();
        let call_count2_clone = call_count2.clone();
        
        publisher.subscribe("messaging", move |_event| {
            call_count1_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }).unwrap();
        
        publisher.subscribe("messaging", move |_event| {
            call_count2_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }).unwrap();
        
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        let message = Message::new_text(from.clone(), to, "Test".to_string());
        let event = Event::message_received(from, &message);
        
        publisher.publish(event).unwrap();
        
        assert_eq!(call_count1.load(Ordering::SeqCst), 1);
        assert_eq!(call_count2.load(Ordering::SeqCst), 1);
    }
    
    #[test]
    fn test_category_filtering() {
        let publisher = EventPublisher::new();
        let messaging_count = Arc::new(AtomicU32::new(0));
        let network_count = Arc::new(AtomicU32::new(0));
        let messaging_count_clone = messaging_count.clone();
        let network_count_clone = network_count.clone();
        
        publisher.subscribe("messaging", move |_event| {
            messaging_count_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }).unwrap();
        
        publisher.subscribe("network", move |_event| {
            network_count_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }).unwrap();
        
        let addr = FourWordAddress::generate().unwrap();
        
        // Publish messaging event
        publisher.publish(Event::message_sent(addr.clone(), Uuid::new_v4())).unwrap();
        
        // Publish network event
        publisher.publish(Event::peer_connected(addr, ConnectionQuality::Good)).unwrap();
        
        assert_eq!(messaging_count.load(Ordering::SeqCst), 1);
        assert_eq!(network_count.load(Ordering::SeqCst), 1);
    }
    
    #[test]
    fn test_clear_subscribers() {
        let publisher = EventPublisher::new();
        
        publisher.subscribe("messaging", |_event| Ok(())).unwrap();
        assert_eq!(publisher.subscriber_count("messaging"), 1);
        
        publisher.clear_subscribers();
        assert_eq!(publisher.subscriber_count("messaging"), 0);
    }
    
    // Test event handler trait
    struct TestEventHandler {
        call_count: Arc<AtomicU32>,
    }
    
    impl EventHandler for TestEventHandler {
        fn handle_event(&self, _event: &Event) -> Result<()> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        
        fn can_handle(&self, event_type: &EventType) -> bool {
            matches!(event_type, EventType::MessageReceived { .. })
        }
    }
    
    #[test]
    fn test_event_handler() {
        let publisher = EventPublisher::new();
        let call_count = Arc::new(AtomicU32::new(0));
        let handler = Arc::new(TestEventHandler {
            call_count: call_count.clone(),
        });
        
        publisher.add_handler(handler);
        
        let from = FourWordAddress::generate().unwrap();
        let to = FourWordAddress::generate().unwrap();
        let message = Message::new_text(from.clone(), to, "Test".to_string());
        let event = Event::message_received(from, &message);
        
        publisher.publish(event).unwrap();
        
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        
        // Publish an event that the handler can't handle
        let addr = FourWordAddress::generate().unwrap();
        publisher.publish(Event::peer_connected(addr, ConnectionQuality::Good)).unwrap();
        
        // Call count should remain the same
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
}