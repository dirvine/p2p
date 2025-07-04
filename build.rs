// Copyright 2024 MaidSafe Limited
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

use std::process::Command;
use std::path::Path;
use std::env;
use std::fs;
use std::io::Write;

fn main() {
    // License-related rerun triggers
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=P2P_LICENSE_PATH");
    println!("cargo:rerun-if-env-changed=P2P_LICENSE_KEY");
    println!("cargo:rerun-if-env-changed=P2P_ENFORCE_LICENSE");
    
    // Handle license configuration
    configure_licensing();
    
    // Flutter-related rerun triggers
    println!("cargo:rerun-if-changed=apps/ant-connect/lib/main.dart");
    println!("cargo:rerun-if-changed=apps/ant-connect/pubspec.yaml");
    
    // Check if Flutter is available
    let flutter_available = Command::new("flutter")
        .arg("--version")
        .output()
        .is_ok();
    
    if flutter_available {
        let flutter_app_path = Path::new("apps/ant-connect");
        
        if flutter_app_path.exists() {
            println!("cargo:warning=Building Flutter web app...");
            
            let output = Command::new("flutter")
                .arg("build")
                .arg("web")
                .arg("--release")
                .current_dir(flutter_app_path)
                .output();
                
            match output {
                Ok(output) => {
                    if output.status.success() {
                        println!("cargo:warning=Flutter web build successful");
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        println!("cargo:warning=Flutter build failed: {}", stderr);
                    }
                }
                Err(e) => {
                    println!("cargo:warning=Failed to run flutter build: {}", e);
                }
            }
        } else {
            println!("cargo:warning=Flutter app directory not found");
        }
    } else {
        println!("cargo:warning=Flutter not available, skipping web build");
    }
}

fn configure_licensing() {
    // Check if we're building with commercial features
    let commercial_feature = env::var("CARGO_FEATURE_COMMERCIAL").is_ok();
    let agpl_feature = env::var("CARGO_FEATURE_AGPL_COMPLIANCE").is_ok();
    
    if commercial_feature && agpl_feature {
        panic!("Cannot enable both 'commercial' and 'agpl-compliance' features");
    }
    
    // Validate commercial license if commercial feature is enabled
    if commercial_feature {
        validate_commercial_license();
    }
    
    // Set configuration based on license type
    if commercial_feature {
        println!("cargo:rustc-cfg=license_type=\"commercial\"");
        println!("cargo:rustc-cfg=commercial_optimizations");
    } else {
        println!("cargo:rustc-cfg=license_type=\"agpl\"");
        if !agpl_feature {
            println!("cargo:warning=Building without explicit license feature. Defaulting to AGPL-3.0.");
        }
    }
    
    // Generate license header file
    generate_license_header();
}

fn validate_commercial_license() {
    // Check for license in environment or file
    let has_license_path = env::var("P2P_LICENSE_PATH").is_ok();
    let has_license_key = env::var("P2P_LICENSE_KEY").is_ok();
    let has_license_file = Path::new("license.json").exists();
    
    if !has_license_path && !has_license_key && !has_license_file {
        // Check if we should enforce licensing
        if env::var("P2P_ENFORCE_LICENSE").unwrap_or_default() == "true" {
            panic!(
                "\n\nCommercial license required!\n\
                 \n\
                 The 'commercial' feature is enabled but no license was found.\n\
                 \n\
                 Please provide a license using one of:\n\
                 1. P2P_LICENSE_PATH environment variable\n\
                 2. P2P_LICENSE_KEY environment variable\n\
                 3. license.json file in project root\n\
                 \n\
                 To purchase a commercial license: saorsalabs@gmail.com\n\
                 To disable enforcement (dev only): P2P_ENFORCE_LICENSE=false\n\n"
            );
        } else {
            println!("cargo:warning=Commercial feature enabled without license. Development mode only!");
        }
    }
}

fn generate_license_header() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let header_path = Path::new(&out_dir).join("license_header.rs");
    
    let header_content = if env::var("CARGO_FEATURE_COMMERCIAL").is_ok() {
        r#"
//! Auto-generated license header
pub const LICENSE_TYPE: &str = "Commercial";
pub const LICENSE_NOTICE: &str = "Copyright 2024 MaidSafe Limited - Commercial License";
pub const LICENSE_URL: &str = "https://maidsafe.net/licensing";
"#
    } else {
        r#"
//! Auto-generated license header
pub const LICENSE_TYPE: &str = "AGPL-3.0-or-later";
pub const LICENSE_NOTICE: &str = "Copyright 2024 MaidSafe Limited - AGPL-3.0-or-later";
pub const LICENSE_URL: &str = "https://www.gnu.org/licenses/agpl-3.0.html";
"#
    };
    
    let mut file = fs::File::create(&header_path).unwrap();
    file.write_all(header_content.as_bytes()).unwrap();
}