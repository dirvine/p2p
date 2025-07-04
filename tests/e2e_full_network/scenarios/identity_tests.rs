
//! Identity and organization management tests

use anyhow::{Context, Result};
use p2p_core::identity::{
    EnhancedIdentity, IdentityManager, Organization, OrganizationInfo, 
    OrganizationType, Department, DepartmentInfo, Team, TeamInfo,
    OrganizationRole, AdminPermissions, ManagerPermissions, MemberPermissions,
    ManagerScope, Device, DeviceType, IdentityProof, ProofType,
};
use std::collections::HashMap;
use crate::infrastructure::{
    test_network::DistributedTestNetwork,
    test_reporter::{TestEvent, TestEventType},
};

/// Test complete identity and organization management
pub async fn test_full_identity_system(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🔐 Testing Identity & Organization System");
    println!("=========================================");
    
    // 1. Create identities with quantum crypto
    test_create_enhanced_identities(network).await
        .context("Failed to test identity creation")?;
    
    // 2. Test organization hierarchy
    test_organization_hierarchy(network).await
        .context("Failed to test organization hierarchy")?;
    
    // 3. Test permission system
    test_permission_management(network).await
        .context("Failed to test permissions")?;
    
    // 4. Test device management
    test_multi_device_support(network).await
        .context("Failed to test device management")?;
    
    // 5. Test identity proofs
    test_identity_verification(network).await
        .context("Failed to test identity verification")?;
    
    Ok(())
}

/// Test creating enhanced identities with quantum crypto
async fn test_create_enhanced_identities(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n📝 Creating enhanced identities...");
    
    for (i, node) in network.local_nodes.iter_mut().enumerate() {
        // Create identity with custom name
        let identity_name = format!("TestUser{}", i);
        let identity = node.identity.clone();
        
        // Verify quantum crypto capabilities
        assert!(identity.capabilities.contains_key("ml_kem"));
        assert!(identity.capabilities.contains_key("ml_dsa"));
        assert!(identity.capabilities.contains_key("aes_256_gcm"));
        
        // Verify three-word address format
        let parts: Vec<&str> = identity.base_identity.three_word_address.split('.').collect();
        assert_eq!(parts.len(), 3, "Three-word address should have 3 parts");
        
        network.reporter.report_progress(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: format!("node_{}", i),
            event_type: TestEventType::NodeStarted,
            details: {
                let mut details = HashMap::new();
                details.insert("identity_name".to_string(), serde_json::json!(identity_name));
                details.insert("three_word_address".to_string(), 
                    serde_json::json!(identity.base_identity.three_word_address));
                details.insert("quantum_ready".to_string(), serde_json::json!(true));
                details
            },
            success: true,
        }).await;
    }
    
    println!("✅ Created {} enhanced identities", network.local_nodes.len());
    Ok(())
}

