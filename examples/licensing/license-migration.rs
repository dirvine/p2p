// Copyright 2024 MaidSafe Limited
//
// This example demonstrates migrating between license types

use p2p_foundation::{
    P2PNode, NodeConfig,
    licensing::{LicenseChecker, LicenseType, LicenseStatus, CommercialLicense},
};
use std::io::Write;
use anyhow::Result;

/// Example demonstrating license migration scenarios
#[tokio::main]
async fn main() -> Result<()> {
    println!("P2P Foundation License Migration Example");
    println!("=======================================\n");
    
    // Show menu
    println!("Select migration scenario:");
    println!("1. AGPL → Commercial");
    println!("2. Commercial → AGPL");
    println!("3. SMB → Enterprise");
    println!("4. Check current license");
    
    print!("\nEnter choice (1-4): ");
    std::io::stdout().flush()?;
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    match input.trim() {
        "1" => migrate_agpl_to_commercial().await?,
        "2" => migrate_commercial_to_agpl().await?,
        "3" => upgrade_commercial_tier().await?,
        "4" => check_current_license().await?,
        _ => println!("Invalid choice"),
    }
    
    Ok(())
}

/// Migrate from AGPL to Commercial license
async fn migrate_agpl_to_commercial() -> Result<()> {
    println!("\n🔄 Migrating from AGPL to Commercial License");
    println!("===========================================");
    
    // Step 1: Check current status
    println!("\n1️⃣ Checking current license status...");
    let current_checker = LicenseChecker::new(); // Default is AGPL
    let current_status = current_checker.status()?;
    println!("   Current: {}", current_status.license_type);
    
    // Step 2: Explain implications
    println!("\n2️⃣ Migration implications:");
    println!("   ✅ Can keep source code private");
    println!("   ✅ No AGPL compliance requirements");
    println!("   ✅ Commercial support included");
    println!("   💰 Requires license purchase");
    
    // Step 3: Simulate license purchase
    println!("\n3️⃣ Simulating license purchase...");
    let commercial_license = create_commercial_license("Migration Corp", "enterprise")?;
    
    // Step 4: Save license
    println!("\n4️⃣ Saving commercial license...");
    let license_path = std::env::temp_dir().join("p2p-license.json");
    commercial_license.save_to_file(&license_path)?;
    println!("   Saved to: {}", license_path.display());
    
    // Step 5: Update application
    println!("\n5️⃣ Updating application configuration...");
    let new_checker = LicenseChecker::with_status(commercial_license.to_status(0));
    
    // Step 6: Remove AGPL requirements
    println!("\n6️⃣ Removing AGPL compliance features...");
    remove_agpl_compliance_code();
    
    // Step 7: Enable commercial features
    println!("\n7️⃣ Enabling commercial features...");
    enable_commercial_features(&new_checker)?;
    
    println!("\n✅ Migration complete!");
    println!("   New license: {}", new_checker.license_type()?);
    
    Ok(())
}

/// Migrate from Commercial to AGPL license
async fn migrate_commercial_to_agpl() -> Result<()> {
    println!("\n🔄 Migrating from Commercial to AGPL License");
    println!("===========================================");
    
    // Step 1: Check current commercial license
    println!("\n1️⃣ Checking current commercial license...");
    let commercial_license = create_commercial_license("Example Corp", "enterprise")?;
    let current_checker = LicenseChecker::with_status(commercial_license.to_status(0));
    println!("   Current: {}", current_checker.license_type()?);
    
    // Step 2: Explain requirements
    println!("\n2️⃣ AGPL requirements:");
    println!("   ⚠️  Must open source entire application");
    println!("   ⚠️  Must provide source to network users");
    println!("   ⚠️  Cannot keep modifications private");
    println!("   ✅ No license fees");
    
    // Step 3: Prepare for open sourcing
    println!("\n3️⃣ Preparing for open source release...");
    prepare_open_source_release()?;
    
    // Step 4: Add AGPL compliance
    println!("\n4️⃣ Adding AGPL compliance features...");
    add_agpl_compliance_code()?;
    
    // Step 5: Update license
    println!("\n5️⃣ Updating to AGPL license...");
    let agpl_checker = LicenseChecker::new(); // Default AGPL
    
    // Step 6: Remove commercial features
    println!("\n6️⃣ Removing commercial-only features...");
    remove_commercial_features();
    
    // Step 7: Publish source code
    println!("\n7️⃣ Publishing source code...");
    publish_source_code()?;
    
    println!("\n✅ Migration complete!");
    println!("   New license: {}", agpl_checker.license_type()?);
    println!("   Source available at: https://github.com/yourorg/project");
    
    Ok(())
}

/// Upgrade commercial license tier
async fn upgrade_commercial_tier() -> Result<()> {
    println!("\n⬆️  Upgrading Commercial License Tier");
    println!("====================================");
    
    // Step 1: Check current tier
    println!("\n1️⃣ Current license tier...");
    let smb_license = create_commercial_license("Small Corp", "smb")?;
    let current_checker = LicenseChecker::with_status(smb_license.to_status(45)); // 45 users
    println!("   Current: {}", current_checker.license_type()?);
    println!("   Users: 45/50");
    
    // Step 2: Show upgrade benefits
    println!("\n2️⃣ Enterprise tier benefits:");
    println!("   ✅ Unlimited users (current: 50 limit)");
    println!("   ✅ 24/7 priority support");
    println!("   ✅ Advanced analytics");
    println!("   ✅ SLA guarantees");
    
    // Step 3: Simulate upgrade
    println!("\n3️⃣ Processing license upgrade...");
    let enterprise_license = create_commercial_license("Small Corp", "enterprise")?;
    
    // Step 4: Apply new license
    println!("\n4️⃣ Applying new license...");
    let new_checker = LicenseChecker::with_status(enterprise_license.to_status(45));
    
    // Step 5: Verify new features
    println!("\n5️⃣ Verifying new features...");
    verify_enterprise_features(&new_checker)?;
    
    println!("\n✅ Upgrade complete!");
    println!("   New tier: {}", new_checker.license_type()?);
    println!("   Users: 45/unlimited");
    
    Ok(())
}

