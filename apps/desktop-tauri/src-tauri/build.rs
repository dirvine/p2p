use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Handle frontend for packaging scenarios
    prepare_frontend_for_packaging();
    
    // Only bundle frontend when building for crates.io publication
    if env::var("CARGO_FEATURE_BUNDLE_FRONTEND").is_ok() || env::var("BUNDLE_FRONTEND").is_ok() {
        bundle_frontend_assets();
    }
    
    tauri_build::build()
}

fn prepare_frontend_for_packaging() {
    // When packaging for crates.io, the ../src directory doesn't exist
    // So we create a minimal one to satisfy Tauri's compile-time checks
    let src_dir = Path::new("../src");
    
    if !src_dir.exists() {
        println!("cargo:warning=Creating minimal frontend directory for packaging");
        
        // Create the directory
        fs::create_dir_all(src_dir).ok();
        
        // Create minimal files
        fs::write(src_dir.join("index.html"), 
            r#"<!DOCTYPE html><html><head><title>Saorsa</title></head><body>Loading...</body></html>"#
        ).ok();
        
        fs::write(src_dir.join("styles.css"), "/* Saorsa styles */").ok();
        fs::write(src_dir.join("main.js"), "// Saorsa main").ok();
    }
}

fn bundle_frontend_assets() {
    println!("cargo:rerun-if-changed=../src");
    
    let out_dir = env::var("OUT_DIR").unwrap();
    
    // Generate a Rust file with embedded frontend content
    let src_dir = Path::new("../src");
    let generated_path = Path::new(&out_dir).join("frontend_bundle_generated.rs");
    
    let mut generated_content = String::new();
    generated_content.push_str("/// Auto-generated frontend bundle\n\n");
    
    if src_dir.exists() {
        // Read and embed the actual frontend files
        if let Ok(index_html) = fs::read_to_string(src_dir.join("index.html")) {
            generated_content.push_str(&format!(
                "pub const INDEX_HTML: &str = r###\"{}\"###;\n\n", 
                index_html
            ));
        }
        
        if let Ok(styles_css) = fs::read_to_string(src_dir.join("styles.css")) {
            generated_content.push_str(&format!(
                "pub const STYLES_CSS: &str = r###\"{}\"###;\n\n", 
                styles_css
            ));
        }
        
        if let Ok(main_js) = fs::read_to_string(src_dir.join("main.js")) {
            generated_content.push_str(&format!(
                "pub const MAIN_JS: &str = r###\"{}\"###;\n\n", 
                main_js
            ));
        }
    } else {
        // Package environment - use minimal content
        println!("cargo:warning=Running in package environment, using minimal frontend");
        generated_content.push_str("pub const INDEX_HTML: &str = r###\"<!DOCTYPE html><html><head><title>Saorsa</title></head><body><div id='app'>Loading Saorsa...</div></body></html>\"###;\n\n");
        generated_content.push_str("pub const STYLES_CSS: &str = r###\"/* Saorsa styles */\"###;\n\n");
        generated_content.push_str("pub const MAIN_JS: &str = r###\"// Saorsa main\"###;\n\n");
    }
    
    // Write the generated file
    fs::write(&generated_path, generated_content)
        .expect("Failed to write generated frontend bundle");
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}
