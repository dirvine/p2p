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

//! Tests for four-word address error handling

#[cfg(test)]
mod tests {
    use crate::error::{IdentityError, P2PError};
    use crate::identity::four_words::{FourWordAddress, WordEncoder};
    use crate::identity::node_identity::NodeId;

    #[test]
    fn test_four_word_from_node_id_success() {
        let node_id = NodeId([0x42; 32]);
        let result = FourWordAddress::from_node_id(&node_id);

        // Should succeed with valid node ID
        assert!(result.is_ok());
        let address = result.unwrap();
        assert_eq!(address.words().len(), 4);
    }

    #[test]
    fn test_four_word_encoding_error() {
        // Test with invalid data (too short)
        let short_data = &[0u8; 3]; // Less than required 8 bytes
        let result = WordEncoder::encode(short_data);

        // Should fail with appropriate error
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            P2PError::Identity(IdentityError::InvalidFourWordAddress(_))
        ));
    }

    #[test]
    fn test_four_word_parsing_error() {
        // Test parsing invalid four-word address
        let invalid_addresses = vec![
            "invalid",                             // Too few words
            "one-two-three-four-five",             // Too many words
            "invalid-word-that-doesnt-exist-test", // Invalid words
            "",                                    // Empty string
        ];

        for invalid in invalid_addresses {
            let result = FourWordAddress::from_string(invalid);
            assert!(result.is_err(), "Should fail to parse: {}", invalid);
        }
    }
}
