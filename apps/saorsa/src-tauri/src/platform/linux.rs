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

// src/platform/linux.rs
use anyhow::Result;
use std::process::Command;

#[derive(Debug)]
pub struct LinuxAuth;

impl LinuxAuth {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
    
    pub async fn is_available(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            // Check if polkit is available
            Command::new("which")
                .arg("pkexec")
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }
    
    pub async fn authenticate(&self, reason: &str) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            tracing::info!("Linux authentication requested: {}", reason);
            
            // For now, simulate authentication
            // In production, this would use polkit or another system authentication
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            
            // Simple check that doesn't require actual system authentication for demo
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(anyhow::anyhow!("Linux authentication not available on this platform"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_linux_auth_creation() {
        let auth = LinuxAuth::new();
        assert!(auth.is_ok());
    }
    
    #[tokio::test]
    async fn test_linux_auth_availability() {
        let auth = LinuxAuth::new().unwrap();
        let available = auth.is_available().await;
        
        #[cfg(target_os = "linux")]
        println!("Linux auth available: {}", available);
        
        #[cfg(not(target_os = "linux"))]
        assert!(!available);
    }
}