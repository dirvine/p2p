#[cfg(test)]
mod tests {
    use super::super::content_addressing::*;
    use blake3;

    #[test]
    fn test_content_hash_creation() {
        let data = b"Hello, DHT!";
        let hash = ContentHash::new(data);
        
        // Verify hash is deterministic
        let hash2 = ContentHash::new(data);
        assert_eq!(hash, hash2);
        
        // Verify different data produces different hash
        let different_data = b"Different data";
        let different_hash = ContentHash::new(different_data);
        assert_ne!(hash, different_hash);
    }

    #[test]
    fn test_content_verification() {
        let data = b"Test data for verification";
        let hash = ContentHash::new(data);
        
        assert!(hash.verify(data));
        assert!(!hash.verify(b"Wrong data"));
    }

    #[test]
    fn test_content_store_and_retrieve() {
        let mut store = ContentStore::new();
        let data = b"Store this data";
        let hash = ContentHash::new(data);
        
        // Store data
        store.store(hash.clone(), data.to_vec());
        
        // Retrieve data
        let retrieved = store.retrieve(&hash);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), data);
        
        // Try to retrieve non-existent data
        let non_existent = ContentHash::new(b"not stored");
        assert!(store.retrieve(&non_existent).is_none());
    }

    #[test]
    fn test_content_deduplication() {
        let mut store = ContentStore::new();
        let data = b"Duplicate data";
        let hash = ContentHash::new(data);
        
        // Store same data multiple times
        store.store(hash.clone(), data.to_vec());
        store.store(hash.clone(), data.to_vec());
        
        // Should only store once (deduplication)
        assert_eq!(store.size(), 1);
    }

    #[test]
    fn test_content_metadata() {
        let data = b"Data with metadata";
        let content = Content::new(data.to_vec(), ContentType::Binary);
        
        assert_eq!(content.data(), data);
        assert_eq!(content.content_type(), ContentType::Binary);
        assert!(content.created_at() > 0);
    }

    #[test]
    fn test_content_serialization() {
        let data = b"Serializable content";
        let content = Content::new(data.to_vec(), ContentType::Text);
        
        // Serialize
        let serialized = serde_json::to_string(&content).unwrap();
        
        // Deserialize
        let deserialized: Content = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.data(), content.data());
        assert_eq!(deserialized.hash(), content.hash());
    }

    #[test]
    fn test_large_content_handling() {
        let large_data = vec![0u8; 1_000_000]; // 1MB
        let hash = ContentHash::new(&large_data);
        
        let mut store = ContentStore::new();
        store.store(hash.clone(), large_data.clone());
        
        let retrieved = store.retrieve(&hash).unwrap();
        assert_eq!(retrieved.len(), 1_000_000);
    }

    #[test]
    fn test_content_type_variants() {
        let types = vec![
            ContentType::Binary,
            ContentType::Text,
            ContentType::Json,
            ContentType::Application("custom".to_string()),
        ];
        
        for content_type in types {
            let content = Content::new(b"test".to_vec(), content_type.clone());
            assert_eq!(content.content_type(), content_type);
        }
    }
}