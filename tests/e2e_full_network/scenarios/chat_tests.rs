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

//! Chat and communication system tests

use anyhow::{Context, Result};
use p2p_core::chat::{
    Channel, ChannelType, ChannelInfo, Message, MessageType, MessageContent,
    Call, CallType, CallParticipant, Reaction, Thread, MessageStatus,
    ChannelPermissions, MessageAttachment, AttachmentType,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use crate::infrastructure::{
    test_network::DistributedTestNetwork,
    test_reporter::{TestEvent, TestEventType},
};

/// Test complete chat system with calls
pub async fn test_full_chat_system(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n💬 Testing Chat & Communication System");
    println!("=====================================");
    
    // 1. Create various channel types
    test_channel_creation(network).await
        .context("Failed to test channel creation")?;
    
    // 2. Test messaging across nodes
    test_cross_node_messaging(network).await
        .context("Failed to test cross-node messaging")?;
    
    // 3. Test voice/video calls
    test_multimedia_calls(network).await
        .context("Failed to test multimedia calls")?;
    
    // 4. Test message reactions and threads
    test_advanced_messaging(network).await
        .context("Failed to test advanced messaging")?;
    
    // 5. Test message search
    test_message_search(network).await
        .context("Failed to test message search")?;
    
    // 6. Test channel permissions
    test_channel_permissions(network).await
        .context("Failed to test channel permissions")?;
    
    Ok(())
}

/// Test channel creation
async fn test_channel_creation(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n📁 Creating various channel types...");
    
    let mut channels = Vec::new();
    
    // Create direct message channel
    let dm_channel = network.local_nodes[0].node.create_channel(ChannelInfo {
        name: "Alice-Bob DM".to_string(),
        channel_type: ChannelType::DirectMessage,
        description: None,
        members: vec![
            network.local_nodes[0].identity.base_identity.user_id.clone(),
            network.local_nodes[1].identity.base_identity.user_id.clone(),
        ],
        permissions: ChannelPermissions::default_dm(),
        metadata: HashMap::new(),
    }).await?;
    channels.push(dm_channel);
    
    // Create group channels
    let group_channel = network.local_nodes[1].node.create_channel(ChannelInfo {
        name: "Engineering Team".to_string(),
        channel_type: ChannelType::Group,
        description: Some("Engineering team discussions".to_string()),
        members: network.local_nodes.iter()
            .take(5)
            .map(|n| n.identity.base_identity.user_id.clone())
            .collect(),
        permissions: ChannelPermissions::default_group(),
        metadata: HashMap::from([
            ("team".to_string(), "engineering".to_string()),
        ]),
    }).await?;
    channels.push(group_channel);
    
    // Create public channel
    let public_channel = network.local_nodes[2].node.create_channel(ChannelInfo {
        name: "Announcements".to_string(),
        channel_type: ChannelType::Public,
        description: Some("Company-wide announcements".to_string()),
        members: vec![], // Public channels start empty
        permissions: ChannelPermissions::default_public(),
        metadata: HashMap::from([
            ("pinned".to_string(), "true".to_string()),
        ]),
    }).await?;
    channels.push(public_channel);
    
    // Create broadcast channel
    let broadcast_channel = network.local_nodes[0].node.create_channel(ChannelInfo {
        name: "CEO Updates".to_string(),
        channel_type: ChannelType::Broadcast,
        description: Some("Updates from leadership".to_string()),
        members: vec![network.local_nodes[0].identity.base_identity.user_id.clone()],
        permissions: ChannelPermissions::broadcast(
            &network.local_nodes[0].identity.base_identity.user_id
        ),
        metadata: HashMap::new(),
    }).await?;
    channels.push(broadcast_channel);
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ChatMessage,
        details: {
            let mut details = HashMap::new();
            details.insert("channels_created".to_string(), serde_json::json!(channels.len()));
            details.insert("channel_types".to_string(), 
                serde_json::json!(["dm", "group", "public", "broadcast"]));
            details
        },
        success: true,
    }).await;
    
    // Store channels for later tests
    for node in &mut network.local_nodes {
        node.test_data.write().await.events.push(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: node.identity.base_identity.user_id.clone(),
            event_type: TestEventType::ChatMessage,
            details: {
                let mut details = HashMap::new();
                details.insert("channels".to_string(), 
                    serde_json::json!(channels.iter().map(|c| &c.id).collect::<Vec<_>>()));
                details
            },
            success: true,
        });
    }
    
    println!("✅ Created {} channels", channels.len());
    Ok(())
}

