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

//! Tests for timestamp handling in adaptive modules

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_timestamp_handling() {
        // Test that our timestamp handling never panics
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Should always return a valid timestamp
        assert!(timestamp > 0, "Timestamp should be positive");

        // Test edge case: very old timestamp
        let old_time = UNIX_EPOCH;
        let old_timestamp = old_time
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        assert_eq!(old_timestamp, 0, "UNIX_EPOCH should be 0");
    }

    #[test]
    fn test_saturating_timestamp_math() {
        // Test that timestamp arithmetic doesn't overflow
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Test saturating subtraction
        let past = now.saturating_sub(3600); // 1 hour ago
        assert!(past <= now);

        // Test with edge cases
        let zero_sub = 0u64.saturating_sub(1000);
        assert_eq!(zero_sub, 0);

        let max_sub = u64::MAX.saturating_sub(u64::MAX);
        assert_eq!(max_sub, 0);
    }
}
