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

use anyhow::Result;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// Use the embedded frontend bundle

fn main() -> Result<()> {
    // Check if we need to extract bundled frontend
    let frontend_dir = get_frontend_dir()?;
    
    // Set the TAURI_FRONTEND_DIR environment variable
    env::set_var("TAURI_FRONTEND_DIR", &frontend_dir);
    
    // Run the actual Tauri app
    saorsa_lib::run_desktop_app()
}

fn get_frontend_dir() -> Result<PathBuf> {
    // First check if we're running from source (development)
    let dev_frontend = Path::new("../src");
    if dev_frontend.exists() && dev_frontend.join("index.html").exists() {
        return Ok(dev_frontend.canonicalize()?);
    }
    
    // Check if frontend is already extracted
    let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let saorsa_dir = home_dir.join(".saorsa");
    let frontend_dir = saorsa_dir.join("frontend");
    
    // If frontend doesn't exist, extract it from bundled data
    if !frontend_dir.join("index.html").exists() {
        extract_bundled_frontend(&frontend_dir)?;
    }
    
    Ok(frontend_dir)
}

fn extract_bundled_frontend(target_dir: &Path) -> Result<()> {
    // Create target directory
    fs::create_dir_all(target_dir)?;
    
    // Write files from the embedded strings in the library
    fs::write(target_dir.join("index.html"), saorsa_lib::frontend_bundle::INDEX_HTML)?;
    fs::write(target_dir.join("styles.css"), saorsa_lib::frontend_bundle::STYLES_CSS)?;
    fs::write(target_dir.join("main.js"), saorsa_lib::frontend_bundle::MAIN_JS)?;
    
    println!("🕊️ Saorsa v0.2.7 - Extracted frontend assets to {:?}", target_dir);
    Ok(())
}