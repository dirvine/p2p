//! Comprehensive tests for the input validation framework

use saorsa_core::validation::*;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_peer_id_validation() {
    // Valid peer IDs
    assert!(validate_peer_id("valid_peer_id_123").is_ok());
    assert!(validate_peer_id("PEER-ID-WITH-CAPS").is_ok());
    assert!(validate_peer_id(&format!("peer_with_64_chars_{}", "x".repeat(45))).is_ok());

    // Invalid peer IDs
    assert!(validate_peer_id("short").is_err()); // Too short
    assert!(validate_peer_id(&"x".repeat(100)).is_err()); // Too long
    assert!(validate_peer_id("invalid peer id").is_err()); // Contains space
    assert!(validate_peer_id("peer@id").is_err()); // Invalid character
    assert!(validate_peer_id("").is_err()); // Empty
}

#[test]
fn test_network_address_validation() {
    let ctx = ValidationContext::default();

    // Valid public addresses
    let addr: SocketAddr = "8.8.8.8:53".parse().unwrap();
    assert!(validate_network_address(&addr, &ctx).is_ok());

    let addr: SocketAddr = "[2001:4860:4860::8888]:53".parse().unwrap();
    assert!(validate_network_address(&addr, &ctx).is_ok());

    // Invalid addresses (localhost/private not allowed by default)
    let localhost: SocketAddr = "127.0.0.1:80".parse().unwrap();
    assert!(validate_network_address(&localhost, &ctx).is_err());

    let private_ip: SocketAddr = "192.168.1.1:80".parse().unwrap();
    assert!(validate_network_address(&private_ip, &ctx).is_err());

    let port_zero: SocketAddr = "8.8.8.8:0".parse().unwrap();
    assert!(validate_network_address(&port_zero, &ctx).is_err());

    // Allow localhost when configured
    let ctx_localhost = ValidationContext::default().allow_localhost();
    assert!(validate_network_address(&localhost, &ctx_localhost).is_ok());

    // Allow private IPs when configured
    let ctx_private = ValidationContext::default().allow_private_ips();
    assert!(validate_network_address(&private_ip, &ctx_private).is_ok());
}

#[test]
fn test_message_size_validation() {
    let max_size = 1024 * 1024; // 1MB

    assert!(validate_message_size(1024, max_size).is_ok());
    assert!(validate_message_size(max_size, max_size).is_ok());
    assert!(validate_message_size(max_size + 1, max_size).is_err());
    assert!(validate_message_size(0, max_size).is_ok()); // Empty messages allowed
}

#[test]
fn test_file_path_validation() {
    // Valid paths
    assert!(validate_file_path(Path::new("data/file.txt")).is_ok());
    assert!(validate_file_path(Path::new("/usr/local/bin")).is_ok());
    assert!(validate_file_path(Path::new("relative/path/file.dat")).is_ok());

    // Invalid paths - path traversal
    assert!(validate_file_path(Path::new("../etc/passwd")).is_err());
    assert!(validate_file_path(Path::new("../../secret")).is_err());
    assert!(validate_file_path(Path::new("data/../../../etc")).is_err());

    // Invalid paths - null bytes
    assert!(validate_file_path(Path::new("file\0name")).is_err());

    // Long path components
    let long_component = "x".repeat(300);
    assert!(validate_file_path(Path::new(&long_component)).is_err());
}

#[test]
fn test_dht_key_value_validation() {
    let ctx = ValidationContext::default();

    // Valid DHT key
    let key = vec![0u8; 32]; // 32-byte key
    assert!(validate_dht_key(&key, &ctx).is_ok());

    // Empty key
    assert!(validate_dht_key(&[], &ctx).is_err());

    // Key too large
    let large_key = vec![0u8; 2 * 1024 * 1024]; // 2MB
    assert!(validate_dht_key(&large_key, &ctx).is_err());

    // Valid DHT value
    let value = vec![0u8; 1024]; // 1KB value
    assert!(validate_dht_value(&value, &ctx).is_ok());

    // Value too large
    let large_value = vec![0u8; 11 * 1024 * 1024]; // 11MB
    assert!(validate_dht_value(&large_value, &ctx).is_err());
}

#[test]
fn test_rate_limiter() {
    let config = RateLimitConfig {
        window: Duration::from_secs(1),
        max_requests: 10,
        burst_size: 5,
        ..Default::default()
    };

    let limiter = RateLimiter::new(config);
    let ip: IpAddr = "192.168.1.1".parse().unwrap();

    // Should allow burst
    for i in 0..5 {
        assert!(
            limiter.check_ip(&ip).is_ok(),
            "Failed at burst request {}",
            i
        );
    }

    // Should continue allowing up to rate limit
    std::thread::sleep(Duration::from_millis(200));

    // The limiter has used 5 tokens from burst, but should refill some based on elapsed time
    // With 200ms elapsed and rate of 10/sec, we should get ~2 tokens refilled
    // So we might be able to make 1-2 more requests before hitting the limit
    let mut succeeded = 0;
    for _ in 0..3 {
        if limiter.check_ip(&ip).is_ok() {
            succeeded += 1;
        }
    }
    assert!(
        succeeded >= 1,
        "Should allow at least 1 more request after refill, but allowed {}",
        succeeded
    );

    // Should eventually rate limit
    for _ in 0..10 {
        let _ = limiter.check_ip(&ip);
    }
    assert!(limiter.check_ip(&ip).is_err(), "Rate limiting not enforced");
}

