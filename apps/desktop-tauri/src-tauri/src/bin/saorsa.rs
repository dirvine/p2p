use anyhow::Result;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// Include the generated frontend bundle when the feature is enabled
#[cfg(feature = "bundle-frontend")]
include!(concat!(env!("OUT_DIR"), "/frontend_bundle_generated.rs"));

// Provide empty constants when not bundling
#[cfg(not(feature = "bundle-frontend"))]
const INDEX_HTML: &str = "";
#[cfg(not(feature = "bundle-frontend"))]
const STYLES_CSS: &str = "";
#[cfg(not(feature = "bundle-frontend"))]
const MAIN_JS: &str = "";

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
    
    #[cfg(feature = "bundle-frontend")]
    {
        // Write files from the embedded strings (defined at module level)
        fs::write(target_dir.join("index.html"), INDEX_HTML)?;
        fs::write(target_dir.join("styles.css"), STYLES_CSS)?;
        fs::write(target_dir.join("main.js"), MAIN_JS)?;
        
        println!("Extracted frontend assets to {:?}", target_dir);
    }
    
    #[cfg(not(feature = "bundle-frontend"))]
    {
        return Err(anyhow::anyhow!("Frontend assets not bundled. Please install from source or use the pre-built binaries."));
    }
    
    Ok(())
}