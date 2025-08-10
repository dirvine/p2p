// Copyright 2025 Saorsa Labs Limited
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Profile validation for Four-Word DNS system

#![allow(unused_variables)]

use anyhow::Result;
use crate::identity::FourWordAddress;
use super::{FourWordProfile, ProfileContent};

/// Validator for Four-Word profiles
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProfileValidator {
    max_website_size: usize,
    max_blog_size: usize,
    signature_required: bool,
}

/// Validation result with detailed feedback
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ProfileValidator {
    /// Create a new profile validator with default settings
    pub fn new() -> Self {
        Self {
            max_website_size: 10 * 1024 * 1024, // 10MB
            max_blog_size: 10 * 1024 * 1024,    // 10MB
            signature_required: true,
        }
    }
    
    /// Create validator with custom size limits
    pub fn with_size_limits(max_website: usize, max_blog: usize) -> Self {
        Self {
            max_website_size: max_website,
            max_blog_size: max_blog,
            signature_required: true,
        }
    }
    
    /// Create validator without signature requirement (for testing)
    pub fn without_signature_requirement() -> Self {
        Self {
            max_website_size: 10 * 1024 * 1024,
            max_blog_size: 10 * 1024 * 1024,
            signature_required: false,
        }
    }
    
    /// Validate a complete profile
    pub fn validate_profile(&self, profile: &FourWordProfile) -> Result<ValidationResult> {
        let mut result = ValidationResult::valid();
        
        // Validate the profile's internal integrity first
        if let Err(e) = profile.validate() {
            result.add_error(format!("Profile integrity check failed: {}", e));
        }
        
        // Check signature requirement
        if self.signature_required && profile.signature.is_empty() {
            result.add_error("Profile signature is required but missing".to_string());
        }
        
        // Validate four-word address
        if let Err(e) = self.validate_four_words(&profile.four_words) {
            result.add_error(format!("Four-word validation failed: {}", e));
        }
        
        // Check content size limits
        if let Some(website) = profile.get_website() {
            if website.len() > self.max_website_size {
                result.add_error(format!(
                    "Website content exceeds limit: {} > {} bytes",
                    website.len(),
                    self.max_website_size
                ));
            }
        }
        
        if let Some(blog) = profile.get_blog() {
            if blog.len() > self.max_blog_size {
                result.add_error(format!(
                    "Blog content exceeds limit: {} > {} bytes",
                    blog.len(),
                    self.max_blog_size
                ));
            }
        }
        
        // Validate crypto addresses if present
        if let Some(bitcoin) = profile.get_bitcoin_address() {
            if let Err(e) = self.validate_bitcoin_address(bitcoin) {
                result.add_warning(format!("Bitcoin address validation warning: {}", e));
            }
        }
        
        if let Some(ethereum) = profile.get_ethereum_address() {
            if let Err(e) = self.validate_ethereum_address(ethereum) {
                result.add_warning(format!("Ethereum address validation warning: {}", e));
            }
        }
        
        // Check for malicious content patterns
        if let Ok(malicious_check) = self.check_malicious_content(profile) {
            if !malicious_check.is_valid {
                result.merge(malicious_check);
            }
        }
        
        Ok(result)
    }
    
    /// Validate four-word address format
    pub fn validate_four_words(&self, four_words: &FourWordAddress) -> Result<bool> {
        // Get the individual words
        let words = four_words.words();
        
        // Check that we have exactly 4 words
        if words.len() != 4 {
            return Ok(false);
        }
        
        // Check that none of the words are empty
        if words.iter().any(|w| w.is_empty()) {
            return Ok(false);
        }
        
        // Check that words contain only valid characters (alphabetic and hyphens)
        let is_valid_word = |word: &str| -> bool {
            word.chars().all(|c| c.is_alphabetic())
        };
        
        if !words.iter().all(|w| is_valid_word(w)) {
            return Ok(false);
        }
        
        // Check reasonable word length (between 2 and 20 characters)
        let is_valid_length = |word: &str| -> bool {
            word.len() >= 2 && word.len() <= 20
        };
        
        if !words.iter().all(|w| is_valid_length(w)) {
            return Ok(false);
        }
        
        // Use the built-in validation
        Ok(four_words.is_valid())
    }
    
    /// Validate Bitcoin address format
    pub fn validate_bitcoin_address(&self, address: &str) -> Result<bool> {
        // Basic Bitcoin address validation
        // P2PKH addresses start with 1
        // P2SH addresses start with 3
        // Bech32 addresses start with bc1 (mainnet) or tb1 (testnet)
        
        if address.is_empty() {
            return Ok(false);
        }
        
        // Check length constraints
        if address.len() < 26 || address.len() > 90 {
            return Ok(false);
        }
        
        // Check address format
        if address.starts_with('1') || address.starts_with('3') {
            // Legacy/P2SH address - should be base58
            let valid_chars = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
            Ok(address.chars().all(|c| valid_chars.contains(c)))
        } else if address.starts_with("bc1") || address.starts_with("tb1") {
            // Bech32 address
            let valid_chars = "023456789acdefghjklmnpqrstuvwxyz";
            let addr_part = &address[3..];
            Ok(addr_part.chars().all(|c| valid_chars.contains(c)))
        } else {
            Ok(false)
        }
    }
    
    /// Validate Ethereum address format
    pub fn validate_ethereum_address(&self, address: &str) -> Result<bool> {
        // Ethereum addresses are 42 characters (0x + 40 hex chars)
        if !address.starts_with("0x") {
            return Ok(false);
        }
        
        if address.len() != 42 {
            return Ok(false);
        }
        
        // Check that remaining characters are valid hex
        let hex_part = &address[2..];
        Ok(hex_part.chars().all(|c| c.is_ascii_hexdigit()))
    }
    
    /// Validate markdown content
    pub fn validate_markdown(&self, content: &str) -> Result<bool> {
        // Basic markdown validation
        // Check for common XSS patterns
        let dangerous_patterns = [
            "<script",
            "javascript:",
            "onerror=",
            "onclick=",
            "onload=",
            "<iframe",
            "<embed",
            "<object",
            "data:text/html",
        ];
        
        let lower_content = content.to_lowercase();
        for pattern in &dangerous_patterns {
            if lower_content.contains(pattern) {
                return Ok(false);
            }
        }
        
        // Check for reasonable content (not empty, not too short)
        if content.trim().is_empty() {
            return Ok(false);
        }
        
        // Basic markdown structure validation
        // Allow common markdown elements
        Ok(true)
    }
    
    /// Check content size limits
    pub fn check_size_limits(&self, profile: &FourWordProfile) -> Result<ValidationResult> {
        let mut result = ValidationResult::valid();
        
        // Check website size
        if let Some(website) = profile.get_website() {
            if website.len() > self.max_website_size {
                result.add_error(format!(
                    "Website content exceeds maximum size: {} bytes (limit: {} bytes)",
                    website.len(),
                    self.max_website_size
                ));
            } else if website.len() > self.max_website_size * 80 / 100 {
                result.add_warning(format!(
                    "Website content is approaching size limit: {} bytes (80% of {} bytes)",
                    website.len(),
                    self.max_website_size
                ));
            }
        }
        
        // Check blog size
        if let Some(blog) = profile.get_blog() {
            if blog.len() > self.max_blog_size {
                result.add_error(format!(
                    "Blog content exceeds maximum size: {} bytes (limit: {} bytes)",
                    blog.len(),
                    self.max_blog_size
                ));
            } else if blog.len() > self.max_blog_size * 80 / 100 {
                result.add_warning(format!(
                    "Blog content is approaching size limit: {} bytes (80% of {} bytes)",
                    blog.len(),
                    self.max_blog_size
                ));
            }
        }
        
        // Check total content size
        let total_size: usize = profile.content.iter().map(|c| {
            match c {
                ProfileContent::Website(s) | ProfileContent::Blog(s) => s.len(),
                ProfileContent::BitcoinAddress(s) | ProfileContent::EthereumAddress(s) => s.len(),
                ProfileContent::CustomData(k, v) => k.len() + v.len(),
            }
        }).sum();
        
        let max_total = self.max_website_size + self.max_blog_size;
        if total_size > max_total {
            result.add_error(format!(
                "Total content size exceeds limit: {} bytes (limit: {} bytes)",
                total_size,
                max_total
            ));
        }
        
        Ok(result)
    }
    
    /// Validate profile signature
    pub fn validate_signature(&self, profile: &FourWordProfile, public_key: &[u8]) -> Result<bool> {
        // If no signature is present and not required, that's valid
        if profile.signature.is_empty() && !self.signature_required {
            return Ok(true);
        }
        
        // If signature is required but missing, that's invalid
        if profile.signature.is_empty() && self.signature_required {
            return Ok(false);
        }
        
        // Verify the signature using the profile's verify method
        profile.verify_signature(public_key)
    }
    
    /// Check for malicious content patterns
    pub fn check_malicious_content(&self, profile: &FourWordProfile) -> Result<ValidationResult> {
        let mut result = ValidationResult::valid();
        
        // Define dangerous patterns to check for
        let dangerous_patterns = [
            "<script", "javascript:", "onerror=", "onclick=", "onload=",
            "<iframe", "<embed", "<object", "data:text/html",
            "eval(", "document.cookie", "window.location",
            "../", "..", "file://", "data:application",
        ];
        
        // Check website content
        if let Some(website) = profile.get_website() {
            let lower_website = website.to_lowercase();
            for pattern in &dangerous_patterns {
                if lower_website.contains(pattern) {
                    result.add_error(format!("Potentially malicious pattern found in website: {}", pattern));
                }
            }
        }
        
        // Check blog content
        if let Some(blog) = profile.get_blog() {
            let lower_blog = blog.to_lowercase();
            for pattern in &dangerous_patterns {
                if lower_blog.contains(pattern) {
                    result.add_error(format!("Potentially malicious pattern found in blog: {}", pattern));
                }
            }
        }
        
        // Check custom data
        for content in &profile.content {
            if let ProfileContent::CustomData(key, value) = content {
                let lower_value = value.to_lowercase();
                for pattern in &dangerous_patterns {
                    if lower_value.contains(pattern) {
                        result.add_error(format!("Potentially malicious pattern found in custom data '{}': {}", key, pattern));
                    }
                }
            }
        }
        
        // Warn about suspicious URLs
        let url_patterns = ["http://", "https://", "ftp://"];
        let mut url_count = 0;
        
        for content_str in [profile.get_website(), profile.get_blog()].iter().filter_map(|&x| x) {
            for pattern in &url_patterns {
                url_count += content_str.matches(pattern).count();
            }
        }
        
        if url_count > 10 {
            result.add_warning(format!("High number of URLs detected: {} (possible spam)", url_count));
        }
        
        Ok(result)
    }
    
    /// Validate profile timestamp consistency
    pub fn validate_timestamps(&self, profile: &FourWordProfile) -> Result<bool> {
        // Check that updated_at is not before created_at
        if profile.updated_at < profile.created_at {
            return Ok(false);
        }
        
        // Check that timestamps are not in the future
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Allow a small grace period (5 minutes) for clock skew
        let grace_period = 300;
        
        if profile.created_at > now + grace_period {
            return Ok(false);
        }
        
        if profile.updated_at > now + grace_period {
            return Ok(false);
        }
        
        // Check that timestamps are reasonable (not too old - e.g., before year 2020)
        let min_timestamp = 1577836800; // January 1, 2020
        
        if profile.created_at < min_timestamp {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Comprehensive profile validation
    pub fn comprehensive_validate(&self, profile: &FourWordProfile, public_key: Option<&[u8]>) -> Result<ValidationResult> {
        let mut result = ValidationResult::valid();
        
        // 1. Basic profile validation
        if let Ok(basic_validation) = self.validate_profile(profile) {
            result.merge(basic_validation);
        }
        
        // 2. Timestamp validation
        if let Ok(valid_timestamps) = self.validate_timestamps(profile) {
            if !valid_timestamps {
                result.add_error("Invalid timestamps detected".to_string());
            }
        }
        
        // 3. Size limit validation
        if let Ok(size_validation) = self.check_size_limits(profile) {
            result.merge(size_validation);
        }
        
        // 4. Signature validation if public key provided
        if let Some(pk) = public_key {
            if self.signature_required || !profile.signature.is_empty() {
                match self.validate_signature(profile, pk) {
                    Ok(true) => {
                        // Signature is valid
                    }
                    Ok(false) => {
                        result.add_error("Invalid signature".to_string());
                    }
                    Err(e) => {
                        result.add_error(format!("Signature validation failed: {}", e));
                    }
                }
            }
        } else if self.signature_required {
            result.add_error("Signature required but no public key provided for validation".to_string());
        }
        
        // 5. Markdown content validation
        if let Some(website) = profile.get_website() {
            if let Ok(valid) = self.validate_markdown(website) {
                if !valid {
                    result.add_error("Website content contains invalid markdown or dangerous patterns".to_string());
                }
            }
        }
        
        if let Some(blog) = profile.get_blog() {
            if let Ok(valid) = self.validate_markdown(blog) {
                if !valid {
                    result.add_error("Blog content contains invalid markdown or dangerous patterns".to_string());
                }
            }
        }
        
        // 6. Add informational warnings
        if profile.content.is_empty() {
            result.add_warning("Profile has no content".to_string());
        }
        
        if profile.version > 1000 {
            result.add_warning(format!("Unusually high version number: {}", profile.version));
        }
        
        Ok(result)
    }
}

impl Default for ProfileValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationResult {
    /// Create a valid result
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    /// Create an invalid result with error
    pub fn invalid(error: String) -> Self {
        Self {
            is_valid: false,
            errors: vec![error],
            warnings: Vec::new(),
        }
    }
    
    /// Add error to validation result
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_valid = false;
    }
    
    /// Add warning to validation result
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
    
    /// Check if result has any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    
    /// Check if result has any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
    
    /// Merge two validation results
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.is_valid = self.is_valid && other.is_valid;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::dns::ProfileContent;

    #[test]
    fn test_validator_creation() {
        let validator = ProfileValidator::new();
        assert_eq!(validator.max_website_size, 10 * 1024 * 1024);
        assert_eq!(validator.max_blog_size, 10 * 1024 * 1024);
        assert!(validator.signature_required);
    }
    
    #[test]
    fn test_validator_with_custom_limits() {
        let validator = ProfileValidator::with_size_limits(1024, 2048);
        assert_eq!(validator.max_website_size, 1024);
        assert_eq!(validator.max_blog_size, 2048);
        assert!(validator.signature_required);
    }
    
    #[test]
    fn test_validator_without_signature() {
        let validator = ProfileValidator::without_signature_requirement();
        assert!(!validator.signature_required);
    }
    
    #[test]
    fn test_validation_result_creation() {
        let valid_result = ValidationResult::valid();
        assert!(valid_result.is_valid);
        assert!(valid_result.errors.is_empty());
        assert!(valid_result.warnings.is_empty());
        
        let invalid_result = ValidationResult::invalid("Test error".to_string());
        assert!(!invalid_result.is_valid);
        assert_eq!(invalid_result.errors.len(), 1);
        assert_eq!(invalid_result.errors[0], "Test error");
    }
    
    #[test]
    fn test_validation_result_modification() {
        let mut result = ValidationResult::valid();
        
        result.add_warning("Test warning".to_string());
        assert!(result.has_warnings());
        assert!(!result.has_errors());
        assert!(result.is_valid);
        
        result.add_error("Test error".to_string());
        assert!(result.has_errors());
        assert!(!result.is_valid);
    }
    
    #[test]
    fn test_validation_result_merge() {
        let mut result1 = ValidationResult::valid();
        result1.add_warning("Warning 1".to_string());
        
        let mut result2 = ValidationResult::valid();
        result2.add_error("Error 1".to_string());
        
        result1.merge(result2);
        assert!(!result1.is_valid);
        assert_eq!(result1.errors.len(), 1);
        assert_eq!(result1.warnings.len(), 1);
    }
    
    #[test]
    fn test_profile_validation() {
        let validator = ProfileValidator::without_signature_requirement();
        let four_words = FourWordAddress::generate().unwrap();
        let profile = FourWordProfile::new(four_words);
        
        let result = validator.validate_profile(&profile);
        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(validation.is_valid);
        assert!(validation.errors.is_empty());
    }
    
    #[test]
    fn test_four_words_validation() {
        let validator = ProfileValidator::new();
        let four_words = FourWordAddress::generate().unwrap();
        
        let result = validator.validate_four_words(&four_words);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
    
    #[test]
    fn test_bitcoin_address_validation() {
        let validator = ProfileValidator::new();
        
        // Valid P2PKH address (starts with 1)
        let valid_p2pkh = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
        let result = validator.validate_bitcoin_address(valid_p2pkh);
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        // Valid P2SH address (starts with 3)
        let valid_p2sh = "3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy";
        let result = validator.validate_bitcoin_address(valid_p2sh);
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        // Invalid address
        let invalid = "invalid_address";
        let result = validator.validate_bitcoin_address(invalid);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
    
    #[test]
    fn test_ethereum_address_validation() {
        let validator = ProfileValidator::new();
        
        // Valid Ethereum address
        let valid_addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bE42";
        let result = validator.validate_ethereum_address(valid_addr);
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        // Invalid - wrong length
        let invalid_short = "0x742d35Cc";
        let result = validator.validate_ethereum_address(invalid_short);
        assert!(result.is_ok());
        assert!(!result.unwrap());
        
        // Invalid - no 0x prefix
        let invalid_prefix = "742d35Cc6634C0532925a3b844Bc9e7595f0bE42";
        let result = validator.validate_ethereum_address(invalid_prefix);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
    
    #[test]
    fn test_markdown_validation() {
        let validator = ProfileValidator::new();
        
        // Valid markdown
        let valid_markdown = "# My Website\n\nWelcome to my site!";
        let result = validator.validate_markdown(valid_markdown);
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        // Invalid - contains script tag
        let invalid_xss = "# Site\n<script>alert('xss')</script>";
        let result = validator.validate_markdown(invalid_xss);
        assert!(result.is_ok());
        assert!(!result.unwrap());
        
        // Invalid - empty
        let empty = "";
        let result = validator.validate_markdown(empty);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
    
    #[test]
    fn test_size_limits_checking() {
        let validator = ProfileValidator::with_size_limits(100, 200);
        let four_words = FourWordAddress::generate().unwrap();
        let profile = FourWordProfile::new(four_words)
            .with_website("x".repeat(150)) // Should exceed limit
            .with_blog("y".repeat(250));   // Should exceed limit
        
        let result = validator.check_size_limits(&profile);
        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(!validation.is_valid);
        assert!(validation.has_errors());
        assert_eq!(validation.errors.len(), 3); // website, blog, and total size errors
    }
    
    #[test]
    fn test_signature_validation() {
        let validator = ProfileValidator::new();
        let four_words = FourWordAddress::generate().unwrap();
        let mut profile = FourWordProfile::new(four_words);
        let private_key = vec![0x42u8; 32];
        
        // Sign the profile
        profile.sign(&private_key).unwrap();
        
        // Should validate with correct key
        let result = validator.validate_signature(&profile, &private_key);
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        // Should fail with wrong key
        let wrong_key = vec![0x99u8; 32];
        let result = validator.validate_signature(&profile, &wrong_key);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
    
    #[test]
    fn test_malicious_content_checking() {
        let validator = ProfileValidator::new();
        let four_words = FourWordAddress::generate().unwrap();
        let profile = FourWordProfile::new(four_words)
            .with_website("<script>alert('xss')</script>".to_string());
        
        let result = validator.check_malicious_content(&profile);
        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(!validation.is_valid);
        assert!(validation.has_errors());
        assert!(validation.errors[0].contains("malicious pattern"));
    }
    
    #[test]
    fn test_timestamp_validation() {
        let validator = ProfileValidator::new();
        let four_words = FourWordAddress::generate().unwrap();
        let mut profile = FourWordProfile::new(four_words);
        
        // Valid timestamps should pass
        let result = validator.validate_timestamps(&profile);
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        // Invalid timestamp (updated_at before created_at)
        profile.updated_at = profile.created_at - 1000;
        let result = validator.validate_timestamps(&profile);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
    
    #[test]
    fn test_comprehensive_validation() {
        let validator = ProfileValidator::new();
        let four_words = FourWordAddress::generate().unwrap();
        let mut profile = FourWordProfile::new(four_words)
            .with_website("# My Website".to_string())
            .with_bitcoin_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string());
        
        let private_key = vec![0x42u8; 32];
        profile.sign(&private_key).unwrap();
        
        let result = validator.comprehensive_validate(&profile, Some(&private_key));
        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(validation.is_valid);
        assert!(validation.warnings.is_empty() || validation.warnings[0].contains("no content"));
    }
    
    #[test]
    fn test_comprehensive_validation_without_signature() {
        let validator = ProfileValidator::without_signature_requirement();
        let four_words = FourWordAddress::generate().unwrap();
        let profile = FourWordProfile::new(four_words)
            .with_website("# Test Site".to_string());
        
        let result = validator.comprehensive_validate(&profile, None);
        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(validation.is_valid);
    }
}