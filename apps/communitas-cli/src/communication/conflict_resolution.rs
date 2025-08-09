// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Conflict resolution mechanisms for community synchronization

use anyhow::{Result, anyhow};
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::identity::FourWordAddress;
use super::community_sync::*;

/// Conflict resolution engine
#[derive(Debug)]
pub struct ConflictResolver {
    /// Default resolution strategy
    default_strategy: ConflictResolutionStrategy,
    /// Per-community resolution strategies
    community_strategies: HashMap<Uuid, ConflictResolutionStrategy>,
    /// Trusted peers for resolution
    trusted_peers: HashSet<FourWordAddress>,
    /// Resolution history for learning
    resolution_history: Vec<ResolutionRecord>,
}

/// Record of conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionRecord {
    pub conflict_id: Uuid,
    pub community_id: Uuid,
    pub strategy_used: ConflictResolutionStrategy,
    pub conflicting_versions: usize,
    pub resolved_at: u64,
    pub success: bool,
    pub metadata: HashMap<String, String>,
}

/// Detailed conflict analysis
#[derive(Debug, Clone)]
pub struct ConflictAnalysis {
    pub conflict_id: Uuid,
    pub community_id: Uuid,
    pub conflict_type: ConflictType,
    pub severity: ConflictSeverity,
    pub affected_fields: Vec<String>,
    pub can_auto_resolve: bool,
    pub recommended_strategy: ConflictResolutionStrategy,
    pub resolution_options: Vec<ResolutionOption>,
}

/// Types of conflicts
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConflictType {
    /// Concurrent member additions/removals
    MembershipConflict,
    /// Permission changes for same member
    PermissionConflict,
    /// Metadata updates to same key
    MetadataConflict,
    /// Community settings changes
    SettingsConflict,
    /// Description updates
    DescriptionConflict,
    /// Complex conflicts involving multiple fields
    ComplexConflict,
}

/// Conflict severity levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConflictSeverity {
    /// Minor conflicts that can be auto-resolved
    Low,
    /// Moderate conflicts that may need attention
    Medium,
    /// Major conflicts requiring manual intervention
    High,
    /// Critical conflicts that could cause data loss
    Critical,
}

/// Resolution options for conflicts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionOption {
    pub option_id: String,
    pub name: String,
    pub description: String,
    pub strategy: ConflictResolutionStrategy,
    pub confidence: f32, // 0.0 to 1.0
    pub preview: Community, // What the result would look like
}

/// Merge result from conflict resolution
#[derive(Debug, Clone)]
pub struct MergeResult {
    pub success: bool,
    pub resolved_community: Option<Community>,
    pub conflicts_resolved: usize,
    pub unresolved_conflicts: Vec<UnresolvedConflict>,
    pub merge_strategy_used: ConflictResolutionStrategy,
    pub metadata: HashMap<String, String>,
}

/// Unresolved conflict details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresolvedConflict {
    pub field: String,
    pub conflict_type: ConflictType,
    pub values: Vec<ConflictValue>,
    pub reason: String,
}

/// Conflicting value with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictValue {
    pub value: serde_json::Value,
    pub source_peer: FourWordAddress,
    pub timestamp: u64,
    pub version: u64,
}

impl ConflictResolver {
    /// Create a new conflict resolver
    pub fn new(default_strategy: ConflictResolutionStrategy) -> Self {
        ConflictResolver {
            default_strategy,
            community_strategies: HashMap::new(),
            trusted_peers: HashSet::new(),
            resolution_history: Vec::new(),
        }
    }

