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
use saorsa_core::address::{ThreeWords, AddressError};

fuzz_target!(|data: &[u8]| {
    // Fuzz three-word address parsing
    if let Ok(s) = std::str::from_utf8(data) {
        // Test parsing various formats
        let _ = ThreeWords::from_str(s);
        
        // Test with different separators
        let with_dots = s.replace(' ', ".");
        let _ = ThreeWords::from_str(&with_dots);
        
        let with_dashes = s.replace(' ', "-");
        let _ = ThreeWords::from_str(&with_dashes);
        
        // Test case insensitivity
        let _ = ThreeWords::from_str(&s.to_lowercase());
        let _ = ThreeWords::from_str(&s.to_uppercase());
        
        // Test validation of individual words
        let words: Vec<&str> = s.split_whitespace().collect();
        if words.len() == 3 {
            for word in &words {
                // This should validate each word is in dictionary
                let _ = ThreeWords::validate_word(word);
            }
        }
    }
    
    // Fuzz binary address format
    if data.len() >= 32 {
        // Try to construct from bytes
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&data[..32]);
        let _ = ThreeWords::from_bytes(&bytes);
    }
    
    // Fuzz coordinate-based generation
    if data.len() >= 8 {
        let lat = f64::from_le_bytes([
            data[0], data[1], data[2], data[3],
            data[4], data[5], data[6], data[7],
        ]);
        
        if data.len() >= 16 {
            let lon = f64::from_le_bytes([
                data[8], data[9], data[10], data[11],
                data[12], data[13], data[14], data[15],
            ]);
            
            // Clamp to valid coordinate ranges
            let lat = lat.max(-90.0).min(90.0);
            let lon = lon.max(-180.0).min(180.0);
            
            let _ = ThreeWords::from_coordinates(lat, lon);
        }
    }
});