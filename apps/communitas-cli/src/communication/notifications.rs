// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Notification system for real-time events and alerts

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use chrono::Timelike;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use crate::identity::FourWordAddress;
use super::events::{Event, EventType, EventPublisher, NotificationLevel};

/// Notification content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Notification {
    pub id: Uuid,
    pub title: String,
    pub message: String,
    pub level: NotificationLevel,
    pub category: String,
    pub timestamp: u64,
    pub read: bool,
    pub source_event_id: Option<Uuid>,
    pub actions: Vec<NotificationAction>,
}

/// Notification action that can be taken
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
    pub action_type: ActionType,
}

/// Types of actions available on notifications
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    /// Reply to a message
    ReplyMessage { conversation_id: Uuid },
    /// View conversation
    ViewConversation { peer: FourWordAddress },
    /// Connect to peer
    ConnectToPeer { peer: FourWordAddress },
    /// Dismiss notification
    Dismiss,
    /// Open settings
    OpenSettings,
}

/// Configuration for notification preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Enable message notifications
    pub message_notifications: bool,
    /// Enable network notifications
    pub network_notifications: bool,
    /// Enable system notifications  
    pub system_notifications: bool,
    /// Notification sound enabled
    pub sound_enabled: bool,
    /// Show notification previews
    pub show_previews: bool,
    /// Maximum notifications to keep
    pub max_notifications: usize,
    /// Auto-mark as read after seconds
    pub auto_mark_read_seconds: Option<u64>,
    /// Quiet hours (start_hour, end_hour) in 24h format
    pub quiet_hours: Option<(u8, u8)>,
}

/// Notification storage and persistence
#[derive(Debug)]
pub struct NotificationStorage {
    storage_path: PathBuf,
    notifications: VecDeque<Notification>,
    max_notifications: usize,
}

/// Main notification manager
pub struct NotificationManager {
    storage: Arc<Mutex<NotificationStorage>>,
    config: Arc<Mutex<NotificationConfig>>,
    event_publisher: Arc<EventPublisher>,
    subscribers: Arc<Mutex<Vec<Arc<dyn NotificationHandler>>>>,
}

impl std::fmt::Debug for NotificationManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotificationManager")
            .field("notification_count", &self.notification_count())
            .field("unread_count", &self.unread_notification_count())
            .field("subscriber_count", &self.subscribers.lock().unwrap().len())
            .finish()
    }
}

/// Handler trait for processing notifications
pub trait NotificationHandler: Send + Sync {
    fn handle_notification(&self, notification: &Notification) -> Result<()>;
}

impl Default for NotificationConfig {
    fn default() -> Self {
        NotificationConfig {
            message_notifications: true,
            network_notifications: true,
            system_notifications: true,
            sound_enabled: true,
            show_previews: true,
            max_notifications: 1000,
            auto_mark_read_seconds: None,
            quiet_hours: None,
        }
    }
}

impl Notification {
    /// Create a new notification
    pub fn new(
        title: String,
        message: String,
        level: NotificationLevel,
        category: String,
    ) -> Self {
        Notification {
            id: Uuid::new_v4(),
            title,
            message,
            level,
            category,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            read: false,
            source_event_id: None,
            actions: Vec::new(),
        }
    }
    
    /// Create notification from a message received event
    pub fn from_message_received(from: &FourWordAddress, content_preview: &str, message_id: Uuid) -> Self {
        let mut notification = Notification::new(
            "New Message".to_string(),
            format!("From {}: {}", from, content_preview),
            NotificationLevel::Info,
            "messaging".to_string(),
        );
        
        notification.actions = vec![
            NotificationAction {
                id: "reply".to_string(),
                label: "Reply".to_string(),
                action_type: ActionType::ReplyMessage { conversation_id: message_id },
            },
            NotificationAction {
                id: "view".to_string(),
                label: "View Conversation".to_string(),
                action_type: ActionType::ViewConversation { peer: from.clone() },
            },
            NotificationAction {
                id: "dismiss".to_string(),
                label: "Dismiss".to_string(),
                action_type: ActionType::Dismiss,
            },
        ];
        
        notification
    }
    
