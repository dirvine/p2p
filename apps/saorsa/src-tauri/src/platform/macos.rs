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

//! # macOS Platform Authentication Module
//! 
//! Provides TouchID biometric authentication for macOS using
//! the LocalAuthentication framework through Objective-C bindings.
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{info, warn, error};

#[cfg(target_os = "macos")]
use {
    objc::{msg_send, sel, sel_impl, class},
    objc::runtime::{Object, BOOL, YES, NO},
    objc_foundation::{INSString, NSString},
    objc_id::Id,
    block::{Block, ConcreteBlock},
    dispatch::Queue,
    core_foundation::base::{CFRelease, TCFType},
    std::os::raw::c_void,
};

/// TouchID authentication handler for macOS
/// 
/// Uses LocalAuthentication framework to provide
/// biometric authentication on supported Mac devices.
#[derive(Debug)]
pub struct TouchIdAuth {
    #[cfg(target_os = "macos")]
    available: bool,
}

impl TouchIdAuth {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            // Check if biometric authentication is available
            let available = unsafe {
                let la_context_class = class!(LAContext);
                let context: *mut Object = msg_send![la_context_class, new];
                
                if context.is_null() {
                    false
                } else {
                    let policy = 1i64; // LAPolicyDeviceOwnerAuthenticationWithBiometrics
                    let error: *mut Object = std::ptr::null_mut();
                    let can_evaluate: BOOL = msg_send![context, canEvaluatePolicy:policy error:&error];
                    
                    // Clean up
                    let _: () = msg_send![context, release];
                    
                    can_evaluate == YES
                }
            };
            
            Ok(Self { available })
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(Self {})
        }
    }
    
    pub async fn is_available(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.available
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
    
    pub async fn authenticate(&self, reason: &str) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            if !self.available {
                return Err(anyhow::anyhow!("TouchID not available on this device"));
            }
            
            info!("TouchID authentication requested: {}", reason);
            
            // Create a channel for the result
            let (tx, rx) = oneshot::channel::<Result<()>>();
            let tx = Arc::new(std::sync::Mutex::new(Some(tx)));
            
            // Perform authentication on main thread
            let reason_str = reason.to_string();
            Queue::main().exec_async(move || {
                unsafe {
                    // Create LAContext
                    let la_context_class = class!(LAContext);
                    let context: *mut Object = msg_send![la_context_class, new];
                    
                    if context.is_null() {
                        if let Some(tx) = tx.lock().unwrap().take() {
                            let _ = tx.send(Err(anyhow::anyhow!("Failed to create LAContext")));
                        }
                        return;
                    }
                    
                    // Set localized reason
                    let ns_reason = NSString::from_str(&reason_str);
                    let policy = 1i64; // LAPolicyDeviceOwnerAuthenticationWithBiometrics
                    
                    // Create completion block
                    let tx_clone = tx.clone();
                    let block = ConcreteBlock::new(move |success: BOOL, error: *mut Object| {
                        let result = if success == YES {
                            Ok(())
                        } else {
                            let error_msg = if !error.is_null() {
                                let description: *mut Object = msg_send![error, localizedDescription];
                                let desc_str: &NSString = &*(description as *const NSString);
                                desc_str.as_str().to_string()
                            } else {
                                "Unknown authentication error".to_string()
                            };
                            Err(anyhow::anyhow!("TouchID authentication failed: {}", error_msg))
                        };
                        
                        if let Some(tx) = tx_clone.lock().unwrap().take() {
                            let _ = tx.send(result);
                        }
                    });
                    let block = block.copy();
                    
                    // Evaluate policy
                    let _: () = msg_send![context,
                        evaluatePolicy:policy
                        localizedReason:&*ns_reason
                        reply:&*block
                    ];
                    
                    // Context will be released when block completes
                }
            });
            
            // Wait for authentication result
            match rx.await {
                Ok(result) => result,
                Err(_) => Err(anyhow::anyhow!("Authentication was cancelled")),
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(anyhow::anyhow!("TouchID not available on this platform"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_touchid_creation() {
        let auth = TouchIdAuth::new();
        assert!(auth.is_ok());
    }
    
    #[tokio::test]
    async fn test_touchid_availability() {
        let auth = TouchIdAuth::new().unwrap();
        let available = auth.is_available().await;
        
        // This will vary by platform
        #[cfg(target_os = "macos")]
        println!("TouchID available: {}", available);
        
        #[cfg(not(target_os = "macos"))]
        assert!(!available);
    }
}