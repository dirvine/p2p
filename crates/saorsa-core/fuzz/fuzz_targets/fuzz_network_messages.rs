// Copyright (c) 2025 Saorsa Labs Limited

// This file is part of the Saorsa P2P network.

// Licensed under the AGPL-3.0 license:
// <https://www.gnu.org/licenses/agpl-3.0.html>

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.


#![no_main]

use libfuzzer_sys::fuzz_target;
use saorsa_core::network::{Message, MessageType};
use saorsa_core::validation::*;

fuzz_target!(|data: &[u8]| {
    // Skip if data is too small
    if data.len() < 4 {
        return;
    }

    // Fuzz message deserialization
    // This tests that malformed messages don't cause panics
    let _ = bincode::deserialize::<Message>(data);
    
    // Fuzz message validation
    if let Ok(msg) = bincode::deserialize::<Message>(data) {
        // Validate message fields
        let _ = validate_non_empty_string(&msg.id);
        let _ = validate_string_length(&msg.sender, 1, 1024);
        
        // Validate payload size
        if let Some(payload) = &msg.payload {
            let _ = validate_collection_size(payload, 0, 1024 * 1024); // 1MB max
        }
        
        // Validate message type specific constraints
        match &msg.msg_type {
            MessageType::Data { content_hash } => {
                // Validate hash format
                let _ = validate_string_length(content_hash, 64, 64); // SHA256 hex
                let _ = validate_hex_string(content_hash);
            }
            MessageType::Request { query } => {
                let _ = validate_non_empty_string(query);
                let _ = validate_string_length(query, 1, 1024);
            }
            MessageType::Response { request_id, .. } => {
                let _ = validate_non_empty_string(request_id);
            }
            _ => {}
        }
    }
    
    // Fuzz network address parsing
    if let Ok(s) = std::str::from_utf8(data) {
        // Test multiaddr parsing
        let _ = validate_multiaddr(s);
        
        // Test various network address formats
        if s.starts_with("/ip4/") || s.starts_with("/ip6/") {
            let parts: Vec<&str> = s.split('/').collect();
            if parts.len() >= 5 {
                // Validate IP address part
                if let Some(ip_str) = parts.get(2) {
                    let _ = validate_ip_string(ip_str);
                }
                
                // Validate port if present
                if let Some(port_str) = parts.get(4) {
                    if let Ok(port) = port_str.parse::<u16>() {
                        let _ = validate_port(port);
                    }
                }
            }
        }
    }
    
    // Fuzz protocol buffer parsing
    // This simulates receiving malformed protobuf data
    if data.len() > 10 {
        // Skip protobuf header bytes and try to parse
        let _ = protobuf::Message::parse_from_bytes(&data[2..]);
    }
});