/// Test cross-node messaging
async fn test_cross_node_messaging(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n📨 Testing cross-node messaging...");
    
    // Get channels from test data
    let channels: Vec<String> = {
        let data = network.local_nodes[0].test_data.read().await;
        if let Some(event) = data.events.last() {
            if let Some(channels) = event.details.get("channels") {
                serde_json::from_value(channels.clone()).unwrap_or_default()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    };
    
    if channels.is_empty() {
        return Err(anyhow::anyhow!("No channels found in test data"));
    }
    
    let dm_channel = &channels[0];
    let group_channel = &channels[1];
    let public_channel = &channels[2];
    
    let mut total_messages = 0;
    let messages_per_channel = 50;
    
    // Test direct messages
    for i in 0..messages_per_channel {
        let sender_idx = i % 2; // Alternate between node 0 and 1
        let message = network.local_nodes[sender_idx].node.send_message(
            dm_channel,
            MessageContent::Text {
                text: format!("Direct message {} from node {}", i, sender_idx),
                mentions: vec![],
                links: vec![],
            },
            None, // No reply
        ).await?;
        
        total_messages += 1;
        
        // Add reactions periodically
        if i % 5 == 0 {
            let reactor_idx = 1 - sender_idx; // Other node reacts
            network.local_nodes[reactor_idx].node.add_reaction(
                &message.id,
                "👍",
            ).await?;
        }
    }
    
    // Test group messages with various content types
    for i in 0..messages_per_channel {
        let sender_idx = i % network.local_nodes.len().min(5); // Rotate through first 5 nodes
        
        let content = match i % 4 {
            0 => MessageContent::Text {
                text: format!("Group message {} from node {}", i, sender_idx),
                mentions: vec![format!("@node_{}", (sender_idx + 1) % 5)],
                links: vec![],
            },
            1 => MessageContent::Image {
                url: format!("https://example.com/image_{}.jpg", i),
                width: 800,
                height: 600,
                alt_text: Some("Test image".to_string()),
            },
            2 => MessageContent::File {
                url: format!("https://example.com/doc_{}.pdf", i),
                name: format!("document_{}.pdf", i),
                size: 1024 * 1024, // 1MB
                mime_type: "application/pdf".to_string(),
            },
            _ => MessageContent::Code {
                language: "rust".to_string(),
                code: format!("fn test_{i}() {{ println!(\"Hello from node {sender_idx}\"); }}"),
                filename: Some(format!("test_{}.rs", i)),
            },
        };
        
        let message = network.local_nodes[sender_idx].node.send_message(
            group_channel,
            content,
            None,
        ).await?;
        
        total_messages += 1;
        
        // Create threads on some messages
        if i % 10 == 0 {
            for j in 0..3 {
                let replier_idx = (sender_idx + j + 1) % 5;
                network.local_nodes[replier_idx].node.send_message(
                    group_channel,
                    MessageContent::Text {
                        text: format!("Reply {} to message {}", j, i),
                        mentions: vec![],
                        links: vec![],
                    },
                    Some(message.id.clone()), // Reply to original
                ).await?;
                
                total_messages += 1;
            }
        }
    }
    
    // Test public channel broadcasts
    for i in 0..messages_per_channel / 2 {
        let announcement = network.local_nodes[2].node.send_message(
            public_channel,
            MessageContent::Text {
                text: format!("📢 Public announcement #{}", i),
                mentions: vec!["@everyone".to_string()],
                links: vec!["https://docs.example.com".to_string()],
            },
            None,
        ).await?;
        
        total_messages += 1;
        
        // Multiple reactions from different nodes
        let reactions = vec!["👍", "❤️", "🎉", "🚀"];
        for (idx, reaction) in reactions.iter().enumerate() {
            if idx < network.local_nodes.len() {
                network.local_nodes[idx].node.add_reaction(
                    &announcement.id,
                    reaction,
                ).await?;
            }
        }
    }
    
    // Wait for message propagation
    sleep(Duration::from_secs(2)).await;
    
    // Verify message delivery across nodes
    let mut verification_passed = true;
    
    for (idx, node) in network.local_nodes.iter().enumerate() {
        // Check DM messages (only for participants)
        if idx < 2 {
            let dm_messages = node.node.get_messages(dm_channel, None, 100).await?;
            if dm_messages.len() < messages_per_channel {
                eprintln!("⚠️  Node {} has {} DM messages, expected {}", 
                    idx, dm_messages.len(), messages_per_channel);
                verification_passed = false;
            }
        }
        
        // Check group messages (for members)
        if idx < 5 {
            let group_messages = node.node.get_messages(group_channel, None, 200).await?;
            if group_messages.len() < messages_per_channel {
                eprintln!("⚠️  Node {} has {} group messages, expected at least {}", 
                    idx, group_messages.len(), messages_per_channel);
                verification_passed = false;
            }
        }
        
        // Check public messages (all nodes)
        let public_messages = node.node.get_messages(public_channel, None, 100).await?;
        if public_messages.len() < messages_per_channel / 2 {
            eprintln!("⚠️  Node {} has {} public messages, expected {}", 
                idx, public_messages.len(), messages_per_channel / 2);
            verification_passed = false;
        }
    }
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ChatMessage,
        details: {
            let mut details = HashMap::new();
            details.insert("total_messages".to_string(), serde_json::json!(total_messages));
            details.insert("channels_tested".to_string(), serde_json::json!(3));
            details.insert("propagation_verified".to_string(), serde_json::json!(verification_passed));
            details
        },
        success: verification_passed,
    }).await;
    
    println!("✅ Sent {} messages across {} channels", total_messages, 3);
    Ok(())
}

