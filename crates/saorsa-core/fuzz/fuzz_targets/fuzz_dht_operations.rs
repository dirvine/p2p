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
use saorsa_core::dht::{DhtKey, DhtValue};
use saorsa_core::validation::*;

fuzz_target!(|data: &[u8]| {
    // Fuzz DHT key generation
    if !data.is_empty() {
        // Test key from bytes
        let _ = DhtKey::from_bytes(data);
        
        // Test key from string
        if let Ok(s) = std::str::from_utf8(data) {
            let _ = DhtKey::from_str(s);
            
            // Validate key format
            let _ = validate_non_empty_string(s);
            if s.len() == 64 {
                let _ = validate_hex_string(s);
            }
        }
    }
    
    // Fuzz DHT value operations
    if data.len() >= 4 {
        let value_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        
        // Limit to reasonable size to avoid OOM
        let value_size = value_size.min(1024 * 1024); // 1MB max
        
        // Create value from data
        let value_data = if data.len() > 4 {
            &data[4..data.len().min(4 + value_size)]
        } else {
            &[]
        };
        
        // Test value validation
        let _ = validate_collection_size(value_data, 0, 1024 * 1024);
        
        // Test value serialization/deserialization
        if let Ok(value) = DhtValue::new(value_data.to_vec()) {
            let _ = value.validate();
            
            // Test serialization round-trip
            if let Ok(serialized) = bincode::serialize(&value) {
                let _ = bincode::deserialize::<DhtValue>(&serialized);
            }
        }
    }
    
    // Fuzz DHT query parsing
    if let Ok(s) = std::str::from_utf8(data) {
        // Test various query formats
        let queries = [
            format!("GET {}", s),
            format!("PUT {} {}", s, s),
            format!("DELETE {}", s),
            format!("FIND_NODE {}", s),
            format!("FIND_VALUE {}", s),
        ];
        
        for query in &queries {
            // Validate query syntax
            let parts: Vec<&str> = query.split_whitespace().collect();
            if !parts.is_empty() {
                let _ = validate_non_empty_string(parts[0]);
                
                // Validate operation type
                match parts[0] {
                    "GET" | "DELETE" | "FIND_NODE" | "FIND_VALUE" => {
                        if parts.len() >= 2 {
                            let _ = validate_non_empty_string(parts[1]);
                        }
                    }
                    "PUT" => {
                        if parts.len() >= 3 {
                            let _ = validate_non_empty_string(parts[1]);
                            let _ = validate_non_empty_string(parts[2]);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    // Fuzz distance calculations
    if data.len() >= 64 {
        let key1 = &data[..32];
        let key2 = &data[32..64];
        
        if let (Ok(k1), Ok(k2)) = (DhtKey::from_bytes(key1), DhtKey::from_bytes(key2)) {
            let _ = k1.distance(&k2);
        }
    }
});