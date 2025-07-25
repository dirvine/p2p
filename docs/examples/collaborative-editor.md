# Example: Real-Time Collaborative Editor

This example demonstrates building a real-time collaborative text editor using the Adaptive P2P Network's pub/sub capabilities.

## Features

- Real-time collaborative editing
- Operational Transformation (OT) for consistency
- User presence and cursors
- Document persistence
- Offline support with sync

## Complete Implementation

```rust
use saorsa_core::adaptive::*;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque};
use tokio::sync::{RwLock, mpsc};
use std::sync::Arc;
use anyhow::Result;
use uuid::Uuid;
use futures::StreamExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Operation {
    Insert { pos: usize, text: String },
    Delete { pos: usize, len: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Edit {
    id: String,
    user_id: String,
    operation: Operation,
    timestamp: u64,
    parent_version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserPresence {
    user_id: String,
    name: String,
    cursor_position: usize,
    selection: Option<(usize, usize)>,
    color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Message {
    Edit(Edit),
    Presence(UserPresence),
    Join { user_id: String, name: String },
    Leave { user_id: String },
    RequestSync { from_version: u64 },
    SyncResponse { edits: Vec<Edit> },
}

struct Document {
    id: String,
    content: String,
    version: u64,
    pending_edits: VecDeque<Edit>,
    acknowledged_edits: HashMap<String, Edit>,
}

impl Document {
    fn new(id: String) -> Self {
        Self {
            id,
            content: String::new(),
            version: 0,
            pending_edits: VecDeque::new(),
            acknowledged_edits: HashMap::new(),
        }
    }
    
    fn apply_operation(&mut self, op: &Operation) -> Result<()> {
        match op {
            Operation::Insert { pos, text } => {
                if *pos <= self.content.len() {
                    self.content.insert_str(*pos, text);
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Invalid insert position"))
                }
            }
            Operation::Delete { pos, len } => {
                if pos + len <= self.content.len() {
                    self.content.drain(*pos..(*pos + len));
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Invalid delete range"))
                }
            }
        }
    }
    
    fn transform_operation(op1: &Operation, op2: &Operation) -> Operation {
        // Simplified OT - in production, use a proper OT library
        match (op1, op2) {
            (Operation::Insert { pos: pos1, text: text1 }, 
             Operation::Insert { pos: pos2, text: _ }) => {
                if pos1 <= pos2 {
                    op1.clone()
                } else {
                    Operation::Insert {
                        pos: pos1 + text1.len(),
                        text: text1.clone(),
                    }
                }
            }
            (Operation::Insert { pos: pos1, text: text1 },
             Operation::Delete { pos: pos2, len }) => {
                if pos1 <= pos2 {
                    op1.clone()
                } else if pos1 >= pos2 + len {
                    Operation::Insert {
                        pos: pos1 - len,
                        text: text1.clone(),
                    }
                } else {
                    Operation::Insert {
                        pos: *pos2,
                        text: text1.clone(),
                    }
                }
            }
            (Operation::Delete { pos: pos1, len: len1 },
             Operation::Insert { pos: pos2, text }) => {
                if pos1 + len1 <= *pos2 {
                    op1.clone()
                } else if pos1 >= pos2 {
                    Operation::Delete {
                        pos: pos1 + text.len(),
                        len: *len1,
                    }
                } else {
                    Operation::Delete {
                        pos: *pos1,
                        len: len1 + text.len(),
                    }
                }
            }
            (Operation::Delete { pos: pos1, len: len1 },
             Operation::Delete { pos: pos2, len: len2 }) => {
                if pos1 + len1 <= *pos2 {
                    op1.clone()
                } else if pos1 >= pos2 + len2 {
                    Operation::Delete {
                        pos: pos1 - len2,
                        len: *len1,
                    }
                } else {
                    // Overlapping deletes
                    let start = pos1.min(pos2);
                    let end = (pos1 + len1).max(pos2 + len2);
                    Operation::Delete {
                        pos: *start,
                        len: end - start,
                    }
                }
            }
        }
    }
}

pub struct CollaborativeEditor {
    client: Client,
    document: Arc<RwLock<Document>>,
    user_id: String,
    user_name: String,
    presence: Arc<RwLock<HashMap<String, UserPresence>>>,
    message_tx: mpsc::Sender<Message>,
    message_rx: mpsc::Receiver<Message>,
}

impl CollaborativeEditor {
    pub async fn new(
        document_id: String,
        user_name: String,
    ) -> Result<Self> {
        let client = Client::connect(ClientConfig::default()).await?;
        let user_id = Uuid::new_v4().to_string();
        let document = Arc::new(RwLock::new(Document::new(document_id.clone())));
        let presence = Arc::new(RwLock::new(HashMap::new()));
        let (message_tx, message_rx) = mpsc::channel(100);
        
        Ok(Self {
            client,
            document,
            user_id,
            user_name,
            presence,
            message_tx,
            message_rx,
        })
    }
    
    pub async fn start(&mut self) -> Result<()> {
        let doc_id = self.document.read().await.id.clone();
        let topic = format!("collab:{}", doc_id);
        
        // Subscribe to document channel
        let mut stream = self.client.subscribe(&topic).await?;
        
        // Send join message
        let join_msg = Message::Join {
            user_id: self.user_id.clone(),
            name: self.user_name.clone(),
        };
        self.broadcast_message(join_msg).await?;
        
        // Handle incoming messages
        let document = Arc::clone(&self.document);
        let presence = Arc::clone(&self.presence);
        let user_id = self.user_id.clone();
        let tx = self.message_tx.clone();
        
        tokio::spawn(async move {
            while let Some(data) = stream.next().await {
                if let Ok(msg) = serde_json::from_slice::<Message>(&data) {
                    match msg {
                        Message::Edit(edit) => {
                            if edit.user_id != user_id {
                                let _ = tx.send(Message::Edit(edit)).await;
                            }
                        }
                        Message::Presence(pres) => {
                            if pres.user_id != user_id {
                                presence.write().await
                                    .insert(pres.user_id.clone(), pres);
                            }
                        }
                        Message::Join { user_id: uid, name } => {
                            println!("{} joined", name);
                            // Send current state to new user
                            let doc = document.read().await;
                            if doc.acknowledged_edits.len() > 0 {
                                let sync_response = Message::SyncResponse {
                                    edits: doc.acknowledged_edits
                                        .values()
                                        .cloned()
                                        .collect(),
                                };
                                // In real implementation, send directly to user
                            }
                        }
                        Message::Leave { user_id: uid } => {
                            presence.write().await.remove(&uid);
                        }
                        _ => {}
                    }
                }
            }
        });
        
        Ok(())
    }
    
    pub async fn insert_text(&mut self, pos: usize, text: String) -> Result<()> {
        let edit = Edit {
            id: Uuid::new_v4().to_string(),
            user_id: self.user_id.clone(),
            operation: Operation::Insert { pos, text },
            timestamp: current_timestamp(),
            parent_version: self.document.read().await.version,
        };
        
        // Apply locally
        self.apply_edit(&edit).await?;
        
        // Broadcast to others
        self.broadcast_message(Message::Edit(edit)).await?;
        
        Ok(())
    }
    
    pub async fn delete_text(&mut self, pos: usize, len: usize) -> Result<()> {
        let edit = Edit {
            id: Uuid::new_v4().to_string(),
            user_id: self.user_id.clone(),
            operation: Operation::Delete { pos, len },
            timestamp: current_timestamp(),
            parent_version: self.document.read().await.version,
        };
        
        // Apply locally
        self.apply_edit(&edit).await?;
        
        // Broadcast to others
        self.broadcast_message(Message::Edit(edit)).await?;
        
        Ok(())
    }
    
    async fn apply_edit(&mut self, edit: &Edit) -> Result<()> {
        let mut doc = self.document.write().await;
        
        // Transform against pending edits
        let mut transformed_op = edit.operation.clone();
        for pending in &doc.pending_edits {
            transformed_op = Document::transform_operation(
                &transformed_op,
                &pending.operation
            );
        }
        
        // Apply operation
        doc.apply_operation(&transformed_op)?;
        doc.version += 1;
        doc.acknowledged_edits.insert(edit.id.clone(), edit.clone());
        
        Ok(())
    }
    
    pub async fn update_cursor(&mut self, position: usize) -> Result<()> {
        let presence = UserPresence {
            user_id: self.user_id.clone(),
            name: self.user_name.clone(),
            cursor_position: position,
            selection: None,
            color: self.get_user_color(),
        };
        
        self.broadcast_message(Message::Presence(presence)).await?;
        Ok(())
    }
    
    pub async fn save_document(&self) -> Result<ContentHash> {
        let doc = self.document.read().await;
        let data = serde_json::to_vec(&*doc)?;
        self.client.store(data).await
    }
    
    pub async fn load_document(
        &mut self,
        hash: &ContentHash
    ) -> Result<()> {
        let data = self.client.retrieve(hash).await?;
        let loaded_doc: Document = serde_json::from_slice(&data)?;
        
        let mut doc = self.document.write().await;
        *doc = loaded_doc;
        
        Ok(())
    }
    
    async fn broadcast_message(&self, msg: Message) -> Result<()> {
        let doc_id = self.document.read().await.id.clone();
        let topic = format!("collab:{}", doc_id);
        let data = serde_json::to_vec(&msg)?;
        
        self.client.publish(&topic, data).await
    }
    
    fn get_user_color(&self) -> String {
        // Generate consistent color based on user ID
        let colors = vec![
            "#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4",
            "#FECA57", "#DDA0DD", "#98D8C8", "#F7DC6F",
        ];
        let index = self.user_id.bytes().sum::<u8>() as usize % colors.len();
        colors[index].to_string()
    }
    
    pub async fn get_content(&self) -> String {
        self.document.read().await.content.clone()
    }
    
    pub async fn get_users(&self) -> Vec<UserPresence> {
        self.presence.read().await.values().cloned().collect()
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// Example usage
#[tokio::main]
async fn main() -> Result<()> {
    let mut editor = CollaborativeEditor::new(
        "doc-12345".to_string(),
        "Alice".to_string(),
    ).await?;
    
    editor.start().await?;
    
    // Simulate editing
    editor.insert_text(0, "Hello, ".to_string()).await?;
    editor.insert_text(7, "collaborative ".to_string()).await?;
    editor.insert_text(21, "world!".to_string()).await?;
    
    // Update cursor position
    editor.update_cursor(27).await?;
    
    // Save document
    let hash = editor.save_document().await?;
    println!("Document saved with hash: {:?}", hash);
    
    // Keep running
    tokio::signal::ctrl_c().await?;
    
    Ok(())
}
```

