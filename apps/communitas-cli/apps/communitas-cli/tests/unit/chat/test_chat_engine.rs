// Copyright 2025 Saorsa Labs Limited
// Tests for AI chat engine

use anyhow::Result;
use communitas_cli::chat::{ChatEngine, AIModel, Message, Role, ChatOptions, SessionId};
use futures::Stream;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use std::pin::Pin;
use std::future::Future;
use tokio_stream::StreamExt;

// Mock AI Model for testing
struct MockAIModel {
    responses: Vec<String>,
    call_count: AtomicUsize,
    delay: Duration,
    should_fail: bool,
}

impl MockAIModel {
    fn new() -> Self {
        Self {
            responses: vec!["Mock response".to_string()],
            call_count: AtomicUsize::new(0),
            delay: Duration::from_millis(10),
            should_fail: false,
        }
    }
    
    fn with_responses(responses: Vec<String>) -> Self {
        Self {
            responses,
            call_count: AtomicUsize::new(0),
            delay: Duration::from_millis(10),
            should_fail: false,
        }
    }
    
    fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
    
    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }
    
    fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

impl AIModel for MockAIModel {
    fn chat(&self, messages: &[Message], _options: &ChatOptions) -> Pin<Box<dyn Future<Output = Result<String>> + Send + '_>> {
        Box::pin(async move {
            tokio::time::sleep(self.delay).await;
            
            if self.should_fail {
                return Err(anyhow::anyhow!("Mock AI failure"));
            }
            
            let index = self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(self.responses[index % self.responses.len()].clone())
        })
    }
    
    fn chat_stream(
        &self,
        messages: &[Message],
        options: &ChatOptions,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn Stream<Item = String> + Send + Unpin>>> + Send + '_>> {
        Box::pin(async move {
            let response = self.chat(messages, options).await?;
            let chars: Vec<String> = response.chars().map(|c| c.to_string()).collect();
            Ok(Box::new(tokio_stream::iter(chars)) as Box<dyn Stream<Item = String> + Send + Unpin>)
        })
    }
    
    fn supports_vision(&self) -> bool {
        false
    }
    
    fn supports_function_calling(&self) -> bool {
        false
    }
    
    fn model_name(&self) -> &str {
        "mock-model"
    }
}

#[tokio::test]
async fn test_chat_engine_creation() {
    let engine = ChatEngine::new();
    assert_eq!(engine.model_count(), 0);
    assert_eq!(engine.active_sessions(), 0);
}

#[tokio::test]
async fn test_add_ai_model() {
    let mut engine = ChatEngine::new();
    let mock_model = MockAIModel::new();
    
    engine.add_model("test-model", Box::new(mock_model));
    assert_eq!(engine.model_count(), 1);
    assert!(engine.has_model("test-model"));
    assert!(!engine.has_model("non-existent"));
}

#[tokio::test]
async fn test_create_chat_session() -> Result<()> {
    let mut engine = ChatEngine::new();
    let session_id = engine.create_session("test-user").await?;
    
    assert_eq!(engine.active_sessions(), 1);
    assert!(engine.has_session(&session_id));
    
    let session_info = engine.get_session_info(&session_id).await?;
    assert_eq!(session_info.user_id, "test-user");
    assert_eq!(session_info.message_count, 0);
    
    Ok(())
}

#[tokio::test]
async fn test_basic_chat_conversation() -> Result<()> {
    let mut engine = ChatEngine::new();
    let mock_model = MockAIModel::with_responses(vec![
        "Hello! How can I help you?".to_string(),
        "I'm doing well, thank you!".to_string(),
    ]);
    
    engine.add_model("test-model", Box::new(mock_model));
    let session_id = engine.create_session("test-user").await?;
    
    // First message
    let response1 = engine.chat(&session_id, "test-model", "Hello").await?;
    assert_eq!(response1, "Hello! How can I help you?");
    
    // Second message
    let response2 = engine.chat(&session_id, "test-model", "How are you?").await?;
    assert_eq!(response2, "I'm doing well, thank you!");
    
    // Verify session history
    let session_info = engine.get_session_info(&session_id).await?;
    assert_eq!(session_info.message_count, 4); // 2 user + 2 assistant messages
    
    Ok(())
}

