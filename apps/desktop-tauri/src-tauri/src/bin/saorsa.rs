use anyhow::Result;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// Include the generated frontend bundle when the feature is enabled
#[cfg(feature = "bundle-frontend")]
include!(concat!(env!("OUT_DIR"), "/frontend_bundle_generated.rs"));

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
    #[cfg(feature = "bundle-frontend")]
    {
        // Create target directory
        fs::create_dir_all(target_dir)?;
        
        // Write files from the embedded strings
        fs::write(target_dir.join("index.html"), INDEX_HTML)?;
        fs::write(target_dir.join("styles.css"), STYLES_CSS)?;
        fs::write(target_dir.join("main.js"), MAIN_JS)?;
        
        println!("🕊️ Saorsa v0.2.2 - Extracted frontend assets to {:?}", target_dir);
        Ok(())
    }
    
    #[cfg(not(feature = "bundle-frontend"))]
    {
        // For development builds, check if frontend exists in the source directory
        let src_frontend = Path::new("../src");
        if src_frontend.exists() {
            // Copy from source
            fs::create_dir_all(target_dir)?;
            fs::copy(src_frontend.join("index.html"), target_dir.join("index.html"))?;
            fs::copy(src_frontend.join("styles.css"), target_dir.join("styles.css"))?;
            fs::copy(src_frontend.join("main.js"), target_dir.join("main.js"))?;
            println!("🕊️ Saorsa v0.2.2 - Copied frontend assets from source");
            Ok(())
        } else {
            eprintln!();
            eprintln!("╭─────────────────────────────────────────────────────────────────╮");
            eprintln!("│                         🕊️ Saorsa v0.2.2                        │");
            eprintln!("├─────────────────────────────────────────────────────────────────┤");
            eprintln!("│                                                                 │");
            eprintln!("│  Frontend assets not found. Running from source?               │");
            eprintln!("│                                                                 │");
            eprintln!("│  Make sure you're in the correct directory:                    │");
            eprintln!("│     cd p2p/apps/desktop-tauri/src-tauri                       │");
            eprintln!("│                                                                 │");
            eprintln!("│  Or install the bundled version from crates.io:               │");
            eprintln!("│     cargo install saorsa --features bundle-frontend           │");
            eprintln!("│                                                                 │");
            eprintln!("╰─────────────────────────────────────────────────────────────────╯");
            eprintln!();
            
            Err(anyhow::anyhow!("Frontend assets not found"))
        }
    }
}