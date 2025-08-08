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

//! TDD tests for Identity CLI commands
//!
//! Tests the command-line interface for identity management

use saorsa_core::Result;
use saorsa_core::identity::cli::{IdentityCliHandler, IdentityCommand};
use std::path::PathBuf;
use tempfile::TempDir;

#[cfg(test)]
mod cli_command_tests {
    use super::*;

    #[tokio::test]
    async fn test_cli_generate_command() {
        let temp_dir = TempDir::new().unwrap();
        let identity_path = temp_dir.path().join("identity.json");

        // Create CLI handler
        let handler = IdentityCliHandler::new(Some(identity_path.clone()));

        // Test generate command with custom difficulty
        let cmd = IdentityCommand::Generate {
            difficulty: Some(8),
            output: Some(identity_path.clone()),
        };

        let result = handler.execute(cmd).await.unwrap();

        // Should return success message with identity details
        assert!(result.contains("Generated new identity"));
        assert!(result.contains("Node ID:"));
        assert!(result.contains("Word Address:"));
        assert!(result.contains("Saved to:"));

        // File should exist
        assert!(identity_path.exists());
    }

    #[tokio::test]
    async fn test_cli_show_command() {
        let temp_dir = TempDir::new().unwrap();
        let identity_path = temp_dir.path().join("identity.json");

        let handler = IdentityCliHandler::new(Some(identity_path.clone()));

        // First generate an identity
        let gen_cmd = IdentityCommand::Generate {
            difficulty: Some(8),
            output: Some(identity_path.clone()),
        };
        handler.execute(gen_cmd).await.unwrap();

        // Now test show command
        let show_cmd = IdentityCommand::Show {
            path: Some(identity_path.clone()),
        };

        let result = handler.execute(show_cmd).await.unwrap();

        // Should display identity information
        assert!(result.contains("Identity Information"));
        assert!(result.contains("Node ID:"));
        assert!(result.contains("Word Address:"));
        assert!(result.contains("Public Key:"));
        assert!(result.contains("PoW Difficulty:"));
    }

    #[tokio::test]
    async fn test_cli_verify_command() {
        let temp_dir = TempDir::new().unwrap();
        let identity_path = temp_dir.path().join("identity.json");

        let handler = IdentityCliHandler::new(Some(identity_path.clone()));

        // Generate identity
        let gen_cmd = IdentityCommand::Generate {
            difficulty: Some(8),
            output: Some(identity_path.clone()),
        };
        handler.execute(gen_cmd).await.unwrap();

        // Test verify command
        let verify_cmd = IdentityCommand::Verify {
            path: Some(identity_path.clone()),
        };

        let result = handler.execute(verify_cmd).await.unwrap();

        // Should confirm validity
        assert!(result.contains("Identity is valid"));
        assert!(result.contains("✓ Proof of Work: Valid"));
        assert!(result.contains("✓ Cryptographic keys: Valid"));
        assert!(result.contains("✓ Word address: Matches"));
    }

    #[tokio::test]
    async fn test_cli_export_command() {
        let temp_dir = TempDir::new().unwrap();
        let identity_path = temp_dir.path().join("identity.json");
        let export_path = temp_dir.path().join("exported.json");

        let handler = IdentityCliHandler::new(Some(identity_path.clone()));

        // Generate identity
        let gen_cmd = IdentityCommand::Generate {
            difficulty: Some(8),
            output: Some(identity_path.clone()),
        };
        handler.execute(gen_cmd).await.unwrap();

        // Test export command
        let export_cmd = IdentityCommand::Export {
            path: Some(identity_path.clone()),
            output: export_path.clone(),
            format: ExportFormat::Json,
        };

        let result = handler.execute(export_cmd).await.unwrap();

        // Should confirm export
        assert!(result.contains("Identity exported"));
        assert!(export_path.exists());
    }