#[tokio::test]
async fn test_chat_streaming() -> Result<()> {
    let mut engine = ChatEngine::new();
    let mock_model = MockAIModel::with_responses(vec!["Hello".to_string()]);
    
    engine.add_model("test-model", Box::new(mock_model));
    let session_id = engine.create_session("test-user").await?;
    
    let mut stream = engine.chat_stream(&session_id, "test-model", "Hi").await?;
    let mut response_parts = Vec::new();
    
    while let Some(chunk) = stream.next().await {
        response_parts.push(chunk);
    }
    
    let full_response = response_parts.join("");
    assert_eq!(full_response, "Hello");
    
    Ok(())
}

#[tokio::test]
async fn test_context_management() -> Result<()> {
    let mut engine = ChatEngine::new();
    let mock_model = MockAIModel::new();
    
    engine.add_model("test-model", Box::new(mock_model));
    let session_id = engine.create_session("test-user").await?;
    
    // Add messages manually
    engine.add_message(&session_id, Message::user("What's 2+2?")).await?;
    engine.add_message(&session_id, Message::assistant("2+2 equals 4.")).await?;
    engine.add_message(&session_id, Message::user("What about 3+3?")).await?;
    engine.add_message(&session_id, Message::assistant("3+3 equals 6.")).await?;
    
    let context = engine.get_context(&session_id).await?;
    assert_eq!(context.messages.len(), 4);
    
    // Check message order and roles
    assert_eq!(context.messages[0].role, Role::User);
    assert_eq!(context.messages[0].content, "What's 2+2?");
    assert_eq!(context.messages[1].role, Role::Assistant);
    assert_eq!(context.messages[1].content, "2+2 equals 4.");
    assert_eq!(context.messages[2].role, Role::User);
    assert_eq!(context.messages[2].content, "What about 3+3?");
    assert_eq!(context.messages[3].role, Role::Assistant);
    assert_eq!(context.messages[3].content, "3+3 equals 6.");
    
    Ok(())
}

