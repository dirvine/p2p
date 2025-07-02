// src/platform/windows.rs
use anyhow::Result;
use tracing::{info, warn, error};

#[cfg(target_os = "windows")]
use {
    windows::{
        core::*,
        Security::Credentials::UI::*,
        Foundation::*,
    },
    tokio::sync::oneshot,
};

#[derive(Debug)]
pub struct WindowsHelloAuth {
    #[cfg(target_os = "windows")]
    available: bool,
}

impl WindowsHelloAuth {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "windows")]
        {
            // Check if Windows Hello is available
            let available = match UserConsentVerifier::CheckAvailabilityAsync() {
                Ok(operation) => {
                    match operation.get() {
                        Ok(availability) => {
                            matches!(
                                availability,
                                UserConsentVerifierAvailability::Available |
                                UserConsentVerifierAvailability::DeviceBusy
                            )
                        }
                        Err(e) => {
                            warn!("Failed to check Windows Hello availability: {:?}", e);
                            false
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to create availability check: {:?}", e);
                    false
                }
            };
            
            Ok(Self { available })
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(Self {})
        }
    }
    
    pub async fn is_available(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            self.available
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }
    
    pub async fn verify_user(&self, message: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            if !self.available {
                return Err(anyhow::anyhow!("Windows Hello not available on this device"));
            }
            
            info!("Windows Hello authentication requested: {}", message);
            
            // Convert message to HSTRING
            let h_message = HSTRING::from(message);
            
            // Create a channel for the result
            let (tx, rx) = oneshot::channel::<Result<()>>();
            
            // Perform authentication
            tokio::task::spawn_blocking(move || {
                let result = match UserConsentVerifier::RequestVerificationAsync(&h_message) {
                    Ok(operation) => {
                        match operation.get() {
                            Ok(consent_result) => {
                                match consent_result {
                                    UserConsentVerificationResult::Verified => {
                                        info!("Windows Hello authentication successful");
                                        Ok(())
                                    }
                                    UserConsentVerificationResult::DeviceNotPresent => {
                                        Err(anyhow::anyhow!("No biometric device present"))
                                    }
                                    UserConsentVerificationResult::NotConfiguredForUser => {
                                        Err(anyhow::anyhow!("Windows Hello not configured for this user"))
                                    }
                                    UserConsentVerificationResult::DisabledByPolicy => {
                                        Err(anyhow::anyhow!("Windows Hello disabled by policy"))
                                    }
                                    UserConsentVerificationResult::DeviceBusy => {
                                        Err(anyhow::anyhow!("Biometric device is busy"))
                                    }
                                    UserConsentVerificationResult::RetriesExhausted => {
                                        Err(anyhow::anyhow!("Too many failed attempts"))
                                    }
                                    UserConsentVerificationResult::Canceled => {
                                        Err(anyhow::anyhow!("Authentication cancelled by user"))
                                    }
                                    _ => {
                                        Err(anyhow::anyhow!("Unknown authentication error"))
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to get verification result: {:?}", e);
                                Err(anyhow::anyhow!("Failed to get verification result"))
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to request verification: {:?}", e);
                        Err(anyhow::anyhow!("Failed to request Windows Hello verification"))
                    }
                };
                
                let _ = tx.send(result);
            });
            
            // Wait for authentication result
            match rx.await {
                Ok(result) => result,
                Err(_) => Err(anyhow::anyhow!("Authentication task failed")),
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow::anyhow!("Windows Hello not available on this platform"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_windows_hello_creation() {
        let auth = WindowsHelloAuth::new();
        assert!(auth.is_ok());
    }
    
    #[tokio::test]
    async fn test_windows_hello_availability() {
        let auth = WindowsHelloAuth::new().unwrap();
        let available = auth.is_available().await;
        
        #[cfg(target_os = "windows")]
        println!("Windows Hello available: {}", available);
        
        #[cfg(not(target_os = "windows"))]
        assert!(!available);
    }
}