## Frontend Integration

### React Component Example

```javascript
import React, { useState, useEffect, useRef } from 'react';
import { CollabClient } from './p2p-collab-client';

function CollaborativeEditor({ documentId, userName }) {
  const [content, setContent] = useState('');
  const [users, setUsers] = useState([]);
  const [client, setClient] = useState(null);
  const editorRef = useRef(null);
  
  useEffect(() => {
    // Initialize P2P client
    const initClient = async () => {
      const collabClient = new CollabClient(documentId, userName);
      await collabClient.connect();
      
      // Set up event handlers
      collabClient.on('contentChanged', (newContent) => {
        setContent(newContent);
      });
      
      collabClient.on('usersChanged', (newUsers) => {
        setUsers(newUsers);
      });
      
      setClient(collabClient);
    };
    
    initClient();
    
    return () => {
      if (client) {
        client.disconnect();
      }
    };
  }, [documentId, userName]);
  
  const handleInput = async (event) => {
    if (!client) return;
    
    const { selectionStart, selectionEnd, value } = event.target;
    const oldValue = content;
    
    if (value.length > oldValue.length) {
      // Text inserted
      const insertPos = selectionEnd - (value.length - oldValue.length);
      const insertedText = value.slice(insertPos, selectionEnd);
      await client.insertText(insertPos, insertedText);
    } else if (value.length < oldValue.length) {
      // Text deleted
      const deletePos = selectionStart;
      const deleteLen = oldValue.length - value.length;
      await client.deleteText(deletePos, deleteLen);
    }
  };
  
  const handleCursorChange = async (event) => {
    if (!client) return;
    await client.updateCursor(event.target.selectionStart);
  };
  
  const renderUserCursors = () => {
    return users.map(user => (
      <div
        key={user.userId}
        className="user-cursor"
        style={{
          left: `${getCursorPosition(user.cursorPosition)}px`,
          backgroundColor: user.color,
        }}
      >
        <span className="user-name">{user.name}</span>
      </div>
    ));
  };
  
  return (
    <div className="collaborative-editor">
      <div className="user-list">
        <h3>Active Users</h3>
        {users.map(user => (
          <div key={user.userId} className="user">
            <span 
              className="user-indicator" 
              style={{ backgroundColor: user.color }}
            />
            {user.name}
          </div>
        ))}
      </div>
      
      <div className="editor-container">
        <textarea
          ref={editorRef}
          value={content}
          onChange={handleInput}
          onSelect={handleCursorChange}
          className="editor"
          placeholder="Start typing..."
        />
        {renderUserCursors()}
      </div>
    </div>
  );
}
```

