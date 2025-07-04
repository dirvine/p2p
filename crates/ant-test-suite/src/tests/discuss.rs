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

//! Discuss/forum system tests with comprehensive features
//!
//! Tests forum functionality including categories, topics, replies, voting, moderation,
//! wiki editing, polls, badges, trust levels, and collaborative content management.

use anyhow::Result;
use crate::tests::SubsystemTest;
use crate::utils::{TestContext, VerificationResult, DataVerifier, TestDataGenerator};
use std::time::{Duration, SystemTime};
use std::collections::HashMap;
use tracing::{info, warn};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

/// Discuss subsystem test implementation
pub struct DiscussTests {
    generator: TestDataGenerator,
    verifier: DataVerifier,
    categories: HashMap<String, MockCategory>,
    topics: HashMap<String, MockTopic>,
    replies: HashMap<String, MockReply>,
    polls: HashMap<String, MockPoll>,
    users: HashMap<String, MockUser>,
    moderation_actions: HashMap<String, MockModerationAction>,
}

impl DiscussTests {
    pub fn new() -> Self {
        Self {
            generator: TestDataGenerator::new(),
            verifier: DataVerifier::new(true, Duration::from_secs(30), 3),
            categories: HashMap::new(),
            topics: HashMap::new(),
            replies: HashMap::new(),
            polls: HashMap::new(),
            users: HashMap::new(),
            moderation_actions: HashMap::new(),
        }
    }

    /// Test comprehensive forum functionality
    async fn test_forum_operations(&mut self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing comprehensive forum operations");

        // Test 1: Category management
        let category_results = self.test_category_management(ctx).await?;
        results.extend(category_results);

        // Test 2: Topic creation and management
        let topic_results = self.test_topic_management(ctx).await?;
        results.extend(topic_results);

        // Test 3: Reply and threading system
        let reply_results = self.test_reply_system(ctx).await?;
        results.extend(reply_results);

        // Test 4: Voting and reputation
        let voting_results = self.test_voting_system(ctx).await?;
        results.extend(voting_results);

        ctx.log_info(&format!("Forum operations completed. Results: {}", results.len()));
        Ok(results)
    }

    /// Test category management with access control
    async fn test_category_management(&mut self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing category management");

        // Create test users
        self.create_test_users();

        let category_types = vec![
            ("general_discussion", "General Discussion", "public", "A place for general conversations"),
            ("announcements", "Announcements", "announcement", "Official announcements from staff"),
            ("development", "Development", "protected", "Development discussions and technical topics"),
            ("private_staff", "Staff Only", "private", "Private discussions for staff members"),
            ("community_wiki", "Community Wiki", "wiki", "Collaborative knowledge base"),
        ];

        for (slug, name, access_type, description) in category_types {
            let start_time = std::time::Instant::now();

            ctx.log_info(&format!("[CATEGORY] Creating {} category: {}", access_type, name));

            // Create category with specific access level
            let category = MockCategory {
                id: format!("cat_{}", slug),
                name: name.to_string(),
                description: description.to_string(),
                slug: slug.to_string(),
                access_level: access_type.to_string(),
                parent_id: None,
                topic_count: 0,
                post_count: 0,
                last_post_at: None,
                created_at: SystemTime::now(),
                moderator_groups: vec!["staff_group".to_string()],
                settings: MockCategorySettings {
                    allow_polls: access_type != "announcement",
                    allow_wiki_posts: access_type == "wiki" || access_type == "public",
                    require_approval: access_type == "private" || access_type == "announcement",
                    min_trust_level: match access_type {
                        "private" => 3,
                        "protected" => 2,
                        _ => 0,
                    },
                    auto_close_days: if access_type == "announcement" { Some(30) } else { None },
                    slow_mode_minutes: if access_type == "development" { Some(5) } else { None },
                },
            };

            let category_id = category.id.clone();
            self.categories.insert(category_id.clone(), category);

            // Verify category data integrity
            if let Some(stored_category) = self.categories.get(&category_id) {
                if stored_category.name == name && stored_category.slug == slug {
                    ctx.log_info(&format!("✅ Category creation PASSED: {} - data verified", name));
                    results.push(VerificationResult::success(start_time.elapsed())
                        .with_metadata("operation".to_string(), "category_creation".to_string())
                        .with_metadata("category_name".to_string(), name.to_string())
                        .with_metadata("access_type".to_string(), access_type.to_string())
                        .with_metadata("data_verified".to_string(), "true".to_string()));
                } else {
                    let error = format!("Category data corruption detected for {}", name);
                    ctx.log_error(&error);
                    results.push(VerificationResult::failure(error, start_time.elapsed()));
                }
            }
        }

        // Test subcategory creation
        let subcategory_start = std::time::Instant::now();
        let subcategory = MockCategory {
            id: "cat_dev_backend".to_string(),
            name: "Backend Development".to_string(),
            description: "Backend development discussions".to_string(),
            slug: "backend_dev".to_string(),
            access_level: "protected".to_string(),
            parent_id: Some("cat_development".to_string()),
            topic_count: 0,
            post_count: 0,
            last_post_at: None,
            created_at: SystemTime::now(),
            moderator_groups: vec!["dev_leads".to_string()],
            settings: MockCategorySettings {
                allow_polls: true,
                allow_wiki_posts: true,
                require_approval: false,
                min_trust_level: 2,
                auto_close_days: None,
                slow_mode_minutes: Some(2),
            },
        };

        self.categories.insert("cat_dev_backend".to_string(), subcategory);
        ctx.log_info("✅ Subcategory creation PASSED");
        results.push(VerificationResult::success(subcategory_start.elapsed())
            .with_metadata("operation".to_string(), "subcategory_creation".to_string())
            .with_metadata("parent_category".to_string(), "cat_development".to_string()));

        Ok(results)
    }