#[tokio::test]
async fn test_token_limit_management() -> Result<()> {
    let mut engine = ChatEngine::with_token_limit(100);
    let mock_model = MockAIModel::new();
    
    engine.add_model("test-model", Box::new(mock_model));
    let session_id = engine.create_session("test-user").await?;
    
    // Add messages that would exceed token limit
    for i in 0..20 {
        engine.add_message(&session_id, Message::user(&"x".repeat(10))).await?;
        engine.add_message(&session_id, Message::assistant(&"y".repeat(10))).await?;
    }
    
    let context = engine.get_context(&session_id).await?;
    let estimated_tokens = context.estimate_tokens();
    
    // Should stay within token limit
    assert!(estimated_tokens <= 100);
    
    // Should have removed oldest messages
    assert!(context.messages.len() < 40);
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_sessions() -> Result<()> {
    let mut engine = ChatEngine::new();
    let mock_model = MockAIModel::with_responses(vec![
        "Response 1".to_string(),
        "Response 2".to_string(),
        "Response 3".to_string(),
    ]);
    
    engine.add_model("test-model", Box::new(mock_model));
    
    // Create multiple sessions
    let session1 = engine.create_session("user1").await?;
    let session2 = engine.create_session("user2").await?;
    let session3 = engine.create_session("user3").await?;
    
    assert_eq!(engine.active_sessions(), 3);
    
    // Send messages concurrently
    let task1 = engine.chat(&session1, "test-model", "Hello 1");
    let task2 = engine.chat(&session2, "test-model", "Hello 2");
    let task3 = engine.chat(&session3, "test-model", "Hello 3");
    
    let (response1, response2, response3) = tokio::join!(task1, task2, task3);
    
    assert!(response1.is_ok());
    assert!(response2.is_ok());
    assert!(response3.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_session_cleanup() -> Result<()> {
    let mut engine = ChatEngine::with_session_timeout(Duration::from_millis(100));
    let mock_model = MockAIModel::new();
    
    engine.add_model("test-model", Box::new(mock_model));
    let session_id = engine.create_session("test-user").await?;
    
    assert_eq!(engine.active_sessions(), 1);
    
    // Wait for session timeout
    tokio::time::sleep(Duration::from_millis(150)).await;
    
    // Trigger cleanup
    engine.cleanup_expired_sessions().await?;
    
    assert_eq!(engine.active_sessions(), 0);
    assert!(!engine.has_session(&session_id));
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let mut engine = ChatEngine::new();
    let failing_model = MockAIModel::new().with_failure();
    
    engine.add_model("failing-model", Box::new(failing_model));
    let session_id = engine.create_session("test-user").await?;
    
    // Should handle model failures gracefully
    let result = engine.chat(&session_id, "failing-model", "Hello").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Mock AI failure"));
    
    Ok(())
}

#[tokio::test]
async fn test_model_not_found() -> Result<()> {
    let mut engine = ChatEngine::new();
    let session_id = engine.create_session("test-user").await?;
    
    // Should return error for non-existent model
    let result = engine.chat(&session_id, "non-existent-model", "Hello").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Model not found"));
    
    Ok(())
}

#[tokio::test]
async fn test_session_persistence() -> Result<()> {
    let mut engine = ChatEngine::new();
    let mock_model = MockAIModel::with_responses(vec!["Persisted response".to_string()]);
    
    engine.add_model("test-model", Box::new(mock_model));
    let session_id = engine.create_session("test-user").await?;
    
    // Add some conversation
    engine.chat(&session_id, "test-model", "Remember this").await?;
    
    // Save session
    let session_data = engine.export_session(&session_id).await?;
    
    // Create new engine and import session
    let mut new_engine = ChatEngine::new();
    new_engine.add_model("test-model", Box::new(MockAIModel::new()));
    let imported_session_id = new_engine.import_session(session_data).await?;
    
    // Verify session was restored
    let context = new_engine.get_context(&imported_session_id).await?;
    assert_eq!(context.messages.len(), 2); // User message + assistant response
    assert!(context.messages.iter().any(|m| m.content.contains("Remember this")));
    
    Ok(())
}

#[tokio::test]
async fn test_chat_options() -> Result<()> {
    let mut engine = ChatEngine::new();
    let mock_model = MockAIModel::new();
    
    engine.add_model("test-model", Box::new(mock_model));
    let session_id = engine.create_session("test-user").await?;
    
    let options = ChatOptions {
        temperature: Some(0.8),
        max_tokens: Some(150),
        stream: true,
        system_prompt: Some("You are a helpful assistant.".to_string()),
    };
    
    // Should pass options to model (we can't directly test this with mock, 
    // but the API should accept them)
    let result = engine.chat_with_options(&session_id, "test-model", "Hello", options).await;
    assert!(result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_message_validation() -> Result<()> {
    let mut engine = ChatEngine::new();
    let session_id = engine.create_session("test-user").await?;
    
    // Test empty message
    let result = engine.add_message(&session_id, Message::user("")).await;
    assert!(result.is_err());
    
    // Test message too long
    let long_message = "x".repeat(1_000_000);
    let result = engine.add_message(&session_id, Message::user(&long_message)).await;
    assert!(result.is_err());
    
    // Test valid message
    let result = engine.add_message(&session_id, Message::user("Valid message")).await;
    assert!(result.is_ok());
    
    Ok(())
}

// This test will fail initially because the ChatEngine doesn't exist yet
#[test]
#[should_panic(expected = "ChatEngine not implemented")]
fn test_chat_engine_not_implemented() {
    let _engine = ChatEngine::new();
}