    /// Analyze conflicts between community versions
    pub fn analyze_conflicts(&self, versions: &[Community]) -> Result<ConflictAnalysis> {
        if versions.len() < 2 {
            return Err(anyhow!("Need at least 2 versions to analyze conflicts"));
        }

        let community_id = versions[0].id;
        let conflict_id = Uuid::new_v4();
        
        // Ensure all versions are for the same community
        for version in versions {
            if version.id != community_id {
                return Err(anyhow!("All versions must be for the same community"));
            }
        }

        let mut affected_fields = Vec::new();
        let mut conflict_types = Vec::new();
        
        // Analyze membership conflicts
        if self.has_membership_conflicts(versions) {
            affected_fields.push("members".to_string());
            conflict_types.push(ConflictType::MembershipConflict);
        }
        
        // Analyze permission conflicts
        if self.has_permission_conflicts(versions) {
            affected_fields.push("permissions".to_string());
            conflict_types.push(ConflictType::PermissionConflict);
        }
        
        // Analyze metadata conflicts
        if self.has_metadata_conflicts(versions) {
            affected_fields.push("metadata".to_string());
            conflict_types.push(ConflictType::MetadataConflict);
        }
        
        // Analyze description conflicts
        if self.has_description_conflicts(versions) {
            affected_fields.push("description".to_string());
            conflict_types.push(ConflictType::DescriptionConflict);
        }
        
        // Determine overall conflict type
        let conflict_type = if conflict_types.len() > 1 {
            ConflictType::ComplexConflict
        } else if conflict_types.is_empty() {
            // No conflicts detected - versions are compatible
            ConflictType::MetadataConflict // Default to least severe
        } else {
            conflict_types[0].clone()
        };
        
        // Determine severity
        let severity = self.determine_severity(&conflict_type, &affected_fields);
        
        // Check if can auto-resolve
        let can_auto_resolve = self.can_auto_resolve(&conflict_type, versions);
        
        // Recommend strategy
        let recommended_strategy = self.recommend_strategy(community_id, &conflict_type, severity.clone());
        
        // Generate resolution options
        let resolution_options = self.generate_resolution_options(versions, &conflict_type)?;

        Ok(ConflictAnalysis {
            conflict_id,
            community_id,
            conflict_type,
            severity,
            affected_fields,
            can_auto_resolve,
            recommended_strategy,
            resolution_options,
        })
    }

    /// Resolve conflicts using specified strategy
    pub fn resolve_conflicts(
        &mut self,
        versions: &[Community],
        strategy: Option<ConflictResolutionStrategy>,
    ) -> Result<MergeResult> {
        let analysis = self.analyze_conflicts(versions)?;
        let strategy = strategy.unwrap_or(analysis.recommended_strategy.clone());
        
        let merge_result = match strategy {
            ConflictResolutionStrategy::LastWriterWins => {
                self.resolve_last_writer_wins(versions)
            }
            ConflictResolutionStrategy::AutoMerge => {
                self.resolve_auto_merge(versions)
            }
            ConflictResolutionStrategy::Manual => {
                self.resolve_manual(versions)
            }
            ConflictResolutionStrategy::TrustedPeer(ref peer) => {
                self.resolve_trusted_peer(versions, peer)
            }
        };
        
        // Record resolution attempt
        let record = ResolutionRecord {
            conflict_id: analysis.conflict_id,
            community_id: analysis.community_id,
            strategy_used: strategy,
            conflicting_versions: versions.len(),
            resolved_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            success: merge_result.success,
            metadata: merge_result.metadata.clone(),
        };
        
        self.resolution_history.push(record);
        
        Ok(merge_result)
    }

    /// Resolve using last writer wins strategy
    fn resolve_last_writer_wins(&self, versions: &[Community]) -> MergeResult {
        let latest_version = versions
            .iter()
            .max_by_key(|v| v.updated_at)
            .unwrap()
            .clone();
        
        let mut metadata = HashMap::new();
        metadata.insert("strategy".to_string(), "last_writer_wins".to_string());
        metadata.insert("winner_timestamp".to_string(), latest_version.updated_at.to_string());
        
        MergeResult {
            success: true,
            resolved_community: Some(latest_version),
            conflicts_resolved: 1,
            unresolved_conflicts: Vec::new(),
            merge_strategy_used: ConflictResolutionStrategy::LastWriterWins,
            metadata,
        }
    }