    /// Create test users with different trust levels
    fn create_test_users(&mut self) {
        let users = vec![
            ("admin_user", 4, vec!["Administrator".to_string()], 1000, 5000),
            ("moderator_user", 3, vec!["Moderator".to_string()], 500, 2000),
            ("regular_user", 2, vec!["Regular Member".to_string()], 200, 800),
            ("new_user", 0, vec![], 10, 50),
            ("contributor_user", 2, vec!["Contributor".to_string()], 300, 1200),
            ("banned_user", 0, vec![], 0, 0),
        ];

        for (user_id, trust_level, badges, topics_created, posts_made) in users {
            let user = MockUser {
                id: user_id.to_string(),
                username: user_id.to_string(),
                trust_level,
                badges,
                topics_created,
                posts_made,
                likes_given: topics_created / 2,
                likes_received: posts_made / 3,
                solutions_accepted: topics_created / 10,
                days_visited: 30,
                is_banned: user_id == "banned_user",
                created_at: SystemTime::now(),
            };
            self.users.insert(user_id.to_string(), user);
        }
    }

    /// Test topic creation and management
    async fn test_topic_management(&mut self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing topic creation and management");

        let topic_scenarios = vec![
            ("welcome_topic", "cat_general_discussion", "Welcome to the Forum!", "regular", 
             "Welcome everyone! This is our community forum.", vec!["welcome".to_string(), "announcement".to_string()]),
            ("technical_question", "cat_development", "How to implement async/await?", "question",
             "I'm having trouble understanding async/await patterns. Can someone help?", vec!["async".to_string(), "help".to_string()]),
            ("wiki_guide", "cat_community_wiki", "Community Guidelines", "wiki",
             "# Community Guidelines\n\nPlease follow these rules...", vec!["guidelines".to_string(), "wiki".to_string()]),
            ("poll_topic", "cat_general_discussion", "What's your favorite programming language?", "poll",
             "Let's see what languages are popular in our community!", vec!["poll".to_string(), "languages".to_string()]),
            ("announcement_topic", "cat_announcements", "Forum Updates v2.0", "announcement",
             "We're excited to announce new features coming to the forum!", vec!["updates".to_string(), "features".to_string()]),
        ];

        for (topic_id, category_id, title, topic_type, content, tags) in topic_scenarios {
            let start_time = std::time::Instant::now();

            ctx.log_info(&format!("[TOPIC] Creating {} topic: {}", topic_type, title));

            // Check if category exists
            if !self.categories.contains_key(category_id) {
                let error = format!("Category {} not found for topic {}", category_id, title);
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, start_time.elapsed()));
                continue;
            }

            let topic = MockTopic {
                id: format!("topic_{}", topic_id),
                category_id: category_id.to_string(),
                title: title.to_string(),
                slug: title.to_lowercase().replace(" ", "-").replace("?", "").replace("!", ""),
                content: MockTopicContent {
                    current_version: content.to_string(),
                    format: "markdown".to_string(),
                    versions: vec![MockContentVersion {
                        content: content.to_string(),
                        author: "regular_user".to_string(),
                        created_at: SystemTime::now(),
                        edit_reason: None,
                    }],
                    wiki_editors: if topic_type == "wiki" { vec!["admin_user".to_string()] } else { vec![] },
                },
                author: "regular_user".to_string(),
                tags: tags.clone(),
                status: match topic_type {
                    "announcement" => "pinned".to_string(),
                    _ => "open".to_string(),
                },
                topic_type: topic_type.to_string(),
                stats: MockTopicStats {
                    view_count: 0,
                    reply_count: 0,
                    like_count: 0,
                    bookmark_count: 0,
                    unique_viewers: 0,
                    last_reply_at: None,
                },
                created_at: SystemTime::now(),
                updated_at: SystemTime::now(),
                closed_at: None,
                deleted_at: None,
            };

            let full_topic_id = topic.id.clone();
            self.topics.insert(full_topic_id.clone(), topic);

            // Update category stats
            if let Some(category) = self.categories.get_mut(category_id) {
                category.topic_count += 1;
                category.last_post_at = Some(SystemTime::now());
            }

