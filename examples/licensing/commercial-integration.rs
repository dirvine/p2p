
// Copyright 2024 Your Company
// Proprietary and Confidential
//
// This example demonstrates commercial license integration for P2P Foundation

use p2p_foundation::{
    P2PNode, NodeConfig,
    licensing::{LicenseChecker, CommercialLicense, Feature, LicenseType},
};
use std::path::PathBuf;
use anyhow::Result;

/// Example application using P2P Foundation with commercial license
#[tokio::main]
async fn main() -> Result<()> {
    println!("P2P Foundation Commercial Integration Example");
    println!("============================================\n");
    
    // Initialize commercial license
    let license_checker = initialize_commercial_license().await?;
    
    // Check license status
    display_license_status(&license_checker)?;
    
    // Initialize P2P node with commercial features
    let node = create_commercial_node(license_checker).await?;
    
    // Demonstrate commercial-only features
    demonstrate_commercial_features(&node).await?;
    
    println!("\n✅ Commercial integration successful!");
    
    Ok(())
}

/// Initialize commercial license from file or environment
async fn initialize_commercial_license() -> Result<LicenseChecker> {
    println!("🔐 Initializing commercial license...");
    
    // Try to load from multiple sources
    let license = if let Ok(license_path) = std::env::var("P2P_LICENSE_PATH") {
        // Load from environment variable path
        println!("Loading license from: {}", license_path);
        CommercialLicense::load_from_file(license_path)?
    } else if CommercialLicense::exists() {
        // Load from default location
        println!("Loading license from default location");
        CommercialLicense::load()?
    } else if let Ok(license_key) = std::env::var("P2P_LICENSE_KEY") {
        // Create from environment variable
        println!("Creating license from environment variable");
        create_license_from_key(&license_key)?
    } else {
        // Demo license for testing
        println!("⚠️  No license found, creating demo license");
        create_demo_license()
    };
    
    // Validate license
    license.validate()?;
    println!("✅ License validated successfully");
    
    // Create license checker
    let checker = LicenseChecker::with_status(license.to_status(0));
    
    // Initialize global checker for library use
    p2p_foundation::licensing::init_global_checker(license.to_status(0))?;
    
    Ok(checker)
}

/// Create license from license key
fn create_license_from_key(key: &str) -> Result<CommercialLicense> {
    use chrono::{Utc, Duration};
    use p2p_foundation::licensing::CommercialLicenseTier;
    
    // In production, this would verify with license server
    Ok(CommercialLicense {
        key: key.to_string(),
        organization: "Example Corp".to_string(),
        tier: CommercialLicenseTier::Enterprise,
        issued_at: Utc::now(),
        expires_at: Utc::now() + Duration::days(365),
        max_users: None,
        contact_email: "admin@example.com".to_string(),
        metadata: std::collections::HashMap::new(),
    })
}

/// Create demo license for testing
fn create_demo_license() -> CommercialLicense {
    use p2p_foundation::licensing::LicenseValidator;
    
    LicenseValidator::create_demo_license("Demo Organization")
}

/// Display current license status
fn display_license_status(checker: &LicenseChecker) -> Result<()> {
    let status = checker.status()?;
    
    println!("\n📋 License Status:");
    println!("  Type: {}", status.license_type);
    println!("  Organization: {}", status.organization.unwrap_or_default());
    println!("  Valid: {}", if status.is_valid { "✅ Yes" } else { "❌ No" });
    
    if let Some(expires) = status.expires_at {
        let days_left = (expires - chrono::Utc::now()).num_days();
        println!("  Expires: {} ({} days remaining)", expires.format("%Y-%m-%d"), days_left);
    }
    
    // Check available features
    println!("\n🎯 Available Features:");
    let features = vec![
        (Feature::Core, "Core P2P Functionality"),
        (Feature::PriorityBootstrap, "Priority Bootstrap Nodes"),
        (Feature::EnhancedRateLimits, "Enhanced Rate Limits"),
        (Feature::Analytics, "Advanced Analytics"),
        (Feature::ApiAccess, "API Access"),
        (Feature::PremiumSupport, "Premium Support"),
        (Feature::WhiteLabel, "White Label Branding"),
    ];
    
    for (feature, name) in features {
        let available = checker.is_feature_available(feature)?;
        println!("  {} {}", if available { "✅" } else { "❌" }, name);
    }
    
    Ok(())
}

/// Create P2P node with commercial configuration
async fn create_commercial_node(checker: LicenseChecker) -> Result<P2PNode> {
    println!("\n🚀 Creating P2P node with commercial features...");
    
    let mut config = NodeConfig::default();
    
    // Apply commercial optimizations based on license
    if checker.is_feature_available(Feature::PriorityBootstrap)? {
        println!("  ⚡ Enabling priority bootstrap nodes");
        config.bootstrap_nodes = vec![
            "/ip4/premium1.p2p.network/tcp/4001".to_string(),
            "/ip4/premium2.p2p.network/tcp/4001".to_string(),
        ];
    }
    
    if checker.is_feature_available(Feature::EnhancedRateLimits)? {
        println!("  ⚡ Enabling enhanced rate limits");
        // In real implementation, would configure higher limits
    }
    
    // Create node
    let node = P2PNode::new(config).await?;
    println!("✅ P2P node created successfully");
    
    Ok(node)
}