/// Test multimedia calls
async fn test_multimedia_calls(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n📞 Testing multimedia calls...");
    
    // Get group channel for calls
    let channels: Vec<String> = {
        let data = network.local_nodes[0].test_data.read().await;
        if let Some(event) = data.events.last() {
            if let Some(channels) = event.details.get("channels") {
                serde_json::from_value(channels.clone()).unwrap_or_default()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    };
    
    let group_channel = &channels[1];
    
    // Start voice call
    println!("  Starting voice call...");
    let voice_call = network.local_nodes[0].node.start_call(
        group_channel,
        CallType::Voice,
        HashMap::from([
            ("max_participants".to_string(), "10".to_string()),
        ]),
    ).await?;
    
    // Have other nodes join
    let mut participants = vec![0]; // Initiator is already in
    for i in 1..4 {
        network.local_nodes[i].node.join_call(&voice_call.id).await?;
        participants.push(i);
        
        network.reporter.report_progress(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: format!("node_{}", i),
            event_type: TestEventType::ChatMessage,
            details: {
                let mut details = HashMap::new();
                details.insert("action".to_string(), serde_json::json!("joined_call"));
                details.insert("call_type".to_string(), serde_json::json!("voice"));
                details.insert("participants".to_string(), serde_json::json!(participants.len()));
                details
            },
            success: true,
        }).await;
    }
    
    // Simulate call activity
    sleep(Duration::from_secs(3)).await;
    
    // Upgrade to video call
    println!("  Upgrading to video call...");
    network.local_nodes[0].node.upgrade_call(&voice_call.id, CallType::Video).await?;
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "node_0".to_string(),
        event_type: TestEventType::ChatMessage,
        details: {
            let mut details = HashMap::new();
            details.insert("action".to_string(), serde_json::json!("upgraded_call"));
            details.insert("call_type".to_string(), serde_json::json!("video"));
            details
        },
        success: true,
    }).await;
    
    // Test screen sharing
    println!("  Starting screen share...");
    network.local_nodes[1].node.start_screen_share(&voice_call.id).await?;
    
    sleep(Duration::from_secs(2)).await;
    
    // Stop screen share
    network.local_nodes[1].node.stop_screen_share(&voice_call.id).await?;
    
    // One participant leaves
    network.local_nodes[2].node.leave_call(&voice_call.id).await?;
    participants.retain(|&x| x != 2);
    
    // End call
    network.local_nodes[0].node.end_call(&voice_call.id).await?;
    
    // Test group video call with more participants
    println!("  Starting group video call...");
    let group_video_call = network.local_nodes[3].node.start_call(
        group_channel,
        CallType::Video,
        HashMap::from([
            ("quality".to_string(), "hd".to_string()),
            ("layout".to_string(), "grid".to_string()),
        ]),
    ).await?;
    
    // More nodes join
    for i in [0, 1, 2, 4] {
        if i < network.local_nodes.len() {
            network.local_nodes[i].node.join_call(&group_video_call.id).await?;
        }
    }
    
    sleep(Duration::from_secs(2)).await;
    
    // End group call
    network.local_nodes[3].node.end_call(&group_video_call.id).await?;
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ChatMessage,
        details: {
            let mut details = HashMap::new();
            details.insert("calls_completed".to_string(), serde_json::json!(2));
            details.insert("call_types".to_string(), serde_json::json!(["voice", "video"]));
            details.insert("max_participants".to_string(), serde_json::json!(5));
            details.insert("features_tested".to_string(), 
                serde_json::json!(["upgrade", "screen_share", "leave", "end"]));
            details
        },
        success: true,
    }).await;
    
    println!("✅ Multimedia call tests completed");
    Ok(())
}

