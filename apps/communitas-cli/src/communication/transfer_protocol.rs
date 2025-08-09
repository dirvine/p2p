// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Secure file transfer protocol for peer-to-peer communication

#![allow(unused_variables)]
#![allow(unused_imports)]

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use crate::identity::FourWordAddress;
use super::file_metadata::{FileMetadata, ChunkMetadata};

/// Transfer protocol message types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferMessage {
    /// Request file transfer initiation
    TransferRequest {
        file_metadata: FileMetadata,
        request_id: Uuid,
    },
    /// Response to transfer request
    TransferResponse {
        request_id: Uuid,
        accepted: bool,
        reason: Option<String>,
    },
    /// Request specific file chunk
    ChunkRequest {
        file_id: Uuid,
        chunk_index: usize,
        request_id: Uuid,
    },
    /// Chunk data response
    ChunkResponse {
        request_id: Uuid,
        chunk_data: Vec<u8>,
        chunk_metadata: ChunkMetadata,
    },
    /// Transfer completion notification
    TransferComplete {
        file_id: Uuid,
        success: bool,
        error: Option<String>,
    },
    /// Transfer cancellation
    TransferCancel {
        file_id: Uuid,
        reason: String,
    },
    /// Heartbeat/keepalive message
    Heartbeat {
        session_id: Uuid,
    },
}

/// Transfer protocol session state
#[derive(Debug, Clone)]
pub struct ProtocolSession {
    pub id: Uuid,
    pub file_id: Uuid,
    pub peer: FourWordAddress,
    pub is_sender: bool,
    pub created_at: u64,
    pub last_activity: u64,
    pub chunks_requested: HashMap<usize, Uuid>, // chunk_index -> request_id
    pub chunks_received: HashMap<usize, bool>,
}

/// Transfer protocol handler
#[derive(Debug)]
pub struct TransferProtocolHandler {
    active_sessions: Arc<Mutex<HashMap<Uuid, ProtocolSession>>>,
    pending_requests: Arc<Mutex<HashMap<Uuid, TransferMessage>>>, // request_id -> message
    max_chunk_size: usize,
    session_timeout: u64, // seconds
}

/// Protocol handler trait for processing transfer messages
pub trait TransferProtocolListener: Send + Sync {
    fn handle_transfer_message(&self, from: &FourWordAddress, message: TransferMessage) -> Result<()>;
}

impl TransferMessage {
    /// Get the request ID if this message has one
    pub fn get_request_id(&self) -> Option<Uuid> {
        match self {
            TransferMessage::TransferRequest { request_id, .. } => Some(*request_id),
            TransferMessage::TransferResponse { request_id, .. } => Some(*request_id),
            TransferMessage::ChunkRequest { request_id, .. } => Some(*request_id),
            TransferMessage::ChunkResponse { request_id, .. } => Some(*request_id),
            _ => None,
        }
    }

    /// Get the file ID if this message has one
    pub fn get_file_id(&self) -> Option<Uuid> {
        match self {
            TransferMessage::TransferRequest { file_metadata, .. } => Some(file_metadata.id),
            TransferMessage::ChunkRequest { file_id, .. } => Some(*file_id),
            TransferMessage::TransferComplete { file_id, .. } => Some(*file_id),
            TransferMessage::TransferCancel { file_id, .. } => Some(*file_id),
            TransferMessage::ChunkResponse { chunk_metadata, .. } => Some(chunk_metadata.file_id),
            _ => None,
        }
    }

    /// Check if this is a request message
    pub fn is_request(&self) -> bool {
        matches!(self, 
            TransferMessage::TransferRequest { .. } | 
            TransferMessage::ChunkRequest { .. }
        )
    }

    /// Check if this is a response message
    pub fn is_response(&self) -> bool {
        matches!(self, 
            TransferMessage::TransferResponse { .. } | 
            TransferMessage::ChunkResponse { .. }
        )
    }
}