/// Demonstrate commercial-only features
async fn demonstrate_commercial_features(node: &P2PNode) -> Result<()> {
    println!("\n🔧 Demonstrating commercial features...");
    
    // Feature 1: Analytics (Enterprise+)
    demonstrate_analytics().await?;
    
    // Feature 2: API Access (All commercial)
    demonstrate_api_access().await?;
    
    // Feature 3: Priority Support (Enterprise+)
    demonstrate_priority_support()?;
    
    Ok(())
}

/// Demonstrate analytics feature
async fn demonstrate_analytics() -> Result<()> {
    let checker = p2p_foundation::licensing::global_checker();
    
    if checker.is_feature_available(Feature::Analytics)? {
        println!("\n📊 Analytics Dashboard:");
        println!("  Total Peers: 1,234");
        println!("  Messages/sec: 456");
        println!("  Bandwidth: 12.3 MB/s");
        println!("  Uptime: 99.9%");
        
        // In production, would collect real metrics
        #[cfg(feature = "commercial")]
        {
            use p2p_foundation::analytics::AnalyticsCollector;
            let collector = AnalyticsCollector::new();
            let metrics = collector.collect().await?;
            println!("  Real metrics: {:?}", metrics);
        }
    } else {
        println!("\n📊 Analytics: Not available (requires Enterprise license)");
    }
    
    Ok(())
}

/// Demonstrate API access feature
async fn demonstrate_api_access() -> Result<()> {
    let checker = p2p_foundation::licensing::global_checker();
    
    if checker.is_feature_available(Feature::ApiAccess)? {
        println!("\n🔌 API Access Enabled:");
        println!("  Endpoint: https://api.p2p.network/v1");
        println!("  Rate Limit: 10,000 req/hour");
        println!("  Features: Full API access");
        
        // Example API usage
        println!("\n  Example API calls:");
        println!("  GET /api/v1/node/status");
        println!("  POST /api/v1/messages/send");
        println!("  GET /api/v1/peers/list");
    } else {
        println!("\n🔌 API Access: Not available (requires commercial license)");
    }
    
    Ok(())
}

/// Demonstrate priority support
fn demonstrate_priority_support() -> Result<()> {
    let checker = p2p_foundation::licensing::global_checker();
    
    if checker.is_feature_available(Feature::PremiumSupport)? {
        println!("\n🎯 Premium Support Active:");
        println!("  Support Level: 24/7 Priority");
        println!("  Response Time: < 4 hours");
        println!("  Channels: Email, Phone, Slack");
        println!("  Dedicated Engineer: Yes");
        
        // In production, would show actual support contact
        println!("\n  Contact Support:");
        println!("  📧 priority-support@maidsafe.net");
        println!("  📞 +1-800-MAIDSAFE");
        println!("  💬 Slack: #enterprise-support");
    } else {
        println!("\n🎯 Premium Support: Not available (requires Enterprise license)");
        println!("  Current support: Community forums only");
    }
    
    Ok(())
}

/// License enforcement example
mod enforcement {
    use super::*;
    
    /// Enforce license limits
    pub fn enforce_user_limit(checker: &LicenseChecker, current_users: usize) -> Result<()> {
        checker.check_user_limit(current_users)?;
        Ok(())
    }
    
    /// Check license expiration
    pub fn check_expiration(checker: &LicenseChecker) -> Result<()> {
        checker.check_expiration()?;
        Ok(())
    }
    
    /// Gate premium feature
    pub fn use_premium_feature(feature: Feature) -> Result<()> {
        let checker = p2p_foundation::licensing::global_checker();
        checker.require_feature(feature)?;
        
        // Feature is available, proceed
        println!("Using premium feature: {:?}", feature);
        Ok(())
    }
}

/// Configuration for different license tiers
mod tier_config {
    use super::*;
    
    pub struct TierConfiguration {
        pub max_connections: usize,
        pub message_rate_limit: usize,
        pub storage_limit_gb: usize,
        pub priority_support: bool,
    }
    
    pub fn get_tier_config(license_type: LicenseType) -> TierConfiguration {
        match license_type {
            LicenseType::Agpl => TierConfiguration {
                max_connections: 100,
                message_rate_limit: 10,
                storage_limit_gb: 10,
                priority_support: false,
            },
            LicenseType::CommercialSmb => TierConfiguration {
                max_connections: 500,
                message_rate_limit: 100,
                storage_limit_gb: 100,
                priority_support: false,
            },
            LicenseType::CommercialEnterprise => TierConfiguration {
                max_connections: 10_000,
                message_rate_limit: 1_000,
                storage_limit_gb: 1_000,
                priority_support: true,
            },
            LicenseType::CommercialOem => TierConfiguration {
                max_connections: usize::MAX,
                message_rate_limit: usize::MAX,
                storage_limit_gb: usize::MAX,
                priority_support: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_commercial_license_loading() {
        // Test license loading from various sources
        let result = initialize_commercial_license().await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_feature_gating() {
        let demo_license = create_demo_license();
        let checker = LicenseChecker::with_status(demo_license.to_status(0));
        
        // Core features should be available
        assert!(checker.is_feature_available(Feature::Core).unwrap());
        
        // Premium features depend on license tier
        let has_analytics = checker.is_feature_available(Feature::Analytics).unwrap();
        assert!(!has_analytics); // Demo license is SMB tier
    }
}