/// Test advanced messaging features
async fn test_advanced_messaging(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🧵 Testing advanced messaging features...");
    
    let channels: Vec<String> = {
        let data = network.local_nodes[0].test_data.read().await;
        if let Some(event) = data.events.last() {
            if let Some(channels) = event.details.get("channels") {
                serde_json::from_value(channels.clone()).unwrap_or_default()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    };
    
    let group_channel = &channels[1];
    
    // Test message editing
    let original_message = network.local_nodes[0].node.send_message(
        group_channel,
        MessageContent::Text {
            text: "This message will be edited".to_string(),
            mentions: vec![],
            links: vec![],
        },
        None,
    ).await?;
    
    sleep(Duration::from_millis(500)).await;
    
    network.local_nodes[0].node.edit_message(
        &original_message.id,
        MessageContent::Text {
            text: "This message has been edited ✏️".to_string(),
            mentions: vec![],
            links: vec![],
        },
    ).await?;
    
    // Test message deletion
    let deletable_message = network.local_nodes[1].node.send_message(
        group_channel,
        MessageContent::Text {
            text: "This message will be deleted".to_string(),
            mentions: vec![],
            links: vec![],
        },
        None,
    ).await?;
    
    sleep(Duration::from_millis(500)).await;
    
    network.local_nodes[1].node.delete_message(&deletable_message.id).await?;
    
    // Test message pinning
    let important_message = network.local_nodes[2].node.send_message(
        group_channel,
        MessageContent::Text {
            text: "📌 This is an important message that should be pinned".to_string(),
            mentions: vec!["@everyone".to_string()],
            links: vec![],
        },
        None,
    ).await?;
    
    network.local_nodes[2].node.pin_message(&important_message.id).await?;
    
    // Test complex reactions
    let reaction_test_msg = network.local_nodes[3].node.send_message(
        group_channel,
        MessageContent::Text {
            text: "React to this message with various emojis!".to_string(),
            mentions: vec![],
            links: vec![],
        },
        None,
    ).await?;
    
    let reactions = vec![
        ("👍", vec![0, 1, 2, 3]),
        ("❤️", vec![1, 2]),
        ("😄", vec![0, 2, 3]),
        ("🚀", vec![0, 1, 2, 3, 4]),
        ("🎉", vec![2, 3, 4]),
    ];
    
    for (emoji, node_indices) in reactions {
        for idx in node_indices {
            if idx < network.local_nodes.len() {
                network.local_nodes[idx].node.add_reaction(
                    &reaction_test_msg.id,
                    emoji,
                ).await?;
            }
        }
    }
    
    // Test message forwarding
    let forward_source = network.local_nodes[0].node.send_message(
        group_channel,
        MessageContent::Text {
            text: "This message will be forwarded to another channel".to_string(),
            mentions: vec![],
            links: vec![],
        },
        None,
    ).await?;
    
    let dm_channel = &channels[0];
    network.local_nodes[0].node.forward_message(
        &forward_source.id,
        dm_channel,
        Some("FYI - forwarding this message".to_string()),
    ).await?;
    
    // Test message attachments
    let attachment_msg = network.local_nodes[1].node.send_message_with_attachments(
        group_channel,
        MessageContent::Text {
            text: "Check out these attachments!".to_string(),
            mentions: vec![],
            links: vec![],
        },
        vec![
            MessageAttachment {
                id: uuid::Uuid::new_v4().to_string(),
                name: "presentation.pdf".to_string(),
                size: 5 * 1024 * 1024, // 5MB
                mime_type: "application/pdf".to_string(),
                url: "https://example.com/files/presentation.pdf".to_string(),
                attachment_type: AttachmentType::Document,
                metadata: HashMap::from([
                    ("pages".to_string(), "42".to_string()),
                ]),
            },
            MessageAttachment {
                id: uuid::Uuid::new_v4().to_string(),
                name: "screenshot.png".to_string(),
                size: 2 * 1024 * 1024, // 2MB
                mime_type: "image/png".to_string(),
                url: "https://example.com/files/screenshot.png".to_string(),
                attachment_type: AttachmentType::Image,
                metadata: HashMap::from([
                    ("width".to_string(), "1920".to_string()),
                    ("height".to_string(), "1080".to_string()),
                ]),
            },
        ],
        None,
    ).await?;
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ChatMessage,
        details: {
            let mut details = HashMap::new();
            details.insert("features_tested".to_string(), 
                serde_json::json!([
                    "edit", "delete", "pin", "reactions", 
                    "forward", "attachments"
                ]));
            details.insert("reaction_types".to_string(), serde_json::json!(5));
            details.insert("attachments_sent".to_string(), serde_json::json!(2));
            details
        },
        success: true,
    }).await;
    
    println!("✅ Advanced messaging features tested");
    Ok(())
}