    /// Resolve using auto-merge strategy
    fn resolve_auto_merge(&self, versions: &[Community]) -> MergeResult {
        // Start with the base version (oldest or most common)
        let mut merged = versions
            .iter()
            .min_by_key(|v| v.updated_at)
            .unwrap()
            .clone();
        
        let mut conflicts_resolved = 0;
        let unresolved_conflicts = Vec::new();
        let mut metadata = HashMap::new();
        
        // Merge members from all versions
        for version in versions {
            for member in &version.members {
                if merged.members.insert(member.clone()) {
                    // New member added
                    if let Some(permission) = version.permissions.member_permissions.get(member) {
                        merged.permissions.member_permissions.insert(member.clone(), permission.clone());
                    }
                    conflicts_resolved += 1;
                }
            }
        }
        
        // Merge metadata using last writer wins for each key
        let mut metadata_sources = HashMap::new();
        for version in versions {
            for (key, value) in &version.metadata {
                if let Some(existing) = merged.metadata.get(key) {
                    if existing != value {
                        // Conflict - use version with latest timestamp
                        if version.updated_at > *metadata_sources.get(key).unwrap_or(&0) {
                            merged.metadata.insert(key.clone(), value.clone());
                            metadata_sources.insert(key.clone(), version.updated_at);
                            conflicts_resolved += 1;
                        }
                    }
                } else {
                    merged.metadata.insert(key.clone(), value.clone());
                    metadata_sources.insert(key.clone(), version.updated_at);
                }
            }
        }
        
        // Use latest description
        if let Some(latest_desc_version) = versions.iter().max_by_key(|v| v.updated_at) {
            if merged.description != latest_desc_version.description {
                merged.description = latest_desc_version.description.clone();
                conflicts_resolved += 1;
            }
        }
        
        // Update version and timestamp
        merged.version = versions.iter().map(|v| v.version).max().unwrap_or(1) + 1;
        merged.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        metadata.insert("strategy".to_string(), "auto_merge".to_string());
        metadata.insert("merged_versions".to_string(), versions.len().to_string());
        
        MergeResult {
            success: unresolved_conflicts.is_empty(),
            resolved_community: Some(merged),
            conflicts_resolved,
            unresolved_conflicts,
            merge_strategy_used: ConflictResolutionStrategy::AutoMerge,
            metadata,
        }
    }

    /// Resolve using manual strategy (prepare for manual intervention)
    fn resolve_manual(&self, versions: &[Community]) -> MergeResult {
        // Generate detailed conflict information for manual resolution
        let mut unresolved_conflicts = Vec::new();
        
        // Find conflicts that need manual resolution
        if versions.len() > 1 {
            let base = &versions[0];
            
            for (_i, version) in versions.iter().enumerate().skip(1) {
                if base.description != version.description {
                    unresolved_conflicts.push(UnresolvedConflict {
                        field: "description".to_string(),
                        conflict_type: ConflictType::DescriptionConflict,
                        values: vec![
                            ConflictValue {
                                value: serde_json::Value::String(base.description.clone()),
                                source_peer: base.created_by.clone(),
                                timestamp: base.updated_at,
                                version: base.version,
                            },
                            ConflictValue {
                                value: serde_json::Value::String(version.description.clone()),
                                source_peer: version.created_by.clone(),
                                timestamp: version.updated_at,
                                version: version.version,
                            },
                        ],
                        reason: "Manual resolution required for description changes".to_string(),
                    });
                }
            }
        }
        
        let mut metadata = HashMap::new();
        metadata.insert("strategy".to_string(), "manual".to_string());
        metadata.insert("requires_intervention".to_string(), "true".to_string());
        
        MergeResult {
            success: false, // Manual resolution always requires intervention
            resolved_community: None,
            conflicts_resolved: 0,
            unresolved_conflicts,
            merge_strategy_used: ConflictResolutionStrategy::Manual,
            metadata,
        }
    }

    /// Resolve using trusted peer strategy
    fn resolve_trusted_peer(&self, versions: &[Community], trusted_peer: &FourWordAddress) -> MergeResult {
        // Find version from trusted peer
        let trusted_version = versions
            .iter()
            .find(|v| &v.created_by == trusted_peer || 
                     v.permissions.member_permissions.get(trusted_peer)
                      .map_or(false, |p| matches!(p, PermissionLevel::Owner | PermissionLevel::Admin)))
            .cloned();
        
        let mut metadata = HashMap::new();
        metadata.insert("strategy".to_string(), "trusted_peer".to_string());
        metadata.insert("trusted_peer".to_string(), trusted_peer.to_string());
        
        if let Some(resolved) = trusted_version {
            metadata.insert("resolution_source".to_string(), "trusted_peer_version".to_string());
            MergeResult {
                success: true,
                resolved_community: Some(resolved),
                conflicts_resolved: 1,
                unresolved_conflicts: Vec::new(),
                merge_strategy_used: ConflictResolutionStrategy::TrustedPeer(trusted_peer.clone()),
                metadata,
            }
        } else {
            // Fall back to last writer wins if trusted peer version not found
            metadata.insert("fallback".to_string(), "last_writer_wins".to_string());
            let mut result = self.resolve_last_writer_wins(versions);
            result.merge_strategy_used = ConflictResolutionStrategy::TrustedPeer(trusted_peer.clone());
            result.metadata.extend(metadata);
            result
        }
    }