/// Check current license status
async fn check_current_license() -> Result<()> {
    println!("\n📋 Checking Current License Status");
    println!("=================================");
    
    // Try to load existing license
    if CommercialLicense::exists() {
        let license = CommercialLicense::load()?;
        let checker = LicenseChecker::with_status(license.to_status(0));
        display_license_details(&checker)?;
    } else {
        println!("\nNo commercial license found.");
        println!("Default license: AGPL-3.0");
        
        let checker = LicenseChecker::new();
        display_license_details(&checker)?;
    }
    
    Ok(())
}

/// Helper functions

fn create_commercial_license(org: &str, tier: &str) -> Result<CommercialLicense> {
    use chrono::{Utc, Duration};
    use p2p_foundation::licensing::{CommercialLicenseTier, LicenseValidator};
    
    let tier = match tier {
        "smb" => CommercialLicenseTier::Smb,
        "enterprise" => CommercialLicenseTier::Enterprise,
        "oem" => CommercialLicenseTier::Oem,
        _ => CommercialLicenseTier::Smb,
    };
    
    Ok(CommercialLicense {
        key: LicenseValidator::generate_key(org, tier.as_ref()),
        organization: org.to_string(),
        tier,
        issued_at: Utc::now(),
        expires_at: Utc::now() + Duration::days(365),
        max_users: match tier {
            CommercialLicenseTier::Smb => Some(50),
            _ => None,
        },
        contact_email: format!("admin@{}.com", org.to_lowercase().replace(" ", "")),
        metadata: std::collections::HashMap::new(),
    })
}

fn display_license_details(checker: &LicenseChecker) -> Result<()> {
    use p2p_foundation::licensing::Feature;
    
    let status = checker.status()?;
    
    println!("\n📄 License Information:");
    println!("   Type: {}", status.license_type);
    if let Some(org) = &status.organization {
        println!("   Organization: {}", org);
    }
    println!("   Valid: {}", status.is_valid);
    if let Some(expires) = status.expires_at {
        println!("   Expires: {}", expires.format("%Y-%m-%d"));
    }
    
    println!("\n🎯 Available Features:");
    let features = [
        (Feature::Core, "Core Functionality"),
        (Feature::PriorityBootstrap, "Priority Bootstrap"),
        (Feature::Analytics, "Analytics"),
        (Feature::ApiAccess, "API Access"),
        (Feature::PremiumSupport, "Premium Support"),
    ];
    
    for (feature, name) in &features {
        let available = checker.is_feature_available(*feature)?;
        println!("   {} {}", if available { "✅" } else { "❌" }, name);
    }
    
    Ok(())
}

fn remove_agpl_compliance_code() {
    println!("   - Removing source code disclosure endpoints");
    println!("   - Removing public repository links");
    println!("   - Updating documentation");
}

fn enable_commercial_features(checker: &LicenseChecker) -> Result<()> {
    use p2p_foundation::licensing::Feature;
    
    if checker.is_feature_available(Feature::Analytics)? {
        println!("   - Enabling analytics dashboard");
    }
    if checker.is_feature_available(Feature::PriorityBootstrap)? {
        println!("   - Connecting to priority bootstrap nodes");
    }
    if checker.is_feature_available(Feature::ApiAccess)? {
        println!("   - Enabling API access");
    }
    
    Ok(())
}

fn prepare_open_source_release() -> Result<()> {
    println!("   - Auditing code for proprietary content");
    println!("   - Removing confidential information");
    println!("   - Adding AGPL headers to all files");
    println!("   - Creating public repository");
    Ok(())
}

fn add_agpl_compliance_code() -> Result<()> {
    println!("   - Adding /source endpoint");
    println!("   - Adding license notice to UI");
    println!("   - Creating source download links");
    println!("   - Adding build instructions");
    Ok(())
}

fn remove_commercial_features() {
    println!("   - Disabling analytics dashboard");
    println!("   - Removing priority bootstrap nodes");
    println!("   - Disabling commercial API endpoints");
}

fn publish_source_code() -> Result<()> {
    println!("   - Creating GitHub repository");
    println!("   - Pushing code to public repo");
    println!("   - Adding README with build instructions");
    println!("   - Setting up CI/CD for open source");
    Ok(())
}

fn verify_enterprise_features(checker: &LicenseChecker) -> Result<()> {
    use p2p_foundation::licensing::Feature;
    
    println!("   ✅ User limit: Unlimited");
    
    if checker.is_feature_available(Feature::Analytics)? {
        println!("   ✅ Analytics: Enabled");
    }
    
    if checker.is_feature_available(Feature::PremiumSupport)? {
        println!("   ✅ Premium Support: Active");
    }
    
    Ok(())
}

// Mock trait implementations for the example
impl AsRef<str> for p2p_foundation::licensing::CommercialLicenseTier {
    fn as_ref(&self) -> &str {
        match self {
            Self::Smb => "smb",
            Self::Enterprise => "enterprise",
            Self::Oem => "oem",
        }
    }
}