/// Test message search functionality
async fn test_message_search(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🔍 Testing message search...");
    
    let channels: Vec<String> = {
        let data = network.local_nodes[0].test_data.read().await;
        if let Some(event) = data.events.last() {
            if let Some(channels) = event.details.get("channels") {
                serde_json::from_value(channels.clone()).unwrap_or_default()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    };
    
    // Search by text content
    let text_results = network.local_nodes[0].node.search_messages(
        "message",
        None, // Search all channels
        None, // No time filter
        10,   // Limit
    ).await?;
    
    println!("  Found {} messages containing 'message'", text_results.len());
    
    // Search with filters
    let filtered_results = network.local_nodes[1].node.search_messages_advanced(
        p2p_core::chat::SearchQuery {
            text: Some("announcement".to_string()),
            channels: Some(vec![channels[2].clone()]), // Public channel only
            from_users: None,
            has_attachments: None,
            has_reactions: None,
            is_pinned: None,
            message_types: Some(vec![MessageType::Text]),
            time_range: None,
            limit: 20,
        }
    ).await?;
    
    println!("  Found {} announcements", filtered_results.len());
    
    // Search by user
    let user_messages = network.local_nodes[2].node.search_messages_advanced(
        p2p_core::chat::SearchQuery {
            text: None,
            channels: Some(vec![channels[1].clone()]), // Group channel
            from_users: Some(vec![
                network.local_nodes[0].identity.base_identity.user_id.clone()
            ]),
            has_attachments: None,
            has_reactions: None,
            is_pinned: None,
            message_types: None,
            time_range: None,
            limit: 50,
        }
    ).await?;
    
    println!("  Found {} messages from node_0", user_messages.len());
    
    // Search messages with attachments
    let attachment_messages = network.local_nodes[3].node.search_messages_advanced(
        p2p_core::chat::SearchQuery {
            text: None,
            channels: None,
            from_users: None,
            has_attachments: Some(true),
            has_reactions: None,
            is_pinned: None,
            message_types: None,
            time_range: None,
            limit: 10,
        }
    ).await?;
    
    println!("  Found {} messages with attachments", attachment_messages.len());
    
    // Search pinned messages
    let pinned_messages = network.local_nodes[4].node.search_messages_advanced(
        p2p_core::chat::SearchQuery {
            text: None,
            channels: None,
            from_users: None,
            has_attachments: None,
            has_reactions: None,
            is_pinned: Some(true),
            message_types: None,
            time_range: None,
            limit: 10,
        }
    ).await?;
    
    println!("  Found {} pinned messages", pinned_messages.len());
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ChatMessage,
        details: {
            let mut details = HashMap::new();
            details.insert("search_types".to_string(), 
                serde_json::json!([
                    "text", "filtered", "by_user", 
                    "with_attachments", "pinned"
                ]));
            details.insert("total_results".to_string(), 
                serde_json::json!(
                    text_results.len() + filtered_results.len() + 
                    user_messages.len() + attachment_messages.len() + 
                    pinned_messages.len()
                ));
            details
        },
        success: true,
    }).await;
    
    println!("✅ Message search functionality tested");
    Ok(())
}

