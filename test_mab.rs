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


// Test Multi-Armed Bandit implementation
use saorsa_core::adaptive::{MultiArmedBandit, MABConfig, RouteId, ContentType};

fn main() {
    println!("Testing Multi-Armed Bandit implementation...");
    
    // Create configuration
    let config = MABConfig {
        exploration_bonus: 1.0,
        initial_alpha: 1.0,
        initial_beta: 1.0,
        decay_factor: 0.95,
        min_samples: 5,
        persist_interval: 60,
        max_routes_per_type: 100,
    };
    
    // Create MAB instance
    let mab = MultiArmedBandit::new(config);
    
    // Test route selection
    let route1 = RouteId::new([1u8; 32]);
    let route2 = RouteId::new([2u8; 32]);
    let route3 = RouteId::new([3u8; 32]);
    
    let candidates = vec![route1.clone(), route2.clone(), route3.clone()];
    
    // Select routes
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        // Make some decisions
        for i in 0..10 {
            let decision = mab.select_route(candidates.clone(), ContentType::DHTLookup).await.unwrap();
            println!("Decision {}: Selected route {:?}", i, decision.selected_route);
            
            // Simulate success/failure
            let success = i % 3 != 0;
            mab.update_statistics(decision.selected_route, ContentType::DHTLookup, success).await.unwrap();
        }
        
        // Get current statistics
        let stats = mab.get_statistics().await;
        println!("\nStatistics after 10 decisions:");
        for ((route, content_type), route_stats) in stats {
            println!("Route {:?} for {:?}: successes={}, failures={}, score={:.3}", 
                     route, content_type, route_stats.successes, route_stats.failures, route_stats.success_rate());
        }
    });
    
    println!("\nMulti-Armed Bandit test completed successfully!");
}