    /// Check if there are membership conflicts
    fn has_membership_conflicts(&self, versions: &[Community]) -> bool {
        if versions.len() < 2 { return false; }
        
        let first_members = &versions[0].members;
        versions[1..].iter().any(|v| &v.members != first_members)
    }

    /// Check if there are permission conflicts
    fn has_permission_conflicts(&self, versions: &[Community]) -> bool {
        if versions.len() < 2 { return false; }
        
        let first_perms = &versions[0].permissions.member_permissions;
        versions[1..].iter().any(|v| &v.permissions.member_permissions != first_perms)
    }

    /// Check if there are metadata conflicts
    fn has_metadata_conflicts(&self, versions: &[Community]) -> bool {
        if versions.len() < 2 { return false; }
        
        let first_metadata = &versions[0].metadata;
        versions[1..].iter().any(|v| &v.metadata != first_metadata)
    }

    /// Check if there are description conflicts
    fn has_description_conflicts(&self, versions: &[Community]) -> bool {
        if versions.len() < 2 { return false; }
        
        let first_desc = &versions[0].description;
        versions[1..].iter().any(|v| &v.description != first_desc)
    }

    /// Determine conflict severity
    fn determine_severity(&self, conflict_type: &ConflictType, _affected_fields: &[String]) -> ConflictSeverity {
        match conflict_type {
            ConflictType::MetadataConflict => ConflictSeverity::Low,
            ConflictType::DescriptionConflict => ConflictSeverity::Low,
            ConflictType::PermissionConflict => ConflictSeverity::Medium,
            ConflictType::MembershipConflict => ConflictSeverity::High,
            ConflictType::SettingsConflict => ConflictSeverity::Medium,
            ConflictType::ComplexConflict => ConflictSeverity::High,
        }
    }

    /// Check if conflicts can be auto-resolved
    fn can_auto_resolve(&self, conflict_type: &ConflictType, _versions: &[Community]) -> bool {
        matches!(conflict_type, 
            ConflictType::MetadataConflict | 
            ConflictType::DescriptionConflict |
            ConflictType::MembershipConflict
        )
    }

    /// Recommend resolution strategy
    fn recommend_strategy(
        &self,
        community_id: Uuid,
        conflict_type: &ConflictType,
        severity: ConflictSeverity,
    ) -> ConflictResolutionStrategy {
        // Check for community-specific strategy
        if let Some(strategy) = self.community_strategies.get(&community_id) {
            return strategy.clone();
        }
        
        // Recommend based on conflict type and severity
        match (conflict_type, severity) {
            (_, ConflictSeverity::Critical) => ConflictResolutionStrategy::Manual,
            (ConflictType::ComplexConflict, _) => ConflictResolutionStrategy::Manual,
            (ConflictType::PermissionConflict, ConflictSeverity::High) => ConflictResolutionStrategy::Manual,
            (ConflictType::MembershipConflict, ConflictSeverity::Low) => ConflictResolutionStrategy::AutoMerge,
            _ => self.default_strategy.clone(),
        }
    }