## Advanced Features

### Conflict Resolution

Implement more sophisticated OT:

```rust
use operational_transform::{Operation as OTOp, transform};

impl Document {
    fn apply_ot_operation(&mut self, op: OTOp) -> Result<()> {
        // Use proper OT library for production
        self.content = op.apply(&self.content)?;
        Ok(())
    }
}
```

### Offline Support

Cache edits locally:

```rust
struct OfflineCache {
    pending_edits: Vec<Edit>,
    last_sync_version: u64,
}

impl OfflineCache {
    async fn sync(&mut self, editor: &mut CollaborativeEditor) -> Result<()> {
        for edit in &self.pending_edits {
            editor.apply_edit(edit).await?;
            editor.broadcast_message(Message::Edit(edit.clone())).await?;
        }
        self.pending_edits.clear();
        Ok(())
    }
}
```

### Rich Text Support

Extend operations for formatting:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
enum RichOperation {
    Insert { pos: usize, text: String, format: TextFormat },
    Delete { pos: usize, len: usize },
    Format { start: usize, end: usize, format: TextFormat },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TextFormat {
    bold: Option<bool>,
    italic: Option<bool>,
    color: Option<String>,
    font_size: Option<u32>,
}
```

## Performance Optimization

### Debouncing

Reduce network traffic:

```rust
use tokio::time::{interval, Duration};

struct DebouncedBroadcaster {
    pending: Arc<RwLock<Vec<Message>>>,
    client: Client,
}

impl DebouncedBroadcaster {
    fn start(self) {
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(100));
            