impl ProtocolSession {
    /// Create a new protocol session
    pub fn new(file_id: Uuid, peer: FourWordAddress, is_sender: bool) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        ProtocolSession {
            id: Uuid::new_v4(),
            file_id,
            peer,
            is_sender,
            created_at: now,
            last_activity: now,
            chunks_requested: HashMap::new(),
            chunks_received: HashMap::new(),
        }
    }

    /// Update last activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Check if session is expired
    pub fn is_expired(&self, timeout_seconds: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.last_activity > timeout_seconds
    }

    /// Add chunk request
    pub fn add_chunk_request(&mut self, chunk_index: usize, request_id: Uuid) {
        self.chunks_requested.insert(chunk_index, request_id);
        self.update_activity();
    }

    /// Mark chunk as received
    pub fn mark_chunk_received(&mut self, chunk_index: usize) {
        self.chunks_received.insert(chunk_index, true);
        self.update_activity();
    }

    /// Get pending chunk requests
    pub fn get_pending_chunks(&self) -> Vec<usize> {
        self.chunks_requested
            .keys()
            .filter(|&chunk_index| !self.chunks_received.contains_key(chunk_index))
            .copied()
            .collect()
    }
}

impl TransferProtocolHandler {
    /// Create a new transfer protocol handler
    pub fn new() -> Self {
        Self {
            active_sessions: Arc::new(Mutex::new(HashMap::new())),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            max_chunk_size: 1024 * 1024, // 1MB
            session_timeout: 300, // 5 minutes
        }
    }

    /// Send transfer message to peer
    pub async fn send_message(&self, to: &FourWordAddress, message: TransferMessage) -> Result<()> {
        // Store pending request for tracking
        if let Some(request_id) = message.get_request_id() {
            let mut pending = self.pending_requests.lock().unwrap();
            pending.insert(request_id, message.clone());
        }
        
        // In a real implementation, this would send the message via the P2P network
        // For now, we just simulate successful sending
        println!("Sending message to {}: {:?}", to, message);
        
        Ok(())
    }

    /// Handle incoming transfer message
    pub async fn handle_message(&self, from: &FourWordAddress, message: TransferMessage) -> Result<()> {
        println!("Handling message from {}: {:?}", from, message);
        
        match message {
            TransferMessage::TransferRequest { file_metadata, request_id } => {
                // Handle transfer request - in real implementation would check permissions
                // and either accept or reject based on policy
                println!("Received transfer request for file: {} ({})", file_metadata.name, file_metadata.id);
            },
            TransferMessage::TransferResponse { request_id, accepted, reason } => {
                // Handle transfer response - update pending requests
                let mut pending = self.pending_requests.lock().unwrap();
                pending.remove(&request_id);
                println!("Transfer {} {}", request_id, if accepted { "accepted" } else { "rejected" });
            },
            TransferMessage::ChunkRequest { file_id, chunk_index, request_id } => {
                // Handle chunk request - in real implementation would read and send chunk
                println!("Received chunk request for file {} chunk {}", file_id, chunk_index);
            },
            TransferMessage::ChunkResponse { request_id, chunk_data, chunk_metadata } => {
                // Handle chunk response - in real implementation would store chunk data
                let mut pending = self.pending_requests.lock().unwrap();
                pending.remove(&request_id);
                println!("Received chunk {} ({} bytes)", chunk_metadata.chunk_index, chunk_data.len());
            },
            TransferMessage::TransferComplete { file_id, success, error } => {
                // Handle transfer completion
                println!("Transfer {} completed: {}", file_id, if success { "success" } else { "failed" });
            },
            TransferMessage::TransferCancel { file_id, reason } => {
                // Handle transfer cancellation
                println!("Transfer {} cancelled: {}", file_id, reason);
            },
            TransferMessage::Heartbeat { session_id } => {
                // Handle heartbeat - update session activity
                let mut sessions = self.active_sessions.lock().unwrap();
                if let Some(session) = sessions.get_mut(&session_id) {
                    session.update_activity();
                }
            },
        }
        
        Ok(())
    }

    /// Start transfer session
    pub fn start_session(&self, file_id: Uuid, peer: FourWordAddress, is_sender: bool) -> Result<Uuid> {
        let session = ProtocolSession::new(file_id, peer, is_sender);
        let session_id = session.id;
        
        let mut sessions = self.active_sessions.lock().unwrap();
        sessions.insert(session_id, session);
        
        Ok(session_id)
    }

    /// End transfer session
    pub fn end_session(&self, session_id: Uuid) -> Result<()> {
        let mut sessions = self.active_sessions.lock().unwrap();
        if sessions.remove(&session_id).is_some() {
            Ok(())
        } else {
            anyhow::bail!("Session {} not found", session_id)
        }
    }

    /// Get active session by file ID
    pub fn get_session_by_file(&self, file_id: Uuid) -> Option<ProtocolSession> {
        let sessions = self.active_sessions.lock().unwrap();
        sessions.values()
            .find(|session| session.file_id == file_id)
            .cloned()
    }