    /// Generate resolution options
    fn generate_resolution_options(&self, versions: &[Community], _conflict_type: &ConflictType) -> Result<Vec<ResolutionOption>> {
        let mut options = Vec::new();
        
        // Last writer wins option
        let latest_version = versions.iter().max_by_key(|v| v.updated_at).unwrap();
        options.push(ResolutionOption {
            option_id: "last_writer_wins".to_string(),
            name: "Use Latest Version".to_string(),
            description: "Use the version with the most recent timestamp".to_string(),
            strategy: ConflictResolutionStrategy::LastWriterWins,
            confidence: 0.7,
            preview: latest_version.clone(),
        });
        
        // Auto merge option (if applicable)
        if self.can_auto_resolve(_conflict_type, versions) {
            let merge_result = self.resolve_auto_merge(versions);
            if let Some(merged_community) = merge_result.resolved_community {
                options.push(ResolutionOption {
                    option_id: "auto_merge".to_string(),
                    name: "Automatic Merge".to_string(),
                    description: "Automatically merge compatible changes".to_string(),
                    strategy: ConflictResolutionStrategy::AutoMerge,
                    confidence: 0.8,
                    preview: merged_community,
                });
            }
        }
        
        // Manual resolution option
        options.push(ResolutionOption {
            option_id: "manual".to_string(),
            name: "Manual Resolution".to_string(),
            description: "Resolve conflicts manually with user intervention".to_string(),
            strategy: ConflictResolutionStrategy::Manual,
            confidence: 1.0,
            preview: versions[0].clone(), // Use first version as preview
        });
        
        Ok(options)
    }

    /// Add trusted peer
    pub fn add_trusted_peer(&mut self, peer: FourWordAddress) {
        self.trusted_peers.insert(peer);
    }

    /// Set strategy for specific community
    pub fn set_community_strategy(&mut self, community_id: Uuid, strategy: ConflictResolutionStrategy) {
        self.community_strategies.insert(community_id, strategy);
    }