    #[tokio::test]
    async fn test_cli_sign_command() {
        let temp_dir = TempDir::new().unwrap();
        let identity_path = temp_dir.path().join("identity.json");
        let message_file = temp_dir.path().join("message.txt");

        // Create a test message
        tokio::fs::write(&message_file, b"Test message to sign")
            .await
            .unwrap();

        let handler = IdentityCliHandler::new(Some(identity_path.clone()));

        // Generate identity
        let gen_cmd = IdentityCommand::Generate {
            difficulty: Some(8),
            output: Some(identity_path.clone()),
        };
        handler.execute(gen_cmd).await.unwrap();

        // Test sign command
        let sign_cmd = IdentityCommand::Sign {
            identity: Some(identity_path.clone()),
            message: MessageInput::File(message_file),
            output: None,
        };

        let result = handler.execute(sign_cmd).await.unwrap();

        // Should return signature
        assert!(result.contains("Signature:"));
        assert!(result.contains("Message hash:"));
    }

    #[tokio::test]
    async fn test_cli_error_handling() {
        let handler = IdentityCliHandler::new(None);

        // Test show command with no identity
        let show_cmd = IdentityCommand::Show { path: None };
        let result = handler.execute(show_cmd).await;

        // Should return error
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No identity found")
        );
    }

    #[test]
    fn test_cli_argument_parsing() {
        use clap::Parser;

        // Test generate command parsing
        let args = vec!["identity", "generate", "--difficulty", "16"];
        let parsed = IdentityCommand::try_parse_from(args);
        assert!(parsed.is_ok());

        match parsed.unwrap() {
            IdentityCommand::Generate { difficulty, .. } => {
                assert_eq!(difficulty, Some(16));
            }
            _ => panic!("Wrong command parsed"),
        }

        // Test show command parsing
        let args = vec!["identity", "show", "--path", "/tmp/id.json"];
        let parsed = IdentityCommand::try_parse_from(args);
        assert!(parsed.is_ok());

        // Test invalid command
        let args = vec!["identity", "invalid-command"];
        let parsed = IdentityCommand::try_parse_from(args);
        assert!(parsed.is_err());
    }

    #[tokio::test]
    async fn test_cli_generate_with_seed() {
        let temp_dir = TempDir::new().unwrap();
        let identity_path = temp_dir.path().join("identity.json");

        let handler = IdentityCliHandler::new(Some(identity_path.clone()));

        // Test generate with seed
        let cmd = IdentityCommand::Generate {
            difficulty: Some(8),
            output: Some(identity_path.clone()),
            seed: Some("my-deterministic-seed-phrase".to_string()),
        };

        let result = handler.execute(cmd).await.unwrap();
        assert!(result.contains("Generated new identity"));

        // Generate again with same seed
        let identity_path2 = temp_dir.path().join("identity2.json");
        let cmd2 = IdentityCommand::Generate {
            difficulty: Some(8),
            output: Some(identity_path2.clone()),
            seed: Some("my-deterministic-seed-phrase".to_string()),
        };

        handler.execute(cmd2).await.unwrap();

        // Load both identities and compare
        let id1 = saorsa_core::identity::NodeIdentity::load_from_file(&identity_path)
            .await
            .unwrap();
        let id2 = saorsa_core::identity::NodeIdentity::load_from_file(&identity_path2)
            .await
            .unwrap();

        // Should be identical
        assert_eq!(id1.node_id(), id2.node_id());
        assert_eq!(
            id1.word_address().to_string(),
            id2.word_address().to_string()
        );
    }
}

#[cfg(test)]
mod cli_integration_tests {
    use super::*;
    use std::process::Command;

    #[test]
    #[ignore = "Requires saorsa binary to be built; run with `cargo test -- --ignored` after building"]
    fn test_cli_binary_integration() {
        // Test actual CLI binary if built
        let output = Command::new("saorsa")
            .args(&["identity", "--help"])
            .output()
            .expect("Failed to execute saorsa binary");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Identity management commands"));
        assert!(stdout.contains("generate"));
        assert!(stdout.contains("show"));
        assert!(stdout.contains("verify"));
    }
}

// Helper types for CLI testing

#[derive(Debug)]
enum MessageInput {
    Text(String),
    File(PathBuf),
}

#[derive(Debug)]
enum ExportFormat {
    Json,
    Base64,
    Hex,
}