/// Test organization hierarchy creation
async fn test_organization_hierarchy(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🏢 Testing organization hierarchy...");
    
    let org_creator = &network.local_nodes[0];
    
    // Create main organization
    let org = org_creator.identity.create_organization(OrganizationInfo {
        name: "Test Corp".to_string(),
        org_type: OrganizationType::Business,
        description: "E2E Test Organization".to_string(),
        website: Some("https://testcorp.example".to_string()),
        contact_email: Some("admin@testcorp.example".to_string()),
    }).await?;
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "node_0".to_string(),
        event_type: TestEventType::ProjectCreated,
        details: {
            let mut details = HashMap::new();
            details.insert("org_name".to_string(), serde_json::json!(org.name));
            details.insert("org_id".to_string(), serde_json::json!(org.id));
            details.insert("org_type".to_string(), serde_json::json!("Business"));
            details
        },
        success: true,
    }).await;
    
    // Create departments
    let engineering = org_creator.identity.create_department(&org.id, DepartmentInfo {
        name: "Engineering".to_string(),
        description: "Engineering Department".to_string(),
        parent_dept_id: None,
    }).await?;
    
    let sales = org_creator.identity.create_department(&org.id, DepartmentInfo {
        name: "Sales".to_string(),
        description: "Sales Department".to_string(),
        parent_dept_id: None,
    }).await?;
    
    let marketing = org_creator.identity.create_department(&org.id, DepartmentInfo {
        name: "Marketing".to_string(),
        description: "Marketing Department".to_string(),
        parent_dept_id: None,
    }).await?;
    
    // Create sub-departments
    let backend_eng = org_creator.identity.create_department(&org.id, DepartmentInfo {
        name: "Backend Engineering".to_string(),
        description: "Backend Development".to_string(),
        parent_dept_id: Some(engineering.id.clone()),
    }).await?;
    
    let frontend_eng = org_creator.identity.create_department(&org.id, DepartmentInfo {
        name: "Frontend Engineering".to_string(),
        description: "Frontend Development".to_string(),
        parent_dept_id: Some(engineering.id.clone()),
    }).await?;
    
    // Create teams within departments
    let backend_team = org_creator.identity.create_team(&backend_eng.id, TeamInfo {
        name: "Core API Team".to_string(),
        description: "Core API Development".to_string(),
        max_members: Some(10),
    }).await?;
    
    let frontend_team = org_creator.identity.create_team(&frontend_eng.id, TeamInfo {
        name: "UI/UX Team".to_string(),
        description: "User Interface Development".to_string(),
        max_members: Some(8),
    }).await?;
    
    let sales_team = org_creator.identity.create_team(&sales.id, TeamInfo {
        name: "Enterprise Sales".to_string(),
        description: "Enterprise Customer Sales".to_string(),
        max_members: Some(15),
    }).await?;
    
    // Add members from different nodes with various roles
    let mut member_count = 0;
    for (i, node) in network.local_nodes.iter().enumerate().skip(1) {
        let role = match i % 4 {
            0 => OrganizationRole::Admin { 
                permissions: AdminPermissions {
                    can_manage_org: true,
                    can_manage_members: true,
                    can_manage_departments: true,
                    can_manage_roles: true,
                    can_view_audit_logs: true,
                    can_manage_settings: true,
                }
            },
            1 => OrganizationRole::Manager { 
                scope: ManagerScope::Department(engineering.id.clone()),
                permissions: ManagerPermissions {
                    can_manage_team_members: true,
                    can_create_projects: true,
                    can_approve_requests: true,
                    can_view_team_analytics: true,
                    can_manage_team_settings: true,
                }
            },
            2 => OrganizationRole::Manager {
                scope: ManagerScope::Team(backend_team.id.clone()),
                permissions: ManagerPermissions {
                    can_manage_team_members: true,
                    can_create_projects: true,
                    can_approve_requests: false,
                    can_view_team_analytics: true,
                    can_manage_team_settings: false,
                }
            },
            _ => OrganizationRole::Member { 
                permissions: MemberPermissions {
                    can_view_org_chart: true,
                    can_create_personal_projects: true,
                    can_join_public_channels: true,
                    can_send_messages: true,
                    can_view_directory: true,
                }
            },
        };
        
        org_creator.identity.add_organization_member(
            &org.id, 
            &node.identity.base_identity.user_id, 
            role
        ).await?;
        
        member_count += 1;
        
        // Assign to teams
        match i % 3 {
            0 => {
                org_creator.identity.add_team_member(
                    &backend_team.id,
                    &node.identity.base_identity.user_id,
                ).await?;
            }
            1 => {
                org_creator.identity.add_team_member(
                    &frontend_team.id,
                    &node.identity.base_identity.user_id,
                ).await?;
            }
            _ => {
                org_creator.identity.add_team_member(
                    &sales_team.id,
                    &node.identity.base_identity.user_id,
                ).await?;
            }
        }
    }
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ProjectCreated,
        details: {
            let mut details = HashMap::new();
            details.insert("departments_created".to_string(), serde_json::json!(5));
            details.insert("teams_created".to_string(), serde_json::json!(3));
            details.insert("members_added".to_string(), serde_json::json!(member_count));
            details
        },
        success: true,
    }).await;
    
    // Test cross-node permission checks
    test_cross_node_permissions(network, &org).await?;
    
    println!("✅ Organization hierarchy created successfully");
    Ok(())
}