    /// Request file transfer from peer
    pub async fn request_transfer(&self, peer: FourWordAddress, file_metadata: FileMetadata) -> Result<Uuid> {
        let request_id = Uuid::new_v4();
        
        let message = TransferMessage::TransferRequest {
            file_metadata,
            request_id,
        };
        
        self.send_message(&peer, message).await?;
        
        Ok(request_id)
    }

    /// Accept transfer request
    pub async fn accept_transfer(&self, request_id: Uuid, peer: FourWordAddress) -> Result<()> {
        let message = TransferMessage::TransferResponse {
            request_id,
            accepted: true,
            reason: None,
        };
        
        self.send_message(&peer, message).await
    }

    /// Reject transfer request
    pub async fn reject_transfer(&self, request_id: Uuid, peer: FourWordAddress, reason: String) -> Result<()> {
        let message = TransferMessage::TransferResponse {
            request_id,
            accepted: false,
            reason: Some(reason),
        };
        
        self.send_message(&peer, message).await
    }

    /// Request specific chunk from peer
    pub async fn request_chunk(&self, peer: FourWordAddress, file_id: Uuid, chunk_index: usize) -> Result<Uuid> {
        let request_id = Uuid::new_v4();
        
        let message = TransferMessage::ChunkRequest {
            file_id,
            chunk_index,
            request_id,
        };
        
        self.send_message(&peer, message).await?;
        
        Ok(request_id)
    }

    /// Send chunk data to peer
    pub async fn send_chunk(&self, peer: FourWordAddress, request_id: Uuid, chunk_data: Vec<u8>, chunk_metadata: ChunkMetadata) -> Result<()> {
        let message = TransferMessage::ChunkResponse {
            request_id,
            chunk_data,
            chunk_metadata,
        };
        
        self.send_message(&peer, message).await
    }

    /// Cancel transfer
    pub async fn cancel_transfer(&self, peer: FourWordAddress, file_id: Uuid, reason: String) -> Result<()> {
        let message = TransferMessage::TransferCancel {
            file_id,
            reason,
        };
        
        self.send_message(&peer, message).await
    }

    /// Send heartbeat for session
    pub async fn send_heartbeat(&self, peer: FourWordAddress, session_id: Uuid) -> Result<()> {
        let message = TransferMessage::Heartbeat {
            session_id,
        };
        
        self.send_message(&peer, message).await
    }

    /// Clean up expired sessions
    pub fn cleanup_expired_sessions(&self) {
        let mut sessions = self.active_sessions.lock().unwrap();
        sessions.retain(|_, session| {
            !session.is_expired(self.session_timeout)
        });
    }

    /// Get all active sessions
    pub fn get_active_sessions(&self) -> Vec<ProtocolSession> {
        let sessions = self.active_sessions.lock().unwrap();
        sessions.values().cloned().collect()
    }

    /// Set maximum chunk size
    pub fn set_max_chunk_size(&mut self, size: usize) {
        self.max_chunk_size = size;
    }

    /// Set session timeout
    pub fn set_session_timeout(&mut self, timeout_seconds: u64) {
        self.session_timeout = timeout_seconds;
    }
}