            loop {
                ticker.tick().await;
                
                let messages = {
                    let mut pending = self.pending.write().await;
                    std::mem::take(&mut *pending)
                };
                
                for msg in messages {
                    let _ = self.broadcast(msg).await;
                }
            }
        });
    }
}
```

### Compression

Compress large documents:

```rust
use flate2::Compression;
use flate2::write::GzEncoder;

async fn save_compressed(doc: &Document, client: &Client) -> Result<ContentHash> {
    let data = serde_json::to_vec(doc)?;
    
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&data)?;
    let compressed = encoder.finish()?;
    
    client.store(compressed).await
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_concurrent_edits() {
        let mut editor1 = CollaborativeEditor::new(
            "test-doc".to_string(),
            "User1".to_string(),
        ).await.unwrap();
        
        let mut editor2 = CollaborativeEditor::new(
            "test-doc".to_string(),
            "User2".to_string(),
        ).await.unwrap();
        
        // Simulate concurrent edits
        editor1.insert_text(0, "Hello ".to_string()).await.unwrap();
        editor2.insert_text(0, "World ".to_string()).await.unwrap();
        
        // Wait for sync
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Both should have same content
        let content1 = editor1.get_content().await;
        let content2 = editor2.get_content().await;
        
        assert_eq!(content1, content2);
    }
}
```

## Conclusion

This example demonstrates building a real-time collaborative editor with:
- Operational Transformation for consistency
- Multi-user presence and awareness
- Document persistence in P2P network
- Offline support capabilities
- Scalable pub/sub architecture

The Adaptive P2P Network provides the infrastructure for real-time communication and distributed storage, enabling collaborative applications without central servers.