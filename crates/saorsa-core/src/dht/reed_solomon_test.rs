#[cfg(test)]
mod tests {
    use super::super::reed_solomon::*;

    #[test]
    fn test_reed_solomon_creation() {
        let rs = ReedSolomon::new(4, 2).unwrap();
        assert_eq!(rs.data_shards(), 4);
        assert_eq!(rs.parity_shards(), 2);
        assert_eq!(rs.total_shards(), 6);
    }

    #[test]
    fn test_encode_decode_small_data() {
        let rs = ReedSolomon::new(4, 2).unwrap();
        let data = b"Hello, Reed-Solomon!";
        
        // Encode
        let encoded = rs.encode(data).unwrap();
        assert_eq!(encoded.len(), 6); // 4 data + 2 parity
        
        // Decode without errors
        let decoded = rs.decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_error_correction_single_shard() {
        let rs = ReedSolomon::new(4, 2).unwrap();
        let data = b"Test data for error correction";
        
        let mut encoded = rs.encode(data).unwrap();
        
        // Corrupt one shard
        encoded[0] = vec![0; encoded[0].len()];
        
        // Should still decode correctly
        let decoded = rs.decode_with_recovery(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_error_correction_multiple_shards() {
        let rs = ReedSolomon::new(4, 2).unwrap();
        let data = b"Multiple error correction test";
        
        let mut encoded = rs.encode(data).unwrap();
        
        // Corrupt two shards (equal to parity count)
        encoded[0] = vec![0; encoded[0].len()];
        encoded[3] = vec![0; encoded[3].len()];
        
        // Should still decode correctly
        let decoded = rs.decode_with_recovery(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_too_many_errors() {
        let rs = ReedSolomon::new(4, 2).unwrap();
        let data = b"Too many errors test";
        
        let mut encoded = rs.encode(data).unwrap();
        
        // Corrupt three shards (more than parity count)
        encoded[0] = vec![0; encoded[0].len()];
        encoded[1] = vec![0; encoded[1].len()];
        encoded[2] = vec![0; encoded[2].len()];
        
        // Should fail to decode
        assert!(rs.decode_with_recovery(&encoded).is_err());
    }

    #[test]
    fn test_large_data_encoding() {
        let rs = ReedSolomon::new(8, 4).unwrap();
        let large_data = vec![42u8; 10_000]; // 10KB
        
        let encoded = rs.encode(&large_data).unwrap();
        assert_eq!(encoded.len(), 12); // 8 data + 4 parity
        
        let decoded = rs.decode(&encoded).unwrap();
        assert_eq!(decoded, large_data);
    }

    #[test]
    fn test_different_configurations() {
        let configs = vec![
            (2, 1),   // Minimal configuration
            (4, 2),   // Standard configuration
            (8, 4),   // High redundancy
            (16, 8),  // Maximum redundancy
        ];
        
        for (data_shards, parity_shards) in configs {
            let rs = ReedSolomon::new(data_shards, parity_shards).unwrap();
            let test_data = b"Configuration test data";
            
            let encoded = rs.encode(test_data).unwrap();
            assert_eq!(encoded.len(), data_shards + parity_shards);
            
            let decoded = rs.decode(&encoded).unwrap();
            assert_eq!(decoded, test_data);
        }
    }

    #[test]
    fn test_shard_verification() {
        let rs = ReedSolomon::new(4, 2).unwrap();
        let data = b"Verification test";
        
        let encoded = rs.encode(data).unwrap();
        
        // Verify all shards are valid
        assert!(rs.verify_shards(&encoded).unwrap());
        
        // Corrupt a shard
        let mut corrupted = encoded.clone();
        corrupted[0][0] ^= 1;
        
        // Verification should detect corruption
        assert!(!rs.verify_shards(&corrupted).unwrap());
    }

    #[test]
    fn test_incremental_encoding() {
        let rs = ReedSolomon::new(4, 2).unwrap();
        
        // Encode data in chunks
        let chunk1 = b"First chunk";
        let chunk2 = b"Second chunk";
        let chunk3 = b"Third chunk";
        
        let encoded1 = rs.encode(chunk1).unwrap();
        let encoded2 = rs.encode(chunk2).unwrap();
        let encoded3 = rs.encode(chunk3).unwrap();
        
        // Decode each chunk
        assert_eq!(rs.decode(&encoded1).unwrap(), chunk1);
        assert_eq!(rs.decode(&encoded2).unwrap(), chunk2);
        assert_eq!(rs.decode(&encoded3).unwrap(), chunk3);
    }

    #[test]
    fn test_parallel_encoding() {
        use std::sync::Arc;
        use std::thread;
        
        let rs = Arc::new(ReedSolomon::new(4, 2).unwrap());
        let mut handles = vec![];
        
        for i in 0..10 {
            let rs_clone = rs.clone();
            let handle = thread::spawn(move || {
                let data = format!("Thread {} data", i).into_bytes();
                let encoded = rs_clone.encode(&data).unwrap();
                let decoded = rs_clone.decode(&encoded).unwrap();
                assert_eq!(decoded, data);
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_empty_data() {
        let rs = ReedSolomon::new(4, 2).unwrap();
        let empty_data = b"";
        
        let encoded = rs.encode(empty_data).unwrap();
        let decoded = rs.decode(&encoded).unwrap();
        assert_eq!(decoded, empty_data);
    }

    #[test]
    fn test_invalid_parameters() {
        // Zero data shards
        assert!(ReedSolomon::new(0, 2).is_err());
        
        // Zero parity shards
        assert!(ReedSolomon::new(4, 0).is_err());
        
        // Too many total shards
        assert!(ReedSolomon::new(128, 128).is_err());
    }
}