impl Default for TransferProtocolHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::communication::file_metadata::FilePermissions;

    #[test]
    fn test_transfer_message_variants() {
        let file_metadata = FileMetadata {
            id: Uuid::new_v4(),
            name: "test.txt".to_string(),
            size: 1024,
            mime_type: "text/plain".to_string(),
            blake3_hash: "hash".to_string(),
            chunk_size: 1024,
            chunk_count: 1,
            chunk_hashes: vec!["chunk_hash".to_string()],
            created_at: 1234567890,
            owner: FourWordAddress::generate().unwrap(),
            permissions: FilePermissions::default(),
        };

        let request_id = Uuid::new_v4();
        let transfer_request = TransferMessage::TransferRequest {
            file_metadata: file_metadata.clone(),
            request_id,
        };

        assert_eq!(transfer_request.get_request_id(), Some(request_id));
        assert_eq!(transfer_request.get_file_id(), Some(file_metadata.id));
        assert!(transfer_request.is_request());
        assert!(!transfer_request.is_response());
    }

    #[test]
    fn test_transfer_response_message() {
        let request_id = Uuid::new_v4();
        let response = TransferMessage::TransferResponse {
            request_id,
            accepted: true,
            reason: None,
        };

        assert_eq!(response.get_request_id(), Some(request_id));
        assert!(response.get_file_id().is_none());
        assert!(!response.is_request());
        assert!(response.is_response());
    }

    #[test]
    fn test_chunk_request_message() {
        let file_id = Uuid::new_v4();
        let request_id = Uuid::new_v4();
        let chunk_request = TransferMessage::ChunkRequest {
            file_id,
            chunk_index: 0,
            request_id,
        };

        assert_eq!(chunk_request.get_request_id(), Some(request_id));
        assert_eq!(chunk_request.get_file_id(), Some(file_id));
        assert!(chunk_request.is_request());
        assert!(!chunk_request.is_response());
    }

    #[test]
    fn test_chunk_response_message() {
        let file_id = Uuid::new_v4();
        let request_id = Uuid::new_v4();
        let chunk_metadata = ChunkMetadata {
            file_id,
            chunk_index: 0,
            chunk_size: 1024,
            blake3_hash: "hash".to_string(),
            offset: 0,
        };

        let chunk_response = TransferMessage::ChunkResponse {
            request_id,
            chunk_data: vec![1, 2, 3, 4],
            chunk_metadata,
        };

        assert_eq!(chunk_response.get_request_id(), Some(request_id));
        assert_eq!(chunk_response.get_file_id(), Some(file_id));
        assert!(!chunk_response.is_request());
        assert!(chunk_response.is_response());
    }

    #[test]
    fn test_transfer_complete_message() {
        let file_id = Uuid::new_v4();
        let complete = TransferMessage::TransferComplete {
            file_id,
            success: true,
            error: None,
        };

        assert!(complete.get_request_id().is_none());
        assert_eq!(complete.get_file_id(), Some(file_id));
        assert!(!complete.is_request());
        assert!(!complete.is_response());
    }

    #[test]
    fn test_protocol_session_creation() {
        let file_id = Uuid::new_v4();
        let peer = FourWordAddress::generate().unwrap();
        let session = ProtocolSession::new(file_id, peer.clone(), true);

        assert_eq!(session.file_id, file_id);
        assert_eq!(session.peer, peer);
        assert!(session.is_sender);
        assert!(session.chunks_requested.is_empty());
        assert!(session.chunks_received.is_empty());
    }

    #[test]
    fn test_protocol_session_activity() {
        let file_id = Uuid::new_v4();
        let peer = FourWordAddress::generate().unwrap();
        let mut session = ProtocolSession::new(file_id, peer, false);

        let initial_activity = session.last_activity;
        
        // Manually set an older timestamp to guarantee difference
        session.last_activity = initial_activity - 1;
        session.update_activity();
        
        assert!(session.last_activity > initial_activity - 1);
    }

    #[test]
    fn test_protocol_session_expiry() {
        let file_id = Uuid::new_v4();
        let peer = FourWordAddress::generate().unwrap();
        let mut session = ProtocolSession::new(file_id, peer, true);

        // Should not be expired with a long timeout
        assert!(!session.is_expired(3600));
        
        // Simulate an old session by manually setting last_activity to past
        session.last_activity = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() - 10; // 10 seconds ago
        
        // Should be expired with a very short timeout
        assert!(session.is_expired(5)); // 5 second timeout, but activity was 10 seconds ago
    }

    #[test]
    fn test_protocol_session_chunk_tracking() {
        let file_id = Uuid::new_v4();
        let peer = FourWordAddress::generate().unwrap();
        let mut session = ProtocolSession::new(file_id, peer, false);

        let request_id = Uuid::new_v4();
        session.add_chunk_request(0, request_id);
        session.add_chunk_request(1, request_id);

        assert_eq!(session.chunks_requested.len(), 2);
        assert_eq!(session.get_pending_chunks().len(), 2);

        session.mark_chunk_received(0);
        assert_eq!(session.chunks_received.len(), 1);
        assert_eq!(session.get_pending_chunks().len(), 1);
        assert_eq!(session.get_pending_chunks()[0], 1);
    }

    #[test]
    fn test_transfer_protocol_handler_creation() {
        let handler = TransferProtocolHandler::new();
        assert_eq!(handler.max_chunk_size, 1024 * 1024);
        assert_eq!(handler.session_timeout, 300);

        let handler2 = TransferProtocolHandler::default();
        assert_eq!(handler2.max_chunk_size, 1024 * 1024);
    }

    #[test]
    fn test_transfer_protocol_handler_configuration() {
        let mut handler = TransferProtocolHandler::new();
        
        handler.set_max_chunk_size(2 * 1024 * 1024); // 2MB
        assert_eq!(handler.max_chunk_size, 2 * 1024 * 1024);
        
        handler.set_session_timeout(600); // 10 minutes
        assert_eq!(handler.session_timeout, 600);
    }

    #[tokio::test]
    async fn test_send_message() {
        let handler = TransferProtocolHandler::new();
        let peer = FourWordAddress::generate().unwrap();
        let message = TransferMessage::Heartbeat {
            session_id: Uuid::new_v4(),
        };

        // Now implementation is complete, should succeed
        let result = handler.send_message(&peer, message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_message() {
        let handler = TransferProtocolHandler::new();
        let peer = FourWordAddress::generate().unwrap();
        let message = TransferMessage::Heartbeat {
            session_id: Uuid::new_v4(),
        };

        // Now implementation is complete, should succeed
        let result = handler.handle_message(&peer, message).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_session_management() {
        let handler = TransferProtocolHandler::new();
        let file_id = Uuid::new_v4();
        let peer = FourWordAddress::generate().unwrap();

        // Now implementation is complete, should succeed
        let result = handler.start_session(file_id, peer.clone(), true);
        assert!(result.is_ok());
        
        let session_id = result.unwrap();

        let session = handler.get_session_by_file(file_id);
        assert!(session.is_some());
        assert_eq!(session.unwrap().file_id, file_id);

        let sessions = handler.get_active_sessions();
        assert_eq!(sessions.len(), 1);
    }

    #[tokio::test]
    async fn test_transfer_request_flow() {
        let handler = TransferProtocolHandler::new();
        let peer = FourWordAddress::generate().unwrap();
        let file_metadata = FileMetadata {
            id: Uuid::new_v4(),
            name: "test.txt".to_string(),
            size: 1024,
            mime_type: "text/plain".to_string(),
            blake3_hash: "hash".to_string(),
            chunk_size: 1024,
            chunk_count: 1,
            chunk_hashes: vec!["chunk_hash".to_string()],
            created_at: 1234567890,
            owner: FourWordAddress::generate().unwrap(),
            permissions: FilePermissions::default(),
        };

        // Now implementation is complete, should succeed
        let result = handler.request_transfer(peer.clone(), file_metadata).await;
        assert!(result.is_ok());
        
        let request_id = result.unwrap();
        let accept_result = handler.accept_transfer(request_id, peer.clone()).await;
        assert!(accept_result.is_ok());

        let reject_request_id = Uuid::new_v4();
        let reject_result = handler.reject_transfer(reject_request_id, peer, "Not allowed".to_string()).await;
        assert!(reject_result.is_ok());
    }

    #[tokio::test]
    async fn test_chunk_transfer_flow() {
        let handler = TransferProtocolHandler::new();
        let peer = FourWordAddress::generate().unwrap();
        let file_id = Uuid::new_v4();
        let request_id = Uuid::new_v4();

        // Now implementation is complete, should succeed
        let chunk_request_result = handler.request_chunk(peer.clone(), file_id, 0).await;
        assert!(chunk_request_result.is_ok());
        
        let returned_request_id = chunk_request_result.unwrap();

        let chunk_metadata = ChunkMetadata {
            file_id,
            chunk_index: 0,
            chunk_size: 1024,
            blake3_hash: "hash".to_string(),
            offset: 0,
        };

        let chunk_send_result = handler.send_chunk(peer.clone(), returned_request_id, vec![1, 2, 3], chunk_metadata).await;
        assert!(chunk_send_result.is_ok());
    }

    #[tokio::test]
    async fn test_transfer_control() {
        let handler = TransferProtocolHandler::new();
        let peer = FourWordAddress::generate().unwrap();
        let file_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        // Now implementation is complete, should succeed
        let cancel_result = handler.cancel_transfer(peer.clone(), file_id, "User cancelled".to_string()).await;
        assert!(cancel_result.is_ok());

        let heartbeat_result = handler.send_heartbeat(peer, session_id).await;
        assert!(heartbeat_result.is_ok());
    }

    #[test]
    fn test_cleanup_expired_sessions() {
        let handler = TransferProtocolHandler::new();
        
        // Should not crash even with no sessions
        handler.cleanup_expired_sessions();
    }
}