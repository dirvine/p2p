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
    let src_dir = Path::new("../src");
    let generated_path = Path::new(&out_dir).join("frontend_bundle_generated.rs");
    
    let mut content = Vec::new();
    content.push("/// Auto-generated frontend bundle".to_string());
    content.push("/// This file embeds the complete frontend application".to_string());
    content.push("".to_string());
    
    // Read files or use defaults
    let index_html = if src_dir.join("index.html").exists() {
        fs::read_to_string(src_dir.join("index.html"))
            .unwrap_or_else(|_| include_str!("../src/index.html").to_string())
    } else {
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Saorsa</title>
    <meta charset="UTF-8">
    <style>
        body { 
            font-family: system-ui; 
            display: flex; 
            align-items: center; 
            justify-content: center; 
            height: 100vh; 
            margin: 0;
            background: #f0f0f0;
        }
        .loading { 
            text-align: center;
            padding: 2rem;
            background: white;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
    </style>
</head>
<body>
    <div class="loading">
        <h1>🕊️ Saorsa</h1>
        <p>P2P Messaging Application</p>
        <p>Please build from source for the full experience.</p>
    </div>
</body>
</html>"#.to_string()
    };
    
    let styles_css = if src_dir.join("styles.css").exists() {
        fs::read_to_string(src_dir.join("styles.css"))
            .unwrap_or_else(|_| "/* Saorsa styles */".to_string())
    } else {
        "/* Saorsa styles - minimal */".to_string()
    };
    
    let main_js = if src_dir.join("main.js").exists() {
        fs::read_to_string(src_dir.join("main.js"))
            .unwrap_or_else(|_| "console.log('Saorsa');".to_string())
    } else {
        "console.log('Saorsa v0.2.2 - Minimal frontend');".to_string()
    };
    
    // Escape the content for inclusion in Rust source
    let index_escaped = index_html
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    
    let styles_escaped = styles_css
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    
    let main_escaped = main_js
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    
    // Generate the Rust code
    content.push(format!(r#"pub const INDEX_HTML: &str = "{}";"#, index_escaped));
    content.push("".to_string());
    content.push(format!(r#"pub const STYLES_CSS: &str = "{}";"#, styles_escaped));
    content.push("".to_string());
    content.push(format!(r#"pub const MAIN_JS: &str = "{}";"#, main_escaped));
    
    // Write the generated file
    let generated_content = content.join("\n");
    fs::write(&generated_path, generated_content)
        .expect("Failed to write generated frontend bundle");
    
    println!("cargo:warning=Generated frontend bundle at {:?}", generated_path);
}