    /// Create notification from a peer connected event
    pub fn from_peer_connected(peer: &FourWordAddress) -> Self {
        let mut notification = Notification::new(
            "Peer Connected".to_string(),
            format!("{} is now online", peer),
            NotificationLevel::Info,
            "network".to_string(),
        );
        
        notification.actions = vec![
            NotificationAction {
                id: "message".to_string(),
                label: "Send Message".to_string(),
                action_type: ActionType::ViewConversation { peer: peer.clone() },
            },
            NotificationAction {
                id: "dismiss".to_string(),
                label: "Dismiss".to_string(),
                action_type: ActionType::Dismiss,
            },
        ];
        
        notification
    }
    
    /// Create notification from a peer disconnected event
    pub fn from_peer_disconnected(peer: &FourWordAddress, reason: &str) -> Self {
        Notification::new(
            "Peer Disconnected".to_string(),
            format!("{} went offline: {}", peer, reason),
            NotificationLevel::Warning,
            "network".to_string(),
        )
    }
    
    /// Create notification from a system event
    pub fn from_system_notification(level: NotificationLevel, message: &str) -> Self {
        let title = match level {
            NotificationLevel::Info => "Information",
            NotificationLevel::Warning => "Warning",
            NotificationLevel::Error => "Error",
            NotificationLevel::Critical => "Critical Alert",
        };
        
        Notification::new(
            title.to_string(),
            message.to_string(),
            level,
            "system".to_string(),
        )
    }
    
    /// Mark notification as read
    pub fn mark_read(&mut self) {
        self.read = true;
    }
    
    /// Check if notification is expired based on age
    pub fn is_expired(&self, max_age_seconds: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.timestamp > max_age_seconds
    }
}

impl NotificationStorage {
    /// Create new notification storage
    pub fn new(storage_path: PathBuf) -> Self {
        NotificationStorage {
            storage_path,
            notifications: VecDeque::new(),
            max_notifications: 1000,
        }
    }
    
    /// Store a notification
    pub fn store(&mut self, notification: Notification) {
        self.notifications.push_back(notification);
        
        // Respect notification limits
        while self.notifications.len() > self.max_notifications {
            self.notifications.pop_front();
        }
    }
    
    /// Get all notifications
    pub fn get_all(&self) -> Vec<Notification> {
        self.notifications.iter().cloned().collect()
    }
    
    /// Get unread notifications
    pub fn get_unread(&self) -> Vec<Notification> {
        self.notifications
            .iter()
            .filter(|n| !n.read)
            .cloned()
            .collect()
    }
    
    /// Get notifications by category
    pub fn get_by_category(&self, category: &str) -> Vec<Notification> {
        self.notifications
            .iter()
            .filter(|n| n.category == category)
            .cloned()
            .collect()
    }
    
    /// Mark notification as read
    pub fn mark_read(&mut self, notification_id: Uuid) -> bool {
        if let Some(notification) = self.notifications.iter_mut().find(|n| n.id == notification_id) {
            notification.mark_read();
            true
        } else {
            false
        }
    }
    
    /// Mark all notifications as read
    pub fn mark_all_read(&mut self) {
        for notification in &mut self.notifications {
            notification.mark_read();
        }
    }
    
    /// Remove notification
    pub fn remove(&mut self, notification_id: Uuid) -> bool {
        let original_len = self.notifications.len();
        self.notifications.retain(|n| n.id != notification_id);
        self.notifications.len() != original_len
    }
    
    /// Clear all notifications
    pub fn clear(&mut self) {
        self.notifications.clear();
    }
    
    /// Clean up old notifications
    pub fn cleanup_old(&mut self, max_age_seconds: u64) {
        self.notifications.retain(|n| !n.is_expired(max_age_seconds));
    }
    
    /// Get notification count
    pub fn count(&self) -> usize {
        self.notifications.len()
    }
    
    /// Get unread count
    pub fn unread_count(&self) -> usize {
        self.notifications.iter().filter(|n| !n.read).count()
    }
    
    /// Set maximum notifications limit
    pub fn set_max_notifications(&mut self, max: usize) {
        self.max_notifications = max;
        while self.notifications.len() > max {
            self.notifications.pop_front();
        }
    }
    
