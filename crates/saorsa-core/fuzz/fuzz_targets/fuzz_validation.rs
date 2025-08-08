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
use saorsa_core::validation::*;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

fuzz_target!(|data: &[u8]| {
    // Skip if data is empty
    if data.is_empty() {
        return;
    }

    // Fuzz string validation
    if let Ok(s) = std::str::from_utf8(data) {
        // Test various string validators
        let _ = validate_non_empty_string(s);
        let _ = validate_string_length(s, 1, 1000);
        let _ = validate_alphanumeric(s);
        let _ = validate_ascii_printable(s);
        
        // Test path validation
        let _ = validate_path(s);
        
        // Test JSON validation
        let _ = validate_json_string(s);
        
        // Test regex pattern validation
        let _ = validate_regex_pattern(s);
        
        // Test multiaddress validation
        let _ = validate_multiaddr(s);
    }

    // Fuzz numeric validation
    if data.len() >= 8 {
        let num = u64::from_le_bytes([
            data[0], data[1], data[2], data[3],
            data[4], data[5], data[6], data[7],
        ]);
        
        // Test range validation
        let _ = validate_range(num as usize, 0, 1000000);
        
        // Test port validation
        if num <= u16::MAX as u64 {
            let _ = validate_port(num as u16);
        }
        
        // Test collection size validation
        let vec: Vec<u8> = data.to_vec();
        let _ = validate_collection_size(&vec, 0, 10000);
    }

    // Fuzz IP address validation
    if data.len() >= 4 {
        // Try as IPv4
        let ipv4 = Ipv4Addr::new(data[0], data[1], data[2], data[3]);
        let _ = validate_ip_address(&IpAddr::V4(ipv4));
        
        // Try as IPv6 if enough data
        if data.len() >= 16 {
            let ipv6 = Ipv6Addr::from([
                data[0], data[1], data[2], data[3],
                data[4], data[5], data[6], data[7],
                data[8], data[9], data[10], data[11],
                data[12], data[13], data[14], data[15],
            ]);
            let _ = validate_ip_address(&IpAddr::V6(ipv6));
        }
    }

    // Fuzz combined validators
    if let Ok(s) = std::str::from_utf8(data) {
        // Chain multiple validators
        let _ = validate_non_empty_string(s)
            .and_then(|_| validate_string_length(s, 1, 100))
            .and_then(|_| validate_alphanumeric(s));
    }
});