    /// Get resolution history
    pub fn get_resolution_history(&self) -> &[ResolutionRecord] {
        &self.resolution_history
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        ConflictResolver::new(ConflictResolutionStrategy::LastWriterWins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_communities() -> Vec<Community> {
        let owner1 = FourWordAddress::generate().unwrap();
        
        let mut community1 = Community::new("Test".to_string(), "Description 1".to_string(), owner1.clone());
        let mut community2 = Community::new("Test".to_string(), "Description 2".to_string(), owner1.clone());
        
        // Make them the same community with different versions
        community2.id = community1.id;
        community1.updated_at = 1000;
        community2.updated_at = 2000;
        
        vec![community1, community2]
    }

    #[test]
    fn test_conflict_resolver_creation() {
        let resolver = ConflictResolver::new(ConflictResolutionStrategy::AutoMerge);
        assert_eq!(resolver.default_strategy, ConflictResolutionStrategy::AutoMerge);
        assert!(resolver.trusted_peers.is_empty());
        assert!(resolver.resolution_history.is_empty());
    }

    #[test]
    fn test_conflict_analysis() {
        let resolver = ConflictResolver::default();
        let communities = create_test_communities();
        
        let analysis = resolver.analyze_conflicts(&communities).unwrap();
        
        assert_eq!(analysis.community_id, communities[0].id);
        assert!(matches!(analysis.conflict_type, ConflictType::DescriptionConflict));
        assert_eq!(analysis.severity, ConflictSeverity::Low);
        assert!(analysis.can_auto_resolve);
    }

    #[test]
    fn test_last_writer_wins_resolution() {
        let mut resolver = ConflictResolver::default();
        let communities = create_test_communities();
        
        let result = resolver.resolve_conflicts(&communities, Some(ConflictResolutionStrategy::LastWriterWins)).unwrap();
        
        assert!(result.success);
        assert!(result.resolved_community.is_some());
        
        let resolved = result.resolved_community.unwrap();
        assert_eq!(resolved.description, "Description 2"); // Later timestamp
        assert_eq!(resolved.updated_at, 2000);
    }

    #[test]
    fn test_auto_merge_resolution() {
        let mut resolver = ConflictResolver::default();
        
        let owner1 = FourWordAddress::generate().unwrap();
        let member1 = FourWordAddress::generate().unwrap();
        let member2 = FourWordAddress::generate().unwrap();
        
        let mut community1 = Community::new("Test".to_string(), "Base description".to_string(), owner1.clone());
        let mut community2 = Community::new("Test".to_string(), "Updated description".to_string(), owner1.clone());
        
        community2.id = community1.id;
        community1.updated_at = 1000;
        community2.updated_at = 2000;
        
        // Add different members to each version
        community1.add_member(member1.clone(), PermissionLevel::Write).unwrap();
        community2.add_member(member2.clone(), PermissionLevel::Write).unwrap();
        
        let communities = vec![community1, community2];
        let result = resolver.resolve_conflicts(&communities, Some(ConflictResolutionStrategy::AutoMerge)).unwrap();
        
        assert!(result.success);
        let resolved = result.resolved_community.unwrap();
        
        // Should have both members
        assert!(resolved.members.contains(&member1));
        assert!(resolved.members.contains(&member2));
        
        // Should use latest description
        assert_eq!(resolved.description, "Updated description");
    }

    #[test]
    fn test_manual_resolution() {
        let mut resolver = ConflictResolver::default();
        let communities = create_test_communities();
        
        let result = resolver.resolve_conflicts(&communities, Some(ConflictResolutionStrategy::Manual)).unwrap();
        
        assert!(!result.success); // Manual resolution requires intervention
        assert!(result.resolved_community.is_none());
        assert!(!result.unresolved_conflicts.is_empty());
    }

    #[test]
    fn test_trusted_peer_resolution() {
        let mut resolver = ConflictResolver::default();
        let communities = create_test_communities();
        let trusted_peer = communities[1].created_by.clone();
        
        resolver.add_trusted_peer(trusted_peer.clone());
        
        let result = resolver.resolve_conflicts(
            &communities,
            Some(ConflictResolutionStrategy::TrustedPeer(trusted_peer))
        ).unwrap();
        
        assert!(result.success);
        let resolved = result.resolved_community.unwrap();
        assert_eq!(resolved.description, "Description 1"); // From trusted peer (first match)
    }

    #[test]
    fn test_conflict_detection() {
        let resolver = ConflictResolver::default();
        let communities = create_test_communities();
        
        assert!(resolver.has_description_conflicts(&communities));
        assert!(!resolver.has_membership_conflicts(&communities));
        assert!(!resolver.has_permission_conflicts(&communities));
        assert!(!resolver.has_metadata_conflicts(&communities));
    }

    #[test]
    fn test_severity_determination() {
        let resolver = ConflictResolver::default();
        
        assert_eq!(
            resolver.determine_severity(&ConflictType::MetadataConflict, &[]),
            ConflictSeverity::Low
        );
        assert_eq!(
            resolver.determine_severity(&ConflictType::MembershipConflict, &[]),
            ConflictSeverity::High
        );
        assert_eq!(
            resolver.determine_severity(&ConflictType::ComplexConflict, &[]),
            ConflictSeverity::High
        );
    }

    #[test]
    fn test_resolution_options() {
        let resolver = ConflictResolver::default();
        let communities = create_test_communities();
        
        let options = resolver.generate_resolution_options(&communities, &ConflictType::DescriptionConflict).unwrap();
        
        assert!(!options.is_empty());
        assert!(options.iter().any(|o| o.option_id == "last_writer_wins"));
        assert!(options.iter().any(|o| o.option_id == "auto_merge"));
        assert!(options.iter().any(|o| o.option_id == "manual"));
    }

    #[test]
    fn test_community_specific_strategy() {
        let mut resolver = ConflictResolver::default();
        let communities = create_test_communities();
        let community_id = communities[0].id;
        
        resolver.set_community_strategy(community_id, ConflictResolutionStrategy::Manual);
        
        let strategy = resolver.recommend_strategy(
            community_id,
            &ConflictType::DescriptionConflict,
            ConflictSeverity::Low
        );
        
        assert_eq!(strategy, ConflictResolutionStrategy::Manual);
    }

    #[test]
    fn test_resolution_history_tracking() {
        let mut resolver = ConflictResolver::default();
        let communities = create_test_communities();
        
        let initial_history_len = resolver.resolution_history.len();
        
        let _result = resolver.resolve_conflicts(&communities, None).unwrap();
        
        assert_eq!(resolver.resolution_history.len(), initial_history_len + 1);
        
        let record = resolver.resolution_history.last().unwrap();
        assert_eq!(record.community_id, communities[0].id);
        assert_eq!(record.conflicting_versions, 2);
    }

    #[test]
    fn test_conflict_value_serialization() {
        let peer = FourWordAddress::generate().unwrap();
        let conflict_value = ConflictValue {
            value: serde_json::Value::String("test".to_string()),
            source_peer: peer,
            timestamp: 1234567890,
            version: 1,
        };
        
        let serialized = serde_json::to_string(&conflict_value).unwrap();
        let deserialized: ConflictValue = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(conflict_value.version, deserialized.version);
        assert_eq!(conflict_value.timestamp, deserialized.timestamp);
    }
}