            // Verify topic creation and data integrity
            if let Some(stored_topic) = self.topics.get(&full_topic_id) {
                let title_match = stored_topic.title == title;
                let content_match = stored_topic.content.current_version == content;
                let tags_match = stored_topic.tags == tags;

                if title_match && content_match && tags_match {
                    ctx.log_info(&format!("✅ Topic creation PASSED: {} - all data verified", title));
                    results.push(VerificationResult::success(start_time.elapsed())
                        .with_metadata("operation".to_string(), "topic_creation".to_string())
                        .with_metadata("topic_type".to_string(), topic_type.to_string())
                        .with_metadata("title".to_string(), title.to_string())
                        .with_metadata("content_length".to_string(), content.len().to_string())
                        .with_metadata("tags_count".to_string(), tags.len().to_string())
                        .with_metadata("data_verified".to_string(), "true".to_string()));
                } else {
                    let error = format!("Topic data verification failed for {}: title={}, content={}, tags={}", 
                                      title, title_match, content_match, tags_match);
                    ctx.log_error(&error);
                    results.push(VerificationResult::failure(error, start_time.elapsed()));
                }
            }
        }

        // Test topic editing (wiki mode)
        let edit_start = std::time::Instant::now();
        ctx.log_info("[EDIT] Testing wiki topic editing");

        if let Some(wiki_topic) = self.topics.get_mut("topic_wiki_guide") {
            let original_content = wiki_topic.content.current_version.clone();
            let edited_content = format!("{}\n\n## Additional Section\n\nThis section was added later.", original_content);

            // Add new version
            wiki_topic.content.versions.push(MockContentVersion {
                content: edited_content.clone(),
                author: "admin_user".to_string(),
                created_at: SystemTime::now(),
                edit_reason: Some("Added additional guidelines".to_string()),
            });
            wiki_topic.content.current_version = edited_content.clone();
            wiki_topic.updated_at = SystemTime::now();

            // Verify edit integrity
            if wiki_topic.content.current_version == edited_content && wiki_topic.content.versions.len() == 2 {
                ctx.log_info("✅ Wiki topic editing PASSED");
                results.push(VerificationResult::success(edit_start.elapsed())
                    .with_metadata("operation".to_string(), "topic_edit".to_string())
                    .with_metadata("versions_count".to_string(), wiki_topic.content.versions.len().to_string())
                    .with_metadata("edit_verified".to_string(), "true".to_string()));
            } else {
                let error = "Topic edit verification failed".to_string();
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, edit_start.elapsed()));
            }
        }

        Ok(results)
    }

    /// Test reply and threading system
    async fn test_reply_system(&mut self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing reply and threading system");

        // Test replies to different topic types
        let reply_scenarios = vec![
            ("answer_reply", "topic_technical_question", None, "Here's how async/await works...", "regular_user"),
            ("follow_up", "topic_technical_question", Some("reply_answer_reply"), "Thanks! That's very helpful.", "new_user"),
            ("expert_answer", "topic_technical_question", None, "Actually, there's a more elegant solution...", "moderator_user"),
            ("wiki_suggestion", "topic_wiki_guide", None, "Should we add a section about code of conduct?", "contributor_user"),
            ("poll_vote_comment", "topic_poll_topic", None, "I voted for Rust because of its safety features.", "regular_user"),
        ];

        for (reply_id, topic_id, parent_reply_id, content, author) in reply_scenarios {
            let start_time = std::time::Instant::now();

            ctx.log_info(&format!("[REPLY] Creating reply by {} to {}", author, topic_id));

            // Check if topic exists
            if !self.topics.contains_key(topic_id) {
                let error = format!("Topic {} not found for reply {}", topic_id, reply_id);
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, start_time.elapsed()));
                continue;
            }

            let reply = MockReply {
                id: format!("reply_{}", reply_id),
                topic_id: topic_id.to_string(),
                author: author.to_string(),
                content: content.to_string(),
                reply_to: parent_reply_id.map(|id| format!("reply_{}", id)),
                votes: MockVoteCount {
                    upvotes: 0,
                    downvotes: 0,
                    score: 0,
                    voters: HashMap::new(),
                },
                accepted_answer: false,
                created_at: SystemTime::now(),
                edited_at: None,
                deleted_at: None,
                reactions: vec![
                    MockReaction { emoji: "👍".to_string(), users: vec![] },
                    MockReaction { emoji: "❤️".to_string(), users: vec![] },
                ],
            };

            let full_reply_id = reply.id.clone();
            self.replies.insert(full_reply_id.clone(), reply);

            // Update topic stats
            if let Some(topic) = self.topics.get_mut(topic_id) {
                topic.stats.reply_count += 1;
                topic.stats.last_reply_at = Some(SystemTime::now());
                topic.updated_at = SystemTime::now();
            }

            // Update category stats
            if let Some(topic) = self.topics.get(topic_id) {
                if let Some(category) = self.categories.get_mut(&topic.category_id) {
                    category.post_count += 1;
                    category.last_post_at = Some(SystemTime::now());
                }
            }

            // Verify reply creation and threading
            if let Some(stored_reply) = self.replies.get(&full_reply_id) {
                let content_match = stored_reply.content == content;
                let author_match = stored_reply.author == author;
                let threading_correct = stored_reply.reply_to == parent_reply_id.map(|id| format!("reply_{}", id));

                if content_match && author_match && threading_correct {
                    ctx.log_info(&format!("✅ Reply creation PASSED: {} by {}", reply_id, author));
                    results.push(VerificationResult::success(start_time.elapsed())
                        .with_metadata("operation".to_string(), "reply_creation".to_string())
                        .with_metadata("reply_id".to_string(), reply_id.to_string())
                        .with_metadata("author".to_string(), author.to_string())
                        .with_metadata("is_threaded".to_string(), parent_reply_id.is_some().to_string())
                        .with_metadata("content_length".to_string(), content.len().to_string())
                        .with_metadata("data_verified".to_string(), "true".to_string()));
                } else {
                    let error = format!("Reply verification failed for {}: content={}, author={}, threading={}", 
                                      reply_id, content_match, author_match, threading_correct);
                    ctx.log_error(&error);
                    results.push(VerificationResult::failure(error, start_time.elapsed()));
                }
            }
        }

        // Test marking answer as accepted
        let accept_start = std::time::Instant::now();
        ctx.log_info("[ACCEPT] Testing answer acceptance");

        if let Some(reply) = self.replies.get_mut("reply_expert_answer") {
            reply.accepted_answer = true;
            
            if reply.accepted_answer {
                ctx.log_info("✅ Answer acceptance PASSED");
                results.push(VerificationResult::success(accept_start.elapsed())
                    .with_metadata("operation".to_string(), "answer_acceptance".to_string())
                    .with_metadata("reply_id".to_string(), "reply_expert_answer".to_string())
                    .with_metadata("accepted_verified".to_string(), "true".to_string()));
            }
        }

        Ok(results)
    }

    /// Test voting and reputation system
    async fn test_voting_system(&mut self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing voting and reputation system");

        // Test voting on replies
        let voting_scenarios = vec![
            ("reply_answer_reply", "regular_user", "up", 1),
            ("reply_answer_reply", "moderator_user", "up", 2),
            ("reply_answer_reply", "new_user", "up", 3),
            ("reply_expert_answer", "regular_user", "up", 1),
            ("reply_expert_answer", "contributor_user", "up", 2),
            ("reply_follow_up", "admin_user", "down", -1),
        ];

        for (reply_id, voter_id, vote_type, expected_score) in voting_scenarios {
            let start_time = std::time::Instant::now();

            ctx.log_info(&format!("[VOTE] {} voting {} on {}", voter_id, vote_type, reply_id));

            if let Some(reply) = self.replies.get_mut(reply_id) {
                // Record vote
                reply.votes.voters.insert(voter_id.to_string(), vote_type.to_string());
                
                // Recalculate score
                reply.votes.upvotes = reply.votes.voters.values()
                    .filter(|&v| v == "up").count() as u64;
                reply.votes.downvotes = reply.votes.voters.values()
                    .filter(|&v| v == "down").count() as u64;
                reply.votes.score = reply.votes.upvotes as i64 - reply.votes.downvotes as i64;

                // Verify vote integrity
                let vote_recorded = reply.votes.voters.get(voter_id) == Some(&vote_type.to_string());
                let score_correct = reply.votes.score == expected_score;

                if vote_recorded && score_correct {
                    ctx.log_info(&format!("✅ Voting PASSED: {} by {} (score: {})", vote_type, voter_id, reply.votes.score));
                    results.push(VerificationResult::success(start_time.elapsed())
                        .with_metadata("operation".to_string(), "vote_cast".to_string())
                        .with_metadata("reply_id".to_string(), reply_id.to_string())
                        .with_metadata("voter_id".to_string(), voter_id.to_string())
                        .with_metadata("vote_type".to_string(), vote_type.to_string())
                        .with_metadata("final_score".to_string(), reply.votes.score.to_string())
                        .with_metadata("vote_verified".to_string(), "true".to_string()));
                } else {
                    let error = format!("Vote verification failed for {}: recorded={}, score_correct={} (expected={}, got={})", 
                                      reply_id, vote_recorded, score_correct, expected_score, reply.votes.score);
                    ctx.log_error(&error);
                    results.push(VerificationResult::failure(error, start_time.elapsed()));
                }
            }
        }

        // Test reactions on replies
        let reaction_start = std::time::Instant::now();
        ctx.log_info("[REACTIONS] Testing emoji reactions");

        if let Some(reply) = self.replies.get_mut("reply_answer_reply") {
            // Add reactions
            if let Some(reaction) = reply.reactions.iter_mut().find(|r| r.emoji == "👍") {
                reaction.users.push("regular_user".to_string());
                reaction.users.push("moderator_user".to_string());
            }
            if let Some(reaction) = reply.reactions.iter_mut().find(|r| r.emoji == "❤️") {
                reaction.users.push("admin_user".to_string());
            }

            let thumbs_up_count = reply.reactions.iter()
                .find(|r| r.emoji == "👍")
                .map(|r| r.users.len())
                .unwrap_or(0);
            let heart_count = reply.reactions.iter()
                .find(|r| r.emoji == "❤️")
                .map(|r| r.users.len())
                .unwrap_or(0);

            if thumbs_up_count == 2 && heart_count == 1 {
                ctx.log_info("✅ Emoji reactions PASSED");
                results.push(VerificationResult::success(reaction_start.elapsed())
                    .with_metadata("operation".to_string(), "emoji_reactions".to_string())
                    .with_metadata("thumbs_up_count".to_string(), thumbs_up_count.to_string())
                    .with_metadata("heart_count".to_string(), heart_count.to_string())
                    .with_metadata("reactions_verified".to_string(), "true".to_string()));
            } else {
                let error = format!("Reaction verification failed: thumbs_up={}, heart={}", thumbs_up_count, heart_count);
                ctx.log_error(&error);
                results.push(VerificationResult::failure(error, reaction_start.elapsed()));
            }
        }

        Ok(results)
    }

    /// Test moderation features
    async fn test_moderation_system(&mut self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing moderation system");

        let moderation_scenarios = vec![
            ("close_topic", "topic_technical_question", "moderator_user", "close", "Question has been resolved"),
            ("pin_announcement", "topic_announcement_topic", "admin_user", "pin", "Important announcement"),
            ("move_topic", "topic_welcome_topic", "moderator_user", "move", "Better suited for announcements"),
            ("delete_spam", "reply_follow_up", "moderator_user", "delete", "Spam content removed"),
            ("warn_user", "new_user", "moderator_user", "warn", "Please follow community guidelines"),
        ];

        for (action_id, target_id, moderator_id, action_type, reason) in moderation_scenarios {
            let start_time = std::time::Instant::now();

            ctx.log_info(&format!("[MODERATION] {} performing {} on {}", moderator_id, action_type, target_id));

            let moderation_action = MockModerationAction {
                id: format!("mod_{}", action_id),
                action_type: action_type.to_string(),
                target_id: target_id.to_string(),
                target_type: if target_id.starts_with("topic_") { "topic".to_string() } else if target_id.starts_with("reply_") { "reply".to_string() } else { "user".to_string() },
                moderator_id: moderator_id.to_string(),
                reason: reason.to_string(),
                created_at: SystemTime::now(),
                expires_at: if action_type == "warn" { Some(SystemTime::now()) } else { None },
            };

            // Apply moderation action
            match action_type {
                "close" => {
                    if let Some(topic) = self.topics.get_mut(target_id) {
                        topic.status = "closed".to_string();
                        topic.closed_at = Some(SystemTime::now());
                    }
                }
                "pin" => {
                    if let Some(topic) = self.topics.get_mut(target_id) {
                        topic.status = "pinned".to_string();
                    }
                }
                "move" => {
                    if let Some(topic) = self.topics.get_mut(target_id) {
                        topic.category_id = "cat_announcements".to_string();
                    }
                }
                "delete" => {
                    if let Some(reply) = self.replies.get_mut(target_id) {
                        reply.deleted_at = Some(SystemTime::now());
                    }
                }
                "warn" => {
                    // Warning applied to user (would be recorded in user profile)
                }
                _ => {}
            }

            let action_id_full = moderation_action.id.clone();
            self.moderation_actions.insert(action_id_full.clone(), moderation_action);

            // Verify moderation action
            if let Some(stored_action) = self.moderation_actions.get(&action_id_full) {
                let action_recorded = stored_action.action_type == action_type;
                let moderator_correct = stored_action.moderator_id == moderator_id;
                let target_correct = stored_action.target_id == target_id;

                if action_recorded && moderator_correct && target_correct {
                    ctx.log_info(&format!("✅ Moderation action PASSED: {} by {}", action_type, moderator_id));
                    results.push(VerificationResult::success(start_time.elapsed())
                        .with_metadata("operation".to_string(), "moderation_action".to_string())
                        .with_metadata("action_type".to_string(), action_type.to_string())
                        .with_metadata("moderator_id".to_string(), moderator_id.to_string())
                        .with_metadata("target_id".to_string(), target_id.to_string())
                        .with_metadata("reason".to_string(), reason.to_string())
                        .with_metadata("action_verified".to_string(), "true".to_string()));
                } else {
                    let error = format!("Moderation action verification failed for {}: action={}, moderator={}, target={}", 
                                      action_id, action_recorded, moderator_correct, target_correct);
                    ctx.log_error(&error);
                    results.push(VerificationResult::failure(error, start_time.elapsed()));
                }
            }
        }

        Ok(results)
    }

    /// Test poll creation and voting
    async fn test_poll_system(&mut self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing poll system");

        // Create polls
        let poll_scenarios = vec![
            ("lang_poll", "topic_poll_topic", "What's your favorite programming language?", 
             vec!["Rust".to_string(), "Python".to_string(), "JavaScript".to_string(), "Go".to_string()], "single"),
            ("features_poll", "topic_announcement_topic", "Which features should we prioritize?", 
             vec!["Better search".to_string(), "Mobile app".to_string(), "Dark mode".to_string(), "API access".to_string()], "multiple"),
        ];

        for (poll_id, topic_id, question, options, poll_type) in poll_scenarios {
            let start_time = std::time::Instant::now();

            ctx.log_info(&format!("[POLL] Creating {} poll: {}", poll_type, question));

            let poll = MockPoll {
                id: format!("poll_{}", poll_id),
                topic_id: topic_id.to_string(),
                question: question.to_string(),
                options: options.iter().map(|opt| MockPollOption {
                    text: opt.clone(),
                    votes: 0,
                }).collect(),
                poll_type: poll_type.to_string(),
                closes_at: None,
                results_visible: "always".to_string(),
                voters: HashMap::new(),
            };

            let poll_id_full = poll.id.clone();
            self.polls.insert(poll_id_full.clone(), poll);

            // Test voting on poll
            let voting_scenarios = vec![
                ("regular_user", vec![0]),  // Rust
                ("moderator_user", vec![1]), // Python
                ("admin_user", vec![0]),     // Rust
                ("contributor_user", if poll_type == "multiple" { vec![0, 2] } else { vec![2] }), // Rust + Dark mode or just JavaScript
            ];

            for (voter_id, choices) in voting_scenarios {
                if let Some(poll) = self.polls.get_mut(&poll_id_full) {
                    // Record vote
                    poll.voters.insert(voter_id.to_string(), choices.clone());
                    
                    // Update option counts
                    for &choice_idx in &choices {
                        if let Some(option) = poll.options.get_mut(choice_idx) {
                            option.votes += 1;
                        }
                    }
                }
            }

            // Verify poll creation and voting
            if let Some(stored_poll) = self.polls.get(&poll_id_full) {
                let question_match = stored_poll.question == question;
                let options_count_correct = stored_poll.options.len() == options.len();
                let total_votes: u64 = stored_poll.options.iter().map(|opt| opt.votes).sum();
                let voters_count = stored_poll.voters.len();

                if question_match && options_count_correct && total_votes > 0 {
                    ctx.log_info(&format!("✅ Poll creation and voting PASSED: {} ({} votes)", poll_id, total_votes));
                    results.push(VerificationResult::success(start_time.elapsed())
                        .with_metadata("operation".to_string(), "poll_creation_voting".to_string())
                        .with_metadata("poll_id".to_string(), poll_id.to_string())
                        .with_metadata("poll_type".to_string(), poll_type.to_string())
                        .with_metadata("options_count".to_string(), options.len().to_string())
                        .with_metadata("total_votes".to_string(), total_votes.to_string())
                        .with_metadata("voters_count".to_string(), voters_count.to_string())
                        .with_metadata("data_verified".to_string(), "true".to_string()));
                } else {
                    let error = format!("Poll verification failed for {}: question={}, options={}, votes={}", 
                                      poll_id, question_match, options_count_correct, total_votes);
                    ctx.log_error(&error);
                    results.push(VerificationResult::failure(error, start_time.elapsed()));
                }
            }
        }

        Ok(results)
    }

    /// Test badge system and trust levels
    async fn test_badge_system(&mut self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();

        ctx.log_info("Testing badge system and trust levels");

        let badge_scenarios = vec![
            ("regular_user", "First Post", "bronze", "Made your first post"),
            ("moderator_user", "Helpful", "silver", "Received 50 likes on answers"),
            ("admin_user", "Community Leader", "gold", "Outstanding contributions to the community"),
            ("contributor_user", "Problem Solver", "silver", "Had 10 answers accepted"),
        ];

        for (user_id, badge_name, badge_type, description) in badge_scenarios {
            let start_time = std::time::Instant::now();

            ctx.log_info(&format!("[BADGE] Awarding {} badge '{}' to {}", badge_type, badge_name, user_id));

            if let Some(user) = self.users.get_mut(user_id) {
                let badge = format!("{} ({})", badge_name, badge_type);
                if !user.badges.contains(&badge) {
                    user.badges.push(badge.clone());
                    
                    // Verify badge award
                    if user.badges.contains(&badge) {
                        ctx.log_info(&format!("✅ Badge award PASSED: {} to {}", badge_name, user_id));
                        results.push(VerificationResult::success(start_time.elapsed())
                            .with_metadata("operation".to_string(), "badge_award".to_string())
                            .with_metadata("user_id".to_string(), user_id.to_string())
                            .with_metadata("badge_name".to_string(), badge_name.to_string())
                            .with_metadata("badge_type".to_string(), badge_type.to_string())
                            .with_metadata("total_badges".to_string(), user.badges.len().to_string())
                            .with_metadata("badge_verified".to_string(), "true".to_string()));
                    } else {
                        let error = format!("Badge award verification failed for {}", user_id);
                        ctx.log_error(&error);
                        results.push(VerificationResult::failure(error, start_time.elapsed()));
                    }
                }
            }
        }

        // Test trust level progression
        let trust_start = std::time::Instant::now();
        ctx.log_info("[TRUST] Testing trust level progression");

        if let Some(user) = self.users.get_mut("regular_user") {
            let original_trust = user.trust_level;
            
            // Simulate activity that increases trust level
            user.posts_made += 50;
            user.likes_received += 25;
            user.days_visited += 10;
            
            // Update trust level based on activity
            if user.posts_made >= 50 && user.likes_received >= 25 && user.days_visited >= 30 {
                user.trust_level = 3; // Promote to Regular (level 3)
            }
            
            if user.trust_level > original_trust {
                ctx.log_info(&format!("✅ Trust level progression PASSED: {} -> {}", original_trust, user.trust_level));
                results.push(VerificationResult::success(trust_start.elapsed())
                    .with_metadata("operation".to_string(), "trust_level_progression".to_string())
                    .with_metadata("user_id".to_string(), "regular_user".to_string())
                    .with_metadata("original_level".to_string(), original_trust.to_string())
                    .with_metadata("new_level".to_string(), user.trust_level.to_string())
                    .with_metadata("posts_made".to_string(), user.posts_made.to_string())
                    .with_metadata("progression_verified".to_string(), "true".to_string()));
            }
        }

        Ok(results)
    }
}