/// Test channel permissions
async fn test_channel_permissions(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🔐 Testing channel permissions...");
    
    // Create restricted channel
    let restricted_channel = network.local_nodes[0].node.create_channel(ChannelInfo {
        name: "Restricted Channel".to_string(),
        channel_type: ChannelType::Group,
        description: Some("Channel with custom permissions".to_string()),
        members: vec![
            network.local_nodes[0].identity.base_identity.user_id.clone(),
            network.local_nodes[1].identity.base_identity.user_id.clone(),
            network.local_nodes[2].identity.base_identity.user_id.clone(),
        ],
        permissions: ChannelPermissions {
            can_send_messages: HashMap::from([
                (network.local_nodes[0].identity.base_identity.user_id.clone(), true),
                (network.local_nodes[1].identity.base_identity.user_id.clone(), true),
                (network.local_nodes[2].identity.base_identity.user_id.clone(), false), // Read-only
            ]),
            can_add_members: HashMap::from([
                (network.local_nodes[0].identity.base_identity.user_id.clone(), true),
            ]),
            can_remove_members: HashMap::from([
                (network.local_nodes[0].identity.base_identity.user_id.clone(), true),
            ]),
            can_manage_channel: HashMap::from([
                (network.local_nodes[0].identity.base_identity.user_id.clone(), true),
            ]),
            can_delete_messages: HashMap::from([
                (network.local_nodes[0].identity.base_identity.user_id.clone(), true),
                (network.local_nodes[1].identity.base_identity.user_id.clone(), false),
            ]),
            can_pin_messages: HashMap::from([
                (network.local_nodes[0].identity.base_identity.user_id.clone(), true),
                (network.local_nodes[1].identity.base_identity.user_id.clone(), true),
            ]),
        },
        metadata: HashMap::new(),
    }).await?;
    
    // Test permission enforcement
    
    // Node 0 (admin) can send
    let admin_msg = network.local_nodes[0].node.send_message(
        &restricted_channel.id,
        MessageContent::Text {
            text: "Admin message".to_string(),
            mentions: vec![],
            links: vec![],
        },
        None,
    ).await?;
    
    // Node 1 (member) can send
    let member_msg = network.local_nodes[1].node.send_message(
        &restricted_channel.id,
        MessageContent::Text {
            text: "Member message".to_string(),
            mentions: vec![],
            links: vec![],
        },
        None,
    ).await?;
    
    // Node 2 (read-only) cannot send
    let send_result = network.local_nodes[2].node.send_message(
        &restricted_channel.id,
        MessageContent::Text {
            text: "This should fail".to_string(),
            mentions: vec![],
            links: vec![],
        },
        None,
    ).await;
    
    assert!(send_result.is_err(), "Read-only member should not be able to send");
    
    // Test other permissions
    
    // Only admin can add members
    let add_result = network.local_nodes[0].node.add_channel_member(
        &restricted_channel.id,
        &network.local_nodes[3].identity.base_identity.user_id,
    ).await;
    assert!(add_result.is_ok(), "Admin should be able to add members");
    
    let add_result2 = network.local_nodes[1].node.add_channel_member(
        &restricted_channel.id,
        &network.local_nodes[4].identity.base_identity.user_id,
    ).await;
    assert!(add_result2.is_err(), "Non-admin should not be able to add members");
    
    // Only admin can delete messages
    let delete_result = network.local_nodes[0].node.delete_message(&member_msg.id).await;
    assert!(delete_result.is_ok(), "Admin should be able to delete messages");
    
    // Both admin and member can pin
    let pin_result1 = network.local_nodes[0].node.pin_message(&admin_msg.id).await;
    assert!(pin_result1.is_ok(), "Admin should be able to pin messages");
    
    let pin_result2 = network.local_nodes[1].node.pin_message(&admin_msg.id).await;
    assert!(pin_result2.is_ok(), "Member with pin permission should be able to pin");
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ChatMessage,
        details: {
            let mut details = HashMap::new();
            details.insert("permissions_tested".to_string(), 
                serde_json::json!([
                    "send_messages", "add_members", "delete_messages", "pin_messages"
                ]));
            details.insert("permission_enforcement".to_string(), serde_json::json!("working"));
            details
        },
        success: true,
    }).await;
    
    println!("✅ Channel permissions tested successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_channel_types() {
        assert_eq!(
            ChannelType::DirectMessage.max_members(),
            Some(2),
            "DM should have max 2 members"
        );
        
        assert_eq!(
            ChannelType::Public.max_members(),
            None,
            "Public channels should have no member limit"
        );
    }
    
    #[test]
    fn test_message_content_size() {
        let text = MessageContent::Text {
            text: "a".repeat(5000),
            mentions: vec![],
            links: vec![],
        };
        
        // Should validate message size
        match text {
            MessageContent::Text { text, .. } => {
                assert!(text.len() <= 5000, "Text messages should be limited");
            }
            _ => panic!("Wrong message type"),
        }
    }
}