/// Test cross-node permission checks
async fn test_cross_node_permissions(
    network: &mut DistributedTestNetwork, 
    org: &Organization
) -> Result<()> {
    println!("\n🔒 Testing cross-node permissions...");
    
    // Node 1 (Admin) should be able to manage departments
    let admin_node = &network.local_nodes[1];
    let can_manage = admin_node.identity
        .check_permission(&org.id, "manage_departments")
        .await?;
    assert!(can_manage, "Admin should be able to manage departments");
    
    // Node 2 (Department Manager) should manage team members in their department
    let dept_manager = &network.local_nodes[2];
    let can_manage_team = dept_manager.identity
        .check_permission(&org.id, "manage_team_members")
        .await?;
    assert!(can_manage_team, "Department manager should manage team members");
    
    // Node 4 (Regular Member) should NOT manage departments
    if network.local_nodes.len() > 4 {
        let member_node = &network.local_nodes[4];
        let cannot_manage = !member_node.identity
            .check_permission(&org.id, "manage_departments")
            .await?;
        assert!(cannot_manage, "Regular member should not manage departments");
    }
    
    println!("✅ Permission checks passed");
    Ok(())
}

/// Test permission management system
async fn test_permission_management(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n🛡️ Testing permission management...");
    
    let org_admin = &network.local_nodes[0];
    let target_user = &network.local_nodes[3];
    
    // Get organization
    let orgs = org_admin.identity.get_organizations().await?;
    let org = &orgs[0];
    
    // Test role updates
    let original_role = org_admin.identity
        .get_member_role(&org.id, &target_user.identity.base_identity.user_id)
        .await?;
    
    // Upgrade to manager
    let new_role = OrganizationRole::Manager {
        scope: ManagerScope::Organization,
        permissions: ManagerPermissions {
            can_manage_team_members: true,
            can_create_projects: true,
            can_approve_requests: true,
            can_view_team_analytics: true,
            can_manage_team_settings: true,
        }
    };
    
    org_admin.identity.update_member_role(
        &org.id,
        &target_user.identity.base_identity.user_id,
        new_role.clone()
    ).await?;
    
    // Verify role change
    let updated_role = org_admin.identity
        .get_member_role(&org.id, &target_user.identity.base_identity.user_id)
        .await?;
    
    assert!(matches!(updated_role, OrganizationRole::Manager { .. }));
    
    // Test permission inheritance
    let can_create_projects = target_user.identity
        .check_permission(&org.id, "create_projects")
        .await?;
    assert!(can_create_projects, "Manager should be able to create projects");
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::ProjectCreated,
        details: {
            let mut details = HashMap::new();
            details.insert("permission_test".to_string(), serde_json::json!("role_update"));
            details.insert("success".to_string(), serde_json::json!(true));
            details
        },
        success: true,
    }).await;
    
    println!("✅ Permission management tests passed");
    Ok(())
}

/// Test multi-device support
async fn test_multi_device_support(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n📱 Testing multi-device support...");
    
    let primary_node = &network.local_nodes[0];
    
    // Register multiple devices
    let devices = vec![
        ("Desktop", DeviceType::Desktop),
        ("Laptop", DeviceType::Laptop),
        ("Phone", DeviceType::Mobile),
        ("Tablet", DeviceType::Tablet),
    ];
    
    let mut registered_devices = Vec::new();
    
    for (name, device_type) in devices {
        let device = primary_node.identity.register_device(
            name.to_string(),
            device_type,
            Some("Test device for E2E testing".to_string()),
        ).await?;
        
        registered_devices.push(device);
        
        network.reporter.report_progress(TestEvent {
            timestamp: std::time::SystemTime::now(),
            node_id: "node_0".to_string(),
            event_type: TestEventType::NodeStarted,
            details: {
                let mut details = HashMap::new();
                details.insert("device_name".to_string(), serde_json::json!(name));
                details.insert("device_type".to_string(), serde_json::json!(format!("{:?}", device_type)));
                details
            },
            success: true,
        }).await;
    }
    
    // Test device sync
    let all_devices = primary_node.identity.get_devices().await?;
    assert_eq!(all_devices.len(), registered_devices.len() + 1); // +1 for primary device
    
    // Test device revocation
    let device_to_revoke = &registered_devices[0];
    primary_node.identity.revoke_device(&device_to_revoke.id).await?;
    
    let remaining_devices = primary_node.identity.get_devices().await?;
    assert_eq!(remaining_devices.len(), registered_devices.len());
    
    println!("✅ Multi-device support tests passed");
    Ok(())
}