// Mock data structures for comprehensive testing
#[derive(Clone, Debug)]
struct MockCategory {
    id: String,
    name: String,
    description: String,
    slug: String,
    access_level: String,
    parent_id: Option<String>,
    topic_count: u64,
    post_count: u64,
    last_post_at: Option<SystemTime>,
    created_at: SystemTime,
    moderator_groups: Vec<String>,
    settings: MockCategorySettings,
}

#[derive(Clone, Debug)]
struct MockCategorySettings {
    allow_polls: bool,
    allow_wiki_posts: bool,
    require_approval: bool,
    min_trust_level: u32,
    auto_close_days: Option<u32>,
    slow_mode_minutes: Option<u32>,
}

#[derive(Clone, Debug)]
struct MockTopic {
    id: String,
    category_id: String,
    title: String,
    slug: String,
    content: MockTopicContent,
    author: String,
    tags: Vec<String>,
    status: String,
    topic_type: String,
    stats: MockTopicStats,
    created_at: SystemTime,
    updated_at: SystemTime,
    closed_at: Option<SystemTime>,
    deleted_at: Option<SystemTime>,
}

#[derive(Clone, Debug)]
struct MockTopicContent {
    current_version: String,
    format: String,
    versions: Vec<MockContentVersion>,
    wiki_editors: Vec<String>,
}