#[test]
fn test_network_message_validation() {
    let ctx = ValidationContext::default();

    // Valid message
    let valid_msg = NetworkMessage {
        peer_id: "valid_peer_id_123".to_string(),
        payload: vec![0u8; 1024],
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    assert!(valid_msg.validate(&ctx).is_ok());

    // Invalid peer ID
    let invalid_peer = NetworkMessage {
        peer_id: "short".to_string(),
        payload: vec![0u8; 1024],
        timestamp: valid_msg.timestamp,
    };

    assert!(invalid_peer.validate(&ctx).is_err());

    // Payload too large
    let large_payload = NetworkMessage {
        peer_id: "valid_peer_id_123".to_string(),
        payload: vec![0u8; 20 * 1024 * 1024], // 20MB
        timestamp: valid_msg.timestamp,
    };

    assert!(large_payload.validate(&ctx).is_err());

    // Timestamp in future
    let future_msg = NetworkMessage {
        peer_id: "valid_peer_id_123".to_string(),
        payload: vec![0u8; 1024],
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 600, // 10 minutes in future
    };

    assert!(future_msg.validate(&ctx).is_err());
}

#[test]
fn test_api_request_validation() {
    let ctx = ValidationContext::default();

    // Valid request
    let valid_req = ApiRequest {
        method: "GET".to_string(),
        path: "/api/v1/status".to_string(),
        params: [("limit".to_string(), "100".to_string())].into(),
    };

    assert!(valid_req.validate(&ctx).is_ok());

    // Invalid method
    let invalid_method = ApiRequest {
        method: "INVALID".to_string(),
        path: "/api/v1/status".to_string(),
        params: Default::default(),
    };

    assert!(invalid_method.validate(&ctx).is_err());

    // Path traversal attempt
    let path_traversal = ApiRequest {
        method: "GET".to_string(),
        path: "/api/../../../etc/passwd".to_string(),
        params: Default::default(),
    };

    assert!(path_traversal.validate(&ctx).is_err());

    // SQL injection attempt
    let sql_injection = ApiRequest {
        method: "GET".to_string(),
        path: "/api/users".to_string(),
        params: [("id".to_string(), "1; DROP TABLE users;".to_string())].into(),
    };

    assert!(sql_injection.validate(&ctx).is_err());
}

#[test]
fn test_config_value_validation() {
    // Valid values
    assert_eq!(
        validate_config_value::<u32>("100", Some(1), Some(1000)).unwrap(),
        100
    );
    assert_eq!(
        validate_config_value::<f64>("3.14", Some(0.0), Some(10.0)).unwrap(),
        3.14
    );

    // Out of range
    assert!(validate_config_value::<u32>("0", Some(1), Some(1000)).is_err());
    assert!(validate_config_value::<u32>("1001", Some(1), Some(1000)).is_err());

    // Parse error
    assert!(validate_config_value::<u32>("not_a_number", None, None).is_err());
}

#[test]
fn test_sanitization() {
    assert_eq!(sanitize_string("hello world!", 20), "helloworld");
    assert_eq!(sanitize_string("test@#$%123", 20), "test123");
    assert_eq!(
        sanitize_string("user_name-123.test", 20),
        "user_name-123.test"
    );
    assert_eq!(
        sanitize_string("very_long_string_that_exceeds_limit", 10),
        "very_long_"
    );
}

#[test]
fn test_rate_limiter_cleanup() {
    let mut config = RateLimitConfig::default();
    config.cleanup_interval = Duration::from_millis(100);
    config.window = Duration::from_millis(50);

    let limiter = Arc::new(RateLimiter::new(config));

    // Add some IPs
    for i in 0..10 {
        let ip: IpAddr = format!("192.168.1.{}", i).parse().unwrap();
        let _ = limiter.check_ip(&ip);
    }

    // Wait for cleanup
    std::thread::sleep(Duration::from_millis(200));

    // Cleanup should remove old entries
    limiter.cleanup();

    // New requests should work (entries were cleaned up)
    let ip: IpAddr = "192.168.1.1".parse().unwrap();
    assert!(limiter.check_ip(&ip).is_ok());
}

#[cfg(test)]
mod fuzzing_tests {
    use super::*;
    use quickcheck::TestResult;

    quickcheck::quickcheck! {
        fn fuzz_peer_id_validation(input: String) -> TestResult {
        // Skip if input could cause issues
        if input.len() > 1000 {
            return TestResult::discard();
        }

        let result = validate_peer_id(&input);

        // Should only succeed if it matches our criteria
        if result.is_ok() {
            assert!(input.len() >= 16 && input.len() <= 64);
            assert!(input.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
        }

        TestResult::passed()
        }

        fn fuzz_message_size(size: usize, max: usize) -> TestResult {
        if max == 0 || max > 100 * 1024 * 1024 {
            return TestResult::discard();
        }

        let result = validate_message_size(size, max);

        if result.is_ok() {
            assert!(size <= max);
        } else {
            assert!(size > max);
        }

        TestResult::passed()
        }
    }
}