/// Test identity verification system
async fn test_identity_verification(network: &mut DistributedTestNetwork) -> Result<()> {
    println!("\n✅ Testing identity verification...");
    
    // Test various proof types
    let node_a = &network.local_nodes[0];
    let node_b = &network.local_nodes[1];
    
    // Create domain proof
    let domain_proof = node_a.identity.create_identity_proof(
        ProofType::Domain,
        "testcorp.example".to_string(),
        HashMap::from([
            ("dns_record".to_string(), "saorsa-verify=abc123".to_string()),
        ])
    ).await?;
    
    // Create social proof
    let social_proof = node_a.identity.create_identity_proof(
        ProofType::Social,
        "@testcorp".to_string(),
        HashMap::from([
            ("platform".to_string(), "twitter".to_string()),
            ("profile_url".to_string(), "https://twitter.com/testcorp".to_string()),
        ])
    ).await?;
    
    // Create cryptographic proof (cross-signing)
    let crypto_proof = node_a.identity.create_identity_proof(
        ProofType::Cryptographic,
        node_b.identity.base_identity.user_id.clone(),
        HashMap::from([
            ("signature_type".to_string(), "cross_sign".to_string()),
            ("verifier_address".to_string(), node_b.three_word_address().to_string()),
        ])
    ).await?;
    
    // Verify proofs
    let proofs = node_a.identity.get_identity_proofs().await?;
    assert_eq!(proofs.len(), 3);
    
    // Test proof verification from another node
    let verified = node_b.identity.verify_identity_proof(
        &node_a.identity.base_identity.user_id,
        &crypto_proof.id
    ).await?;
    assert!(verified, "Cryptographic proof should be verified");
    
    network.reporter.report_progress(TestEvent {
        timestamp: std::time::SystemTime::now(),
        node_id: "coordinator".to_string(),
        event_type: TestEventType::NodeStarted,
        details: {
            let mut details = HashMap::new();
            details.insert("proof_types_tested".to_string(), serde_json::json!(3));
            details.insert("verification_success".to_string(), serde_json::json!(true));
            details
        },
        success: true,
    }).await;
    
    println!("✅ Identity verification tests passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_three_word_validation() {
        let valid = vec![
            "alpha.bravo.charlie",
            "test.user.address",
            "quantum.secure.node",
        ];
        
        for addr in valid {
            let parts: Vec<&str> = addr.split('.').collect();
            assert_eq!(parts.len(), 3);
            for part in parts {
                assert!(!part.is_empty());
                assert!(part.chars().all(|c| c.is_alphanumeric() || c == '-'));
            }
        }
    }
    
    #[test]
    fn test_organization_role_permissions() {
        let admin_role = OrganizationRole::Admin {
            permissions: AdminPermissions {
                can_manage_org: true,
                can_manage_members: true,
                can_manage_departments: true,
                can_manage_roles: true,
                can_view_audit_logs: true,
                can_manage_settings: true,
            }
        };
        
        match admin_role {
            OrganizationRole::Admin { permissions } => {
                assert!(permissions.can_manage_org);
                assert!(permissions.can_manage_members);
            }
            _ => panic!("Expected admin role"),
        }
    }
}