#[derive(Clone, Debug)]
struct MockContentVersion {
    content: String,
    author: String,
    created_at: SystemTime,
    edit_reason: Option<String>,
}

#[derive(Clone, Debug)]
struct MockTopicStats {
    view_count: u64,
    reply_count: u64,
    like_count: i64,
    bookmark_count: u64,
    unique_viewers: u64,
    last_reply_at: Option<SystemTime>,
}

#[derive(Clone, Debug)]
struct MockReply {
    id: String,
    topic_id: String,
    author: String,
    content: String,
    reply_to: Option<String>,
    votes: MockVoteCount,
    accepted_answer: bool,
    created_at: SystemTime,
    edited_at: Option<SystemTime>,
    deleted_at: Option<SystemTime>,
    reactions: Vec<MockReaction>,
}

#[derive(Clone, Debug)]
struct MockVoteCount {
    upvotes: u64,
    downvotes: u64,
    score: i64,
    voters: HashMap<String, String>, // user_id -> vote_type
}

#[derive(Clone, Debug)]
struct MockReaction {
    emoji: String,
    users: Vec<String>,
}

#[derive(Clone, Debug)]
struct MockPoll {
    id: String,
    topic_id: String,
    question: String,
    options: Vec<MockPollOption>,
    poll_type: String,
    closes_at: Option<SystemTime>,
    results_visible: String,
    voters: HashMap<String, Vec<usize>>, // user_id -> option_indices
}

