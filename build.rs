use std::process::Command;
use std::path::Path;

fn main() {
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