    /// Save to persistent storage (placeholder)
    pub async fn save(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.storage_path).await
            .context("Failed to create notification storage directory")?;
        Ok(())
    }
    
    /// Load from persistent storage (placeholder)
    pub async fn load(&mut self) -> Result<()> {
        tokio::fs::create_dir_all(&self.storage_path).await
            .context("Failed to create notification storage directory")?;
        Ok(())
    }
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new(storage_path: PathBuf, event_publisher: Arc<EventPublisher>) -> Self {
        let storage = NotificationStorage::new(storage_path);
        let config = NotificationConfig::default();
        
        NotificationManager {
            storage: Arc::new(Mutex::new(storage)),
            config: Arc::new(Mutex::new(config)),
            event_publisher,
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Initialize the notification manager and set up event subscriptions
    pub fn initialize(&self) -> Result<()> {
        let manager = Arc::new(self.clone());
        
        // Subscribe to all events to generate notifications
        self.event_publisher.subscribe_all({
            let manager = manager.clone();
            move |event| {
                manager.process_event(event)
            }
        })?;
        
        Ok(())
    }
    
    /// Process an event and potentially create a notification
    fn process_event(&self, event: &Event) -> Result<()> {
        let config = self.config.lock().unwrap();
        
        // Check if notifications are enabled for this category
        let notifications_enabled = match event.category() {
            "messaging" => config.message_notifications,
            "network" => config.network_notifications,
            "system" => config.system_notifications,
            _ => true,
        };
        
        if !notifications_enabled {
            return Ok(());
        }
        
        // Check quiet hours
        if let Some((start_hour, end_hour)) = config.quiet_hours {
            let now = chrono::Local::now();
            let current_hour = now.hour() as u8;
            
            let in_quiet_hours = if start_hour <= end_hour {
                current_hour >= start_hour && current_hour < end_hour
            } else {
                current_hour >= start_hour || current_hour < end_hour
            };
            
            if in_quiet_hours {
                return Ok(());
            }
        }
        
        drop(config);
        
        // Create notification based on event type
        let notification = match &event.event_type {
            EventType::MessageReceived { from, content_preview, message_id } => {
                Some(Notification::from_message_received(from, content_preview, *message_id))
            }
            EventType::PeerConnected { peer_address, .. } => {
                Some(Notification::from_peer_connected(peer_address))
            }
            EventType::PeerDisconnected { peer_address, reason } => {
                Some(Notification::from_peer_disconnected(peer_address, reason))
            }
            EventType::SystemNotification { level, message } => {
                Some(Notification::from_system_notification(level.clone(), message))
            }
            _ => None, // Don't create notifications for all event types
        };
        
        if let Some(mut notification) = notification {
            notification.source_event_id = Some(event.id);
            self.add_notification(notification)?;
        }
        
        Ok(())
    }
    
    /// Add a notification to the system
    pub fn add_notification(&self, notification: Notification) -> Result<()> {
        let mut storage = self.storage.lock().unwrap();
        storage.store(notification.clone());
        drop(storage);
        
        // Notify all subscribers
        let subscribers = self.subscribers.lock().unwrap();
        for handler in subscribers.iter() {
            if let Err(e) = handler.handle_notification(&notification) {
                eprintln!("Error in notification handler: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Subscribe to notifications
    pub fn subscribe(&self, handler: Arc<dyn NotificationHandler>) {
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers.push(handler);
    }
    
    /// Get all notifications
    pub fn get_notifications(&self) -> Vec<Notification> {
        let storage = self.storage.lock().unwrap();
        storage.get_all()
    }
    
    /// Get unread notifications
    pub fn get_unread_notifications(&self) -> Vec<Notification> {
        let storage = self.storage.lock().unwrap();
        storage.get_unread()
    }
    
    /// Get notifications by category
    pub fn get_notifications_by_category(&self, category: &str) -> Vec<Notification> {
        let storage = self.storage.lock().unwrap();
        storage.get_by_category(category)
    }
    
    /// Mark notification as read
    pub fn mark_notification_read(&self, notification_id: Uuid) -> bool {
        let mut storage = self.storage.lock().unwrap();
        storage.mark_read(notification_id)
    }
    
    /// Mark all notifications as read
    pub fn mark_all_notifications_read(&self) {
        let mut storage = self.storage.lock().unwrap();
        storage.mark_all_read();
    }
    
    /// Remove a notification
    pub fn remove_notification(&self, notification_id: Uuid) -> bool {
        let mut storage = self.storage.lock().unwrap();
        storage.remove(notification_id)
    }
    
    /// Clear all notifications
    pub fn clear_all_notifications(&self) {
        let mut storage = self.storage.lock().unwrap();
        storage.clear();
    }
    
    /// Get notification count
    pub fn notification_count(&self) -> usize {
        let storage = self.storage.lock().unwrap();
        storage.count()
    }
    
    /// Get unread notification count
    pub fn unread_notification_count(&self) -> usize {
        let storage = self.storage.lock().unwrap();
        storage.unread_count()
    }
    
    /// Update configuration
    pub fn update_config(&self, new_config: NotificationConfig) {
        let mut config = self.config.lock().unwrap();
        *config = new_config;
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> NotificationConfig {
        let config = self.config.lock().unwrap();
        config.clone()
    }
    
    /// Perform maintenance (cleanup old notifications, auto-read, etc.)
    pub fn maintenance(&self) {
        let mut storage = self.storage.lock().unwrap();
        let config = self.config.lock().unwrap();
        
        // Cleanup old notifications (7 days)
        storage.cleanup_old(7 * 24 * 60 * 60);
        
        // Update max notifications if config changed
        storage.set_max_notifications(config.max_notifications);
        
        // TODO: Implement auto-mark-read based on config.auto_mark_read_seconds
    }
    
    /// Save all data to persistent storage
    pub async fn save(&self) -> Result<()> {
        let storage = self.storage.lock().unwrap();
        storage.save().await
    }
    
    /// Load data from persistent storage
    pub async fn load(&self) -> Result<()> {
        let mut storage = self.storage.lock().unwrap();
        storage.load().await
    }
}

// Manual Clone implementation for NotificationManager
impl Clone for NotificationManager {
    fn clone(&self) -> Self {
        NotificationManager {
            storage: self.storage.clone(),
            config: self.config.clone(),
            event_publisher: self.event_publisher.clone(),
            subscribers: self.subscribers.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_notification_creation() {
        let notification = Notification::new(
            "Test Title".to_string(),
            "Test Message".to_string(),
            NotificationLevel::Info,
            "test".to_string(),
        );
        
        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.message, "Test Message");
        assert_eq!(notification.level, NotificationLevel::Info);
        assert_eq!(notification.category, "test");
        assert!(!notification.read);
        assert!(notification.timestamp > 0);
        assert!(notification.actions.is_empty());
    }
    
    #[test]
    fn test_notification_from_message_received() {
        let from = FourWordAddress::generate().unwrap();
        let message_id = Uuid::new_v4();
        let notification = Notification::from_message_received(&from, "Hello!", message_id);
        
        assert_eq!(notification.title, "New Message");
        assert!(notification.message.contains(&from.to_string()));
        assert!(notification.message.contains("Hello!"));
        assert_eq!(notification.category, "messaging");
        assert_eq!(notification.actions.len(), 3);
        
        // Check actions
        let action_types: Vec<&str> = notification.actions.iter().map(|a| a.id.as_str()).collect();
        assert!(action_types.contains(&"reply"));
        assert!(action_types.contains(&"view"));
        assert!(action_types.contains(&"dismiss"));
    }
    
    #[test]
    fn test_notification_from_peer_events() {
        let peer = FourWordAddress::generate().unwrap();
        
        let connected = Notification::from_peer_connected(&peer);
        assert_eq!(connected.title, "Peer Connected");
        assert!(connected.message.contains(&peer.to_string()));
        assert_eq!(connected.level, NotificationLevel::Info);
        
        let disconnected = Notification::from_peer_disconnected(&peer, "Timeout");
        assert_eq!(disconnected.title, "Peer Disconnected");
        assert!(disconnected.message.contains(&peer.to_string()));
        assert!(disconnected.message.contains("Timeout"));
        assert_eq!(disconnected.level, NotificationLevel::Warning);
    }
    
    #[test]
    fn test_notification_from_system() {
        let notification = Notification::from_system_notification(
            NotificationLevel::Error,
            "System error occurred"
        );
        
        assert_eq!(notification.title, "Error");
        assert_eq!(notification.message, "System error occurred");
        assert_eq!(notification.level, NotificationLevel::Error);
        assert_eq!(notification.category, "system");
    }
    
    #[test]
    fn test_notification_mark_read() {
        let mut notification = Notification::new(
            "Test".to_string(),
            "Test".to_string(),
            NotificationLevel::Info,
            "test".to_string(),
        );
        
        assert!(!notification.read);
        notification.mark_read();
        assert!(notification.read);
    }
    
    #[test]
    fn test_notification_expiry() {
        let mut notification = Notification::new(
            "Test".to_string(),
            "Test".to_string(),
            NotificationLevel::Info,
            "test".to_string(),
        );
        
        // Set a very old timestamp for testing
        notification.timestamp = 1000;
        
        assert!(notification.is_expired(3600)); // Should be expired
        assert!(notification.is_expired(0)); // Immediate expiry
        
        // Test with current timestamp
        let current_notification = Notification::new(
            "Test".to_string(),
            "Test".to_string(),
            NotificationLevel::Info,
            "test".to_string(),
        );
        assert!(!current_notification.is_expired(3600)); // Should not be expired
    }
    
    #[test]
    fn test_notification_storage_creation() {
        let temp_dir = TempDir::new().unwrap();
        let storage = NotificationStorage::new(temp_dir.path().to_path_buf());
        
        assert_eq!(storage.count(), 0);
        assert_eq!(storage.unread_count(), 0);
    }
    
    #[test]
    fn test_notification_storage_operations() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = NotificationStorage::new(temp_dir.path().to_path_buf());
        
        let notification = Notification::new(
            "Test".to_string(),
            "Test Message".to_string(),
            NotificationLevel::Info,
            "test".to_string(),
        );
        let notification_id = notification.id;
        
        // Store notification
        storage.store(notification);
        assert_eq!(storage.count(), 1);
        assert_eq!(storage.unread_count(), 1);
        
        // Mark as read
        assert!(storage.mark_read(notification_id));
        assert_eq!(storage.unread_count(), 0);
        
        // Remove notification
        assert!(storage.remove(notification_id));
        assert_eq!(storage.count(), 0);
    }
    
    #[test]
    fn test_notification_storage_limits() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = NotificationStorage::new(temp_dir.path().to_path_buf());
        storage.set_max_notifications(2);
        
        // Add 3 notifications
        for i in 0..3 {
            let notification = Notification::new(
                format!("Test {}", i),
                "Test".to_string(),
                NotificationLevel::Info,
                "test".to_string(),
            );
            storage.store(notification);
        }
        
        // Should only keep the latest 2
        assert_eq!(storage.count(), 2);
        
        let notifications = storage.get_all();
        assert!(notifications[0].title.contains("Test 1"));
        assert!(notifications[1].title.contains("Test 2"));
    }
    
    #[test]
    fn test_notification_storage_filtering() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = NotificationStorage::new(temp_dir.path().to_path_buf());
        
        // Add notifications with different categories
        storage.store(Notification::new("Msg1".to_string(), "Test".to_string(), NotificationLevel::Info, "messaging".to_string()));
        storage.store(Notification::new("Net1".to_string(), "Test".to_string(), NotificationLevel::Info, "network".to_string()));
        storage.store(Notification::new("Msg2".to_string(), "Test".to_string(), NotificationLevel::Info, "messaging".to_string()));
        
        let messaging_notifications = storage.get_by_category("messaging");
        let network_notifications = storage.get_by_category("network");
        
        assert_eq!(messaging_notifications.len(), 2);
        assert_eq!(network_notifications.len(), 1);
    }
    
    #[test]
    fn test_notification_storage_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = NotificationStorage::new(temp_dir.path().to_path_buf());
        
        // Add old notification (create it with a past timestamp)
        let mut old_notification = Notification::new("Old".to_string(), "Test".to_string(), NotificationLevel::Info, "test".to_string());
        old_notification.timestamp = 1000; // Very old timestamp
        storage.store(old_notification);
        
        // Add recent notification
        storage.store(Notification::new("Recent".to_string(), "Test".to_string(), NotificationLevel::Info, "test".to_string()));
        
        assert_eq!(storage.count(), 2);
        
        // Cleanup old notifications (anything older than 3600 seconds)
        storage.cleanup_old(3600);
        
        assert_eq!(storage.count(), 1);
        let remaining = storage.get_all();
        assert_eq!(remaining[0].title, "Recent");
    }
    
    #[tokio::test]
    async fn test_notification_storage_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = NotificationStorage::new(temp_dir.path().to_path_buf());
        
        // Save and load should not fail
        storage.save().await.unwrap();
        storage.load().await.unwrap();
        
        // Directory should exist
        assert!(temp_dir.path().exists());
    }
    
    #[test]
    fn test_notification_config_default() {
        let config = NotificationConfig::default();
        
        assert!(config.message_notifications);
        assert!(config.network_notifications);
        assert!(config.system_notifications);
        assert!(config.sound_enabled);
        assert!(config.show_previews);
        assert_eq!(config.max_notifications, 1000);
        assert!(config.auto_mark_read_seconds.is_none());
        assert!(config.quiet_hours.is_none());
    }
    
    #[tokio::test]
    async fn test_notification_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let event_publisher = Arc::new(EventPublisher::new());
        let manager = NotificationManager::new(temp_dir.path().to_path_buf(), event_publisher);
        
        assert_eq!(manager.notification_count(), 0);
        assert_eq!(manager.unread_notification_count(), 0);
    }
    
    #[tokio::test]
    async fn test_notification_manager_operations() {
        let temp_dir = TempDir::new().unwrap();
        let event_publisher = Arc::new(EventPublisher::new());
        let manager = NotificationManager::new(temp_dir.path().to_path_buf(), event_publisher);
        
        let notification = Notification::new(
            "Test".to_string(),
            "Test Message".to_string(),
            NotificationLevel::Info,
            "test".to_string(),
        );
        let notification_id = notification.id;
        
        // Add notification
        manager.add_notification(notification).unwrap();
        assert_eq!(manager.notification_count(), 1);
        assert_eq!(manager.unread_notification_count(), 1);
        
        // Mark as read
        assert!(manager.mark_notification_read(notification_id));
        assert_eq!(manager.unread_notification_count(), 0);
        
        // Remove notification
        assert!(manager.remove_notification(notification_id));
        assert_eq!(manager.notification_count(), 0);
    }
    
    #[tokio::test]
    async fn test_notification_manager_config() {
        let temp_dir = TempDir::new().unwrap();
        let event_publisher = Arc::new(EventPublisher::new());
        let manager = NotificationManager::new(temp_dir.path().to_path_buf(), event_publisher);
        
        let mut config = manager.get_config();
        config.message_notifications = false;
        config.max_notifications = 500;
        
        manager.update_config(config);
        
        let updated_config = manager.get_config();
        assert!(!updated_config.message_notifications);
        assert_eq!(updated_config.max_notifications, 500);
    }
    
    // Test notification handler
    struct TestNotificationHandler {
        call_count: Arc<AtomicU32>,
    }
    
    impl NotificationHandler for TestNotificationHandler {
        fn handle_notification(&self, _notification: &Notification) -> Result<()> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }
    
    #[tokio::test]
    async fn test_notification_manager_subscription() {
        let temp_dir = TempDir::new().unwrap();
        let event_publisher = Arc::new(EventPublisher::new());
        let manager = NotificationManager::new(temp_dir.path().to_path_buf(), event_publisher);
        
        let call_count = Arc::new(AtomicU32::new(0));
        let handler = Arc::new(TestNotificationHandler {
            call_count: call_count.clone(),
        });
        
        manager.subscribe(handler);
        
        let notification = Notification::new(
            "Test".to_string(),
            "Test Message".to_string(),
            NotificationLevel::Info,
            "test".to_string(),
        );
        
        manager.add_notification(notification).unwrap();
        
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
    
    #[tokio::test]
    async fn test_notification_manager_maintenance() {
        let temp_dir = TempDir::new().unwrap();
        let event_publisher = Arc::new(EventPublisher::new());
        let manager = NotificationManager::new(temp_dir.path().to_path_buf(), event_publisher);
        
        // Add some notifications
        for i in 0..5 {
            let notification = Notification::new(
                format!("Test {}", i),
                "Test".to_string(),
                NotificationLevel::Info,
                "test".to_string(),
            );
            manager.add_notification(notification).unwrap();
        }
        
        assert_eq!(manager.notification_count(), 5);
        
        // Run maintenance
        manager.maintenance();
        
        // Should not crash and notifications should still be there (no old ones to clean up)
        assert_eq!(manager.notification_count(), 5);
    }
    
    #[tokio::test]
    async fn test_notification_manager_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let event_publisher = Arc::new(EventPublisher::new());
        let manager = NotificationManager::new(temp_dir.path().to_path_buf(), event_publisher);
        
        // Save and load should not fail
        manager.save().await.unwrap();
        manager.load().await.unwrap();
    }
}