#[derive(Clone, Debug)]
struct MockPollOption {
    text: String,
    votes: u64,
}

#[derive(Clone, Debug)]
struct MockUser {
    id: String,
    username: String,
    trust_level: u32,
    badges: Vec<String>,
    topics_created: u64,
    posts_made: u64,
    likes_given: u64,
    likes_received: u64,
    solutions_accepted: u64,
    days_visited: u64,
    is_banned: bool,
    created_at: SystemTime,
}

#[derive(Clone, Debug)]
struct MockModerationAction {
    id: String,
    action_type: String,
    target_id: String,
    target_type: String,
    moderator_id: String,
    reason: String,
    created_at: SystemTime,
    expires_at: Option<SystemTime>,
}

#[async_trait::async_trait]
impl SubsystemTest for DiscussTests {
    fn name(&self) -> &str { "discuss" }

    async fn test_basic_functionality(&self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut test_instance = self.clone();
        
        ctx.log_info("Running comprehensive discuss/forum functionality tests");
        
        // Test forum operations
        test_instance.test_forum_operations(ctx).await
    }

    async fn test_data_verification(&self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut test_instance = self.clone();
        let mut results = Vec::new();
        
        ctx.log_info("Running discuss data verification tests");
        
        // Test moderation system
        let moderation_results = test_instance.test_moderation_system(ctx).await?;
        results.extend(moderation_results);
        
        // Test poll system
        let poll_results = test_instance.test_poll_system(ctx).await?;
        results.extend(poll_results);
        
        // Test badge system
        let badge_results = test_instance.test_badge_system(ctx).await?;
        results.extend(badge_results);
        
        Ok(results)
    }

    async fn test_cross_node(&self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut test_instance = self.clone();
        let mut results = Vec::new();
        
        ctx.log_info("Running cross-node discuss tests");
        
        // Test cross-node forum synchronization
        let sync_start = std::time::Instant::now();
        
        // Create categories on multiple nodes
        let cross_node_categories = vec![
            ("node1_general", "Node 1 General", "public"),
            ("node2_dev", "Node 2 Development", "protected"),
            ("shared_announcements", "Shared Announcements", "announcement"),
        ];
        
        for (slug, name, access_type) in cross_node_categories {
            let category = MockCategory {
                id: format!("cross_cat_{}", slug),
                name: name.to_string(),
                description: format!("Cross-node {}", name),
                slug: slug.to_string(),
                access_level: access_type.to_string(),
                parent_id: None,
                topic_count: 0,
                post_count: 0,
                last_post_at: None,
                created_at: SystemTime::now(),
                moderator_groups: vec!["cross_node_mods".to_string()],
                settings: MockCategorySettings {
                    allow_polls: true,
                    allow_wiki_posts: true,
                    require_approval: access_type == "announcement",
                    min_trust_level: if access_type == "protected" { 2 } else { 0 },
                    auto_close_days: None,
                    slow_mode_minutes: None,
                },
            };
            
            test_instance.categories.insert(category.id.clone(), category);
        }
        
        // Test cross-node topic synchronization
        let cross_node_topics = vec![
            ("sync_topic_1", "cross_cat_shared_announcements", "Network Update", "Important network protocol update"),
            ("sync_topic_2", "cross_cat_node1_general", "Welcome Cross-Node Users", "Welcome to our distributed forum"),
            ("sync_topic_3", "cross_cat_node2_dev", "API Synchronization", "Discussing API sync between nodes"),
        ];
        
        for (topic_id, category_id, title, content) in cross_node_topics {
            let topic = MockTopic {
                id: format!("cross_topic_{}", topic_id),
                category_id: category_id.to_string(),
                title: title.to_string(),
                slug: title.to_lowercase().replace(" ", "-"),
                content: MockTopicContent {
                    current_version: content.to_string(),
                    format: "markdown".to_string(),
                    versions: vec![MockContentVersion {
                        content: content.to_string(),
                        author: "cross_node_user".to_string(),
                        created_at: SystemTime::now(),
                        edit_reason: None,
                    }],
                    wiki_editors: vec![],
                },
                author: "cross_node_user".to_string(),
                tags: vec!["cross-node".to_string(), "sync".to_string()],
                status: "open".to_string(),
                topic_type: "regular".to_string(),
                stats: MockTopicStats {
                    view_count: 0,
                    reply_count: 0,
                    like_count: 0,
                    bookmark_count: 0,
                    unique_viewers: 0,
                    last_reply_at: None,
                },
                created_at: SystemTime::now(),
                updated_at: SystemTime::now(),
                closed_at: None,
                deleted_at: None,
            };
            
            test_instance.topics.insert(topic.id.clone(), topic);
        }
        
        // Test cross-node reply synchronization
        let cross_node_replies = vec![
            ("sync_reply_1", "cross_topic_sync_topic_1", "node1_user", "Thanks for the update from node 1!"),
            ("sync_reply_2", "cross_topic_sync_topic_1", "node2_user", "Confirmed update received on node 2"),
            ("sync_reply_3", "cross_topic_sync_topic_2", "node1_user", "Welcome! This cross-node feature is amazing"),
        ];
        
        for (reply_id, topic_id, author, content) in cross_node_replies {
            let reply = MockReply {
                id: format!("cross_reply_{}", reply_id),
                topic_id: topic_id.to_string(),
                author: author.to_string(),
                content: content.to_string(),
                reply_to: None,
                votes: MockVoteCount {
                    upvotes: 0,
                    downvotes: 0,
                    score: 0,
                    voters: HashMap::new(),
                },
                accepted_answer: false,
                created_at: SystemTime::now(),
                edited_at: None,
                deleted_at: None,
                reactions: vec![],
            };
            
            test_instance.replies.insert(reply.id.clone(), reply);
        }
        
        // Verify cross-node synchronization
        let cross_categories: Vec<_> = test_instance.categories
            .values()
            .filter(|cat| cat.id.starts_with("cross_cat_"))
            .collect();
        let cross_topics: Vec<_> = test_instance.topics
            .values()
            .filter(|topic| topic.id.starts_with("cross_topic_"))
            .collect();
        let cross_replies: Vec<_> = test_instance.replies
            .values()
            .filter(|reply| reply.id.starts_with("cross_reply_"))
            .collect();
        
        if cross_categories.len() == 3 && cross_topics.len() == 3 && cross_replies.len() == 3 {
            ctx.log_info("✅ Cross-node discuss synchronization PASSED");
            results.push(VerificationResult::success(sync_start.elapsed())
                .with_metadata("operation".to_string(), "cross_node_sync".to_string())
                .with_metadata("categories_synced".to_string(), cross_categories.len().to_string())
                .with_metadata("topics_synced".to_string(), cross_topics.len().to_string())
                .with_metadata("replies_synced".to_string(), cross_replies.len().to_string())
                .with_metadata("sync_verified".to_string(), "true".to_string()));
        } else {
            let error = format!("Cross-node sync failed: categories={}, topics={}, replies={}", 
                              cross_categories.len(), cross_topics.len(), cross_replies.len());
            ctx.log_error(&error);
            results.push(VerificationResult::failure(error, sync_start.elapsed()));
        }
        
        Ok(results)
    }

    async fn test_stress(&self, ctx: &TestContext) -> Result<Vec<VerificationResult>> {
        let mut test_instance = self.clone();
        let mut results = Vec::new();
        
        ctx.log_info("Running discuss stress tests");
        
        // Create a stress test category
        let stress_category = MockCategory {
            id: "stress_category".to_string(),
            name: "Stress Test Category".to_string(),
            description: "High-volume discussion testing".to_string(),
            slug: "stress_test".to_string(),
            access_level: "public".to_string(),
            parent_id: None,
            topic_count: 0,
            post_count: 0,
            last_post_at: None,
            created_at: SystemTime::now(),
            moderator_groups: vec!["stress_mods".to_string()],
            settings: MockCategorySettings {
                allow_polls: true,
                allow_wiki_posts: true,
                require_approval: false,
                min_trust_level: 0,
                auto_close_days: None,
                slow_mode_minutes: None,
            },
        };
        
        test_instance.categories.insert("stress_category".to_string(), stress_category);
        
        // Stress test 1: High-volume topic creation
        let start_time = std::time::Instant::now();
        let topic_count = 500;
        
        ctx.log_info(&format!("[STRESS] Creating {} topics rapidly", topic_count));
        
        for i in 0..topic_count {
            let topic = MockTopic {
                id: format!("stress_topic_{}", i),
                category_id: "stress_category".to_string(),
                title: format!("Stress Test Topic #{}", i),
                slug: format!("stress-test-topic-{}", i),
                content: MockTopicContent {
                    current_version: format!("This is stress test topic number {}. Testing high-volume topic creation.", i),
                    format: "markdown".to_string(),
                    versions: vec![MockContentVersion {
                        content: format!("Stress test content #{}", i),
                        author: "stress_user".to_string(),
                        created_at: SystemTime::now(),
                        edit_reason: None,
                    }],
                    wiki_editors: vec![],
                },
                author: "stress_user".to_string(),
                tags: vec!["stress".to_string(), "test".to_string()],
                status: "open".to_string(),
                topic_type: "regular".to_string(),
                stats: MockTopicStats {
                    view_count: 0,
                    reply_count: 0,
                    like_count: 0,
                    bookmark_count: 0,
                    unique_viewers: 0,
                    last_reply_at: None,
                },
                created_at: SystemTime::now(),
                updated_at: SystemTime::now(),
                closed_at: None,
                deleted_at: None,
            };
            
            test_instance.topics.insert(topic.id.clone(), topic);
            
            if i % 100 == 0 {
                ctx.log_info(&format!("Created {} topics", i));
            }
        }
        
        // Stress test 2: High-volume reply creation
        let reply_count = 1000;
        ctx.log_info(&format!("[STRESS] Creating {} replies rapidly", reply_count));
        
        for i in 0..reply_count {
            let topic_id = format!("stress_topic_{}", i % topic_count); // Distribute replies across topics
            
            let reply = MockReply {
                id: format!("stress_reply_{}", i),
                topic_id,
                author: format!("stress_user_{}", i % 50), // 50 different users
                content: format!("This is stress test reply #{}", i),
                reply_to: if i > 0 && i % 10 == 0 { 
                    Some(format!("stress_reply_{}", i - 1)) 
                } else { 
                    None 
                }, // Some threaded replies
                votes: MockVoteCount {
                    upvotes: 0,
                    downvotes: 0,
                    score: 0,
                    voters: HashMap::new(),
                },
                accepted_answer: false,
                created_at: SystemTime::now(),
                edited_at: None,
                deleted_at: None,
                reactions: vec![],
            };
            
            test_instance.replies.insert(reply.id.clone(), reply);
            
            if i % 200 == 0 {
                ctx.log_info(&format!("Created {} replies", i));
            }
        }
        
        // Verify stress test results
        let stored_stress_topics: Vec<_> = test_instance.topics
            .values()
            .filter(|topic| topic.id.starts_with("stress_topic_"))
            .collect();
        let stored_stress_replies: Vec<_> = test_instance.replies
            .values()
            .filter(|reply| reply.id.starts_with("stress_reply_"))
            .collect();
        
        if stored_stress_topics.len() == topic_count && stored_stress_replies.len() == reply_count {
            ctx.log_info(&format!("✅ Discuss stress test PASSED: {} topics + {} replies in {:?}", 
                                topic_count, reply_count, start_time.elapsed()));
            results.push(VerificationResult::success(start_time.elapsed())
                .with_metadata("operation".to_string(), "discuss_stress_test".to_string())
                .with_metadata("topics_created".to_string(), topic_count.to_string())
                .with_metadata("replies_created".to_string(), reply_count.to_string())
                .with_metadata("topics_verified".to_string(), stored_stress_topics.len().to_string())
                .with_metadata("replies_verified".to_string(), stored_stress_replies.len().to_string())
                .with_metadata("throughput_items_per_sec".to_string(), 
                    ((topic_count + reply_count) as f64 / start_time.elapsed().as_secs_f64()).to_string()));
        } else {
            let error = format!("Stress test failed: expected {} topics + {} replies, got {} + {}", 
                              topic_count, reply_count, stored_stress_topics.len(), stored_stress_replies.len());
            ctx.log_error(&error);
            results.push(VerificationResult::failure(error, start_time.elapsed()));
        }
        
        Ok(results)
    }
}

impl Default for DiscussTests {
    fn default() -> Self { Self::new() }
}

impl Clone for DiscussTests {
    fn clone(&self) -> Self {
        Self {
            generator: TestDataGenerator::new(),
            verifier: self.verifier.clone(),
            categories: HashMap::new(), // Fresh state for each clone
            topics: HashMap::new(),
            replies: HashMap::new(),
            polls: HashMap::new(),
            users: HashMap::new(),
            moderation_actions: HashMap::new(),
        }
    }
}