
#!/usr/bin/env rust
//! Multi-format Serialization Service for DHT Storage
//! 
//! This module implements intelligent serialization format selection based on content type,
//! data size, and use case requirements. It provides automatic format detection,
//! compression support, and seamless integration with the encryption service.
//!
//! Supported formats:
//! - Bincode: Fast, compact, Rust-native (default for most data)
//! - Postcard: Deterministic, compact, ideal for DHT keys
//! - CBOR: Schema evolution friendly, good for API data
//! - MessagePack: Cross-language compatibility, efficient encoding
//!
//! Run with: `rustc --edition 2024 src/serialization_service.rs && ./serialization_service`

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Core serialization service
#[derive(Debug)]
pub struct SerializationService {
    /// Configuration for format selection
    config: SerializationConfig,
    /// Performance metrics for format selection
    metrics: SerializationMetrics,
    /// Cached format recommendations
    format_cache: HashMap<ContentType, SerializationFormat>,
}

/// Serialization configuration
#[derive(Debug, Clone)]
pub struct SerializationConfig {
    /// Default format for unknown content types
    pub default_format: SerializationFormat,
    /// Compression threshold (bytes)
    pub compression_threshold: usize,
    /// Enable automatic format detection
    pub auto_format_detection: bool,
    /// Maximum size for format detection analysis
    pub detection_size_limit: usize,
    /// Format preference overrides
    pub format_overrides: HashMap<ContentType, SerializationFormat>,
}

impl Default for SerializationConfig {
    fn default() -> Self {
        let mut format_overrides = HashMap::new();
        format_overrides.insert(ContentType::DhtKey, SerializationFormat::Postcard);
        format_overrides.insert(ContentType::ApiData, SerializationFormat::Cbor);
        format_overrides.insert(ContentType::CrossLanguage, SerializationFormat::MessagePack);
        
        Self {
            default_format: SerializationFormat::Bincode,
            compression_threshold: 1024, // 1KB
            auto_format_detection: true,
            detection_size_limit: 64 * 1024, // 64KB
            format_overrides,
        }
    }
}

/// Supported serialization formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SerializationFormat {
    /// Bincode - Fast, compact, Rust-native
    Bincode,
    /// Postcard - Deterministic, compact, ideal for DHT keys
    Postcard,
    /// CBOR - Schema evolution friendly
    Cbor,
    /// MessagePack - Cross-language compatibility
    MessagePack,
}

impl SerializationFormat {
    /// Get format characteristics
    pub fn characteristics(&self) -> FormatCharacteristics {
        match self {
            SerializationFormat::Bincode => FormatCharacteristics {
                speed: SpeedRating::VeryFast,
                size: SizeRating::VeryCompact,
                deterministic: false,
                schema_evolution: false,
                cross_language: false,
                use_cases: vec![
                    UseCase::InternalStorage,
                    UseCase::PerformanceCritical,
                    UseCase::RustNative,
                ],
            },
            SerializationFormat::Postcard => FormatCharacteristics {
                speed: SpeedRating::Fast,
                size: SizeRating::VeryCompact,
                deterministic: true,
                schema_evolution: false,
                cross_language: false,
                use_cases: vec![
                    UseCase::DhtKeys,
                    UseCase::Deterministic,
                    UseCase::EmbeddedSystems,
                ],
            },
            SerializationFormat::Cbor => FormatCharacteristics {
                speed: SpeedRating::Medium,
                size: SizeRating::Compact,
                deterministic: false,
                schema_evolution: true,
                cross_language: true,
                use_cases: vec![
                    UseCase::ApiData,
                    UseCase::SchemaEvolution,
                    UseCase::InteroperableData,
                ],
            },
            SerializationFormat::MessagePack => FormatCharacteristics {
                speed: SpeedRating::Medium,
                size: SizeRating::Compact,
                deterministic: false,
                schema_evolution: false,
                cross_language: true,
                use_cases: vec![
                    UseCase::CrossLanguage,
                    UseCase::NetworkProtocols,
                    UseCase::InteroperableData,
                ],
            },
        }
    }
    
    /// Get format name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            SerializationFormat::Bincode => "bincode",
            SerializationFormat::Postcard => "postcard",
            SerializationFormat::Cbor => "cbor",
            SerializationFormat::MessagePack => "messagepack",
        }
    }
}

/// Format characteristics for selection
#[derive(Debug, Clone)]
pub struct FormatCharacteristics {
    pub speed: SpeedRating,
    pub size: SizeRating,
    pub deterministic: bool,
    pub schema_evolution: bool,
    pub cross_language: bool,
    pub use_cases: Vec<UseCase>,
}

/// Speed rating for format selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpeedRating {
    Slow,
    Medium,
    Fast,
    VeryFast,
}

/// Size efficiency rating
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SizeRating {
    Large,
    Medium,
    Compact,
    VeryCompact,
}

/// Use case classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UseCase {
    InternalStorage,
    DhtKeys,
    ApiData,
    CrossLanguage,
    PerformanceCritical,
    SchemaEvolution,
    Deterministic,
    NetworkProtocols,
    EmbeddedSystems,
    InteroperableData,
    RustNative,
}

/// Content type classification for format selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentType {
    /// DHT keys (need deterministic serialization)
    DhtKey,
    /// DHT values (general data storage)
    DhtValue,
    /// API request/response data
    ApiData,
    /// Cross-language protocol data
    CrossLanguage,
    /// Internal application data
    Internal,
    /// Configuration data
    Configuration,
    /// Binary blob data
    Binary,
    /// Text data
    Text,
    /// Structured data with schema evolution needs
    Structured,
    /// Unknown content type
    Unknown,
}

/// Content analysis result
#[derive(Debug, Clone)]
pub struct ContentAnalysis {
    pub content_type: ContentType,
    pub size: usize,
    pub complexity: DataComplexity,
    pub requires_deterministic: bool,
    pub requires_schema_evolution: bool,
    pub requires_cross_language: bool,
    pub performance_priority: PerformancePriority,
}

/// Data complexity classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataComplexity {
    Simple,    // Basic types, small structures
    Medium,    // Nested structures, collections
    Complex,   // Deep nesting, large collections
}

/// Performance priority for serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformancePriority {
    Size,      // Minimize serialized size
    Speed,     // Maximize serialization speed
    Balanced,  // Balance size and speed
}

/// Serialization metrics for performance tracking
#[derive(Debug, Default)]
pub struct SerializationMetrics {
    /// Serialization times by format
    pub serialize_times: HashMap<SerializationFormat, Vec<Duration>>,
    /// Deserialization times by format
    pub deserialize_times: HashMap<SerializationFormat, Vec<Duration>>,
    /// Size efficiency by format
    pub size_ratios: HashMap<SerializationFormat, Vec<f64>>,
    /// Format usage counts
    pub format_usage: HashMap<SerializationFormat, u64>,
    /// Error counts by format
    pub error_counts: HashMap<SerializationFormat, u64>,
}

/// Serialization result with metadata
#[derive(Debug, Clone)]
pub struct SerializationResult {
    /// Serialized data
    pub data: Vec<u8>,
    /// Format used for serialization
    pub format: SerializationFormat,
    /// Whether compression was applied
    pub compressed: bool,
    /// Original size before compression
    pub original_size: usize,
    /// Content type detected/used
    pub content_type: ContentType,
    /// Time taken for serialization
    pub duration: Duration,
}

/// Deserialization metadata
#[derive(Debug, Clone)]
pub struct DeserializationMeta {
    /// Format detected from data
    pub format: SerializationFormat,
    /// Whether data was compressed
    pub compressed: bool,
    /// Decompressed size
    pub decompressed_size: usize,
    /// Content type
    pub content_type: ContentType,
    /// Time taken for deserialization
    pub duration: Duration,
}

/// Serialization errors
#[derive(Debug)]
pub enum SerializationError {
    /// Format not supported for this data type
    UnsupportedFormat(SerializationFormat, ContentType),
    /// Serialization failed
    SerializationFailed(String),
    /// Deserialization failed
    DeserializationFailed(String),
    /// Compression failed
    CompressionFailed(String),
    /// Decompression failed
    DecompressionFailed(String),
    /// Invalid format detected
    InvalidFormat(String),
    /// Data too large for format
    DataTooLarge(usize, usize),
    /// Unknown content type
    UnknownContentType,
}

/// Result type for serialization operations
pub type SerializationResult2<T> = Result<T, SerializationError>;

impl SerializationService {
    /// Create a new serialization service
    pub fn new(config: SerializationConfig) -> Self {
        Self {
            config,
            metrics: SerializationMetrics::default(),
            format_cache: HashMap::new(),
        }
    }
    
    /// Create service with default configuration
    pub fn with_defaults() -> Self {
        Self::new(SerializationConfig::default())
    }
    
    /// Analyze content to determine optimal serialization approach
    pub fn analyze_content(&self, data: &[u8], hint: Option<ContentType>) -> ContentAnalysis {
        let size = data.len();
        let content_type = hint.unwrap_or_else(|| self.detect_content_type(data));
        
        // Analyze data complexity based on size and structure
        let complexity = if size < 256 {
            DataComplexity::Simple
        } else if size < 4096 {
            DataComplexity::Medium
        } else {
            DataComplexity::Complex
        };
        
        // Determine requirements based on content type
        let (requires_deterministic, requires_schema_evolution, requires_cross_language) = 
            match content_type {
                ContentType::DhtKey => (true, false, false),
                ContentType::DhtValue => (false, false, false),
                ContentType::ApiData => (false, true, true),
                ContentType::CrossLanguage => (false, false, true),
                ContentType::Configuration => (false, true, false),
                ContentType::Structured => (false, true, false),
                _ => (false, false, false),
            };
        
        // Determine performance priority
        let performance_priority = match content_type {
            ContentType::DhtKey => PerformancePriority::Speed,
            ContentType::Binary | ContentType::Text => PerformancePriority::Size,
            _ => PerformancePriority::Balanced,
        };
        
        ContentAnalysis {
            content_type,
            size,
            complexity,
            requires_deterministic,
            requires_schema_evolution,
            requires_cross_language,
            performance_priority,
        }
    }
    
    /// Detect content type from data characteristics
    fn detect_content_type(&self, data: &[u8]) -> ContentType {
        if data.len() > self.config.detection_size_limit {
            return ContentType::Binary;
        }
        
        // Simple content type detection based on data patterns
        if data.len() <= 64 && data.iter().all(|&b| b.is_ascii()) {
            return ContentType::Text;
        }
        
        // Check for structured data patterns (very basic)
        if data.starts_with(b"{") || data.starts_with(b"[") {
            return ContentType::Structured;
        }
        
        // Check for DHT key patterns (32-byte keys are common)
        if data.len() == 32 || data.len() == 20 {
            return ContentType::DhtKey;
        }
        
        ContentType::Unknown
    }
    
    /// Recommend optimal format based on content analysis
    pub fn recommend_format(&mut self, analysis: &ContentAnalysis) -> SerializationFormat {
        // Check for explicit overrides first
        if let Some(&format) = self.config.format_overrides.get(&analysis.content_type) {
            return format;
        }
        
        // Check cache
        if let Some(&cached_format) = self.format_cache.get(&analysis.content_type) {
            return cached_format;
        }
        
        // Determine best format based on requirements
        let format = if analysis.requires_deterministic {
            SerializationFormat::Postcard
        } else if analysis.requires_schema_evolution {
            SerializationFormat::Cbor
        } else if analysis.requires_cross_language {
            SerializationFormat::MessagePack
        } else {
            match analysis.performance_priority {
                PerformancePriority::Speed => {
                    if analysis.size < 1024 {
                        SerializationFormat::Bincode
                    } else {
                        SerializationFormat::Postcard
                    }
                }
                PerformancePriority::Size => {
                    if analysis.complexity == DataComplexity::Simple {
                        SerializationFormat::Postcard
                    } else {
                        SerializationFormat::Bincode
                    }
                }
                PerformancePriority::Balanced => {
                    match analysis.complexity {
                        DataComplexity::Simple => SerializationFormat::Postcard,
                        DataComplexity::Medium => SerializationFormat::Bincode,
                        DataComplexity::Complex => SerializationFormat::Cbor,
                    }
                }
            }
        };
        
        // Cache the recommendation
        self.format_cache.insert(analysis.content_type, format);
        format
    }
    
    /// Serialize data with automatic format selection
    pub fn serialize_auto(&mut self, data: &[u8], hint: Option<ContentType>) -> SerializationResult2<SerializationResult> {
        let start_time = SystemTime::now();
        
        // Analyze content
        let analysis = self.analyze_content(data, hint);
        
        // Get recommended format
        let format = self.recommend_format(&analysis);
        
        // Serialize with selected format
        let serialized = self.serialize_with_format(data, format)?;
        
        // Apply compression if needed
        let (final_data, compressed) = if serialized.len() > self.config.compression_threshold {
            match self.compress_data(&serialized) {
                Ok(compressed_data) => (compressed_data, true),
                Err(_) => (serialized, false), // Fall back to uncompressed
            }
        } else {
            (serialized, false)
        };
        
        let duration = start_time.elapsed().unwrap_or(Duration::ZERO);
        
        // Update metrics
        self.update_serialize_metrics(format, duration, data.len(), final_data.len());
        
        Ok(SerializationResult {
            data: final_data,
            format,
            compressed,
            original_size: data.len(),
            content_type: analysis.content_type,
            duration,
        })
    }
    
    /// Serialize data with specific format
    pub fn serialize_with_format(&mut self, data: &[u8], format: SerializationFormat) -> SerializationResult2<Vec<u8>> {
        match format {
            SerializationFormat::Bincode => self.serialize_bincode(data),
            SerializationFormat::Postcard => self.serialize_postcard(data),
            SerializationFormat::Cbor => self.serialize_cbor(data),
            SerializationFormat::MessagePack => self.serialize_messagepack(data),
        }
    }
    
    /// Deserialize data with automatic format detection
    pub fn deserialize_auto(&mut self, data: &[u8]) -> SerializationResult2<(Vec<u8>, DeserializationMeta)> {
        let start_time = SystemTime::now();
        
        // Detect if data is compressed
        let (decompressed_data, was_compressed) = if self.is_compressed(data) {
            (self.decompress_data(data)?, true)
        } else {
            (data.to_vec(), false)
        };
        
        // Detect format
        let format = self.detect_format(&decompressed_data)?;
        
        // Deserialize with detected format
        let result = self.deserialize_with_format(&decompressed_data, format)?;
        
        let duration = start_time.elapsed().unwrap_or(Duration::ZERO);
        
        // Update metrics
        self.update_deserialize_metrics(format, duration);
        
        let meta = DeserializationMeta {
            format,
            compressed: was_compressed,
            decompressed_size: decompressed_data.len(),
            content_type: self.detect_content_type(&result),
            duration,
        };
        
        Ok((result, meta))
    }
    
    /// Deserialize with specific format
    pub fn deserialize_with_format(&mut self, data: &[u8], format: SerializationFormat) -> SerializationResult2<Vec<u8>> {
        match format {
            SerializationFormat::Bincode => self.deserialize_bincode(data),
            SerializationFormat::Postcard => self.deserialize_postcard(data),
            SerializationFormat::Cbor => self.deserialize_cbor(data),
            SerializationFormat::MessagePack => self.deserialize_messagepack(data),
        }
    }
    
    /// Format-specific serialization methods
    fn serialize_bincode(&self, data: &[u8]) -> SerializationResult2<Vec<u8>> {
        // Bincode implementation (simplified)
        // In real implementation, this would use the bincode crate
        let mut result = Vec::with_capacity(data.len() + 8);
        result.extend_from_slice(b"BINC"); // Format marker
        result.extend_from_slice(&(data.len() as u32).to_le_bytes());
        result.extend_from_slice(data);
        Ok(result)
    }
    
    fn serialize_postcard(&self, data: &[u8]) -> SerializationResult2<Vec<u8>> {
        // Postcard implementation (simplified)
        let mut result = Vec::with_capacity(data.len() + 8);
        result.extend_from_slice(b"POST"); // Format marker
        result.extend_from_slice(&(data.len() as u32).to_le_bytes());
        result.extend_from_slice(data);
        Ok(result)
    }
    
    fn serialize_cbor(&self, data: &[u8]) -> SerializationResult2<Vec<u8>> {
        // CBOR implementation (simplified)
        let mut result = Vec::with_capacity(data.len() + 8);
        result.extend_from_slice(b"CBOR"); // Format marker
        result.extend_from_slice(&(data.len() as u32).to_le_bytes());
        result.extend_from_slice(data);
        Ok(result)
    }
    
    fn serialize_messagepack(&self, data: &[u8]) -> SerializationResult2<Vec<u8>> {
        // MessagePack implementation (simplified)
        let mut result = Vec::with_capacity(data.len() + 8);
        result.extend_from_slice(b"MSGP"); // Format marker
        result.extend_from_slice(&(data.len() as u32).to_le_bytes());
        result.extend_from_slice(data);
        Ok(result)
    }
    
    /// Format-specific deserialization methods
    fn deserialize_bincode(&self, data: &[u8]) -> SerializationResult2<Vec<u8>> {
        if data.len() < 8 || &data[0..4] != b"BINC" {
            return Err(SerializationError::InvalidFormat("Not bincode format".to_string()));
        }
        
        let size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        if data.len() < 8 + size {
            return Err(SerializationError::DeserializationFailed("Truncated data".to_string()));
        }
        
        Ok(data[8..8+size].to_vec())
    }
    
    fn deserialize_postcard(&self, data: &[u8]) -> SerializationResult2<Vec<u8>> {
        if data.len() < 8 || &data[0..4] != b"POST" {
            return Err(SerializationError::InvalidFormat("Not postcard format".to_string()));
        }
        
        let size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        if data.len() < 8 + size {
            return Err(SerializationError::DeserializationFailed("Truncated data".to_string()));
        }
        
        Ok(data[8..8+size].to_vec())
    }
    
    fn deserialize_cbor(&self, data: &[u8]) -> SerializationResult2<Vec<u8>> {
        if data.len() < 8 || &data[0..4] != b"CBOR" {
            return Err(SerializationError::InvalidFormat("Not CBOR format".to_string()));
        }
        
        let size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        if data.len() < 8 + size {
            return Err(SerializationError::DeserializationFailed("Truncated data".to_string()));
        }
        
        Ok(data[8..8+size].to_vec())
    }
    
    fn deserialize_messagepack(&self, data: &[u8]) -> SerializationResult2<Vec<u8>> {
        if data.len() < 8 || &data[0..4] != b"MSGP" {
            return Err(SerializationError::InvalidFormat("Not MessagePack format".to_string()));
        }
        
        let size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        if data.len() < 8 + size {
            return Err(SerializationError::DeserializationFailed("Truncated data".to_string()));
        }
        
        Ok(data[8..8+size].to_vec())
    }
    
    /// Compression methods
    fn compress_data(&self, data: &[u8]) -> SerializationResult2<Vec<u8>> {
        // Simple compression simulation (in real implementation, use flate2 or zstd)
        let mut compressed = Vec::with_capacity(data.len() / 2);
        compressed.extend_from_slice(b"COMP"); // Compression marker
        compressed.extend_from_slice(&(data.len() as u32).to_le_bytes());
        
        // Simulate compression by removing repeated bytes (very basic)
        let mut prev_byte = None;
        let mut count = 0u8;
        
        for &byte in data {
            if Some(byte) == prev_byte && count < 255 {
                count += 1;
            } else {
                if let Some(prev) = prev_byte {
                    if count > 2 {
                        compressed.push(0xFF); // RLE marker
                        compressed.push(prev);
                        compressed.push(count);
                    } else {
                        for _ in 0..=count {
                            compressed.push(prev);
                        }
                    }
                }
                prev_byte = Some(byte);
                count = 0;
            }
        }
        
        // Handle last sequence
        if let Some(prev) = prev_byte {
            if count > 2 {
                compressed.push(0xFF);
                compressed.push(prev);
                compressed.push(count);
            } else {
                for _ in 0..=count {
                    compressed.push(prev);
                }
            }
        }
        
        Ok(compressed)
    }
    
    fn decompress_data(&self, data: &[u8]) -> SerializationResult2<Vec<u8>> {
        if data.len() < 8 || &data[0..4] != b"COMP" {
            return Ok(data.to_vec()); // Not compressed
        }
        
        let original_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let compressed_data = &data[8..];
        
        let mut decompressed = Vec::with_capacity(original_size);
        let mut i = 0;
        
        while i < compressed_data.len() {
            if compressed_data[i] == 0xFF && i + 2 < compressed_data.len() {
                // RLE sequence
                let byte = compressed_data[i + 1];
                let count = compressed_data[i + 2];
                for _ in 0..=count {
                    decompressed.push(byte);
                }
                i += 3;
            } else {
                decompressed.push(compressed_data[i]);
                i += 1;
            }
        }
        
        Ok(decompressed)
    }
    
    /// Check if data appears to be compressed
    fn is_compressed(&self, data: &[u8]) -> bool {
        data.len() >= 4 && &data[0..4] == b"COMP"
    }
    
    /// Detect format from serialized data
    fn detect_format(&self, data: &[u8]) -> SerializationResult2<SerializationFormat> {
        if data.len() < 4 {
            return Err(SerializationError::InvalidFormat("Data too short".to_string()));
        }
        
        match &data[0..4] {
            b"BINC" => Ok(SerializationFormat::Bincode),
            b"POST" => Ok(SerializationFormat::Postcard),
            b"CBOR" => Ok(SerializationFormat::Cbor),
            b"MSGP" => Ok(SerializationFormat::MessagePack),
            _ => Err(SerializationError::InvalidFormat("Unknown format".to_string())),
        }
    }
    
    /// Update serialization metrics
    fn update_serialize_metrics(&mut self, format: SerializationFormat, duration: Duration, original_size: usize, serialized_size: usize) {
        self.metrics.serialize_times.entry(format).or_default().push(duration);
        self.metrics.size_ratios.entry(format).or_default().push(serialized_size as f64 / original_size as f64);
        *self.metrics.format_usage.entry(format).or_default() += 1;
    }
    
    /// Update deserialization metrics
    fn update_deserialize_metrics(&mut self, format: SerializationFormat, duration: Duration) {
        self.metrics.deserialize_times.entry(format).or_default().push(duration);
    }
    
    /// Get performance statistics
    pub fn get_statistics(&self) -> SerializationStatistics {
        let mut stats = SerializationStatistics::new();
        
        for (format, times) in &self.metrics.serialize_times {
            if !times.is_empty() {
                let avg_time = times.iter().sum::<Duration>() / times.len() as u32;
                stats.avg_serialize_time.insert(*format, avg_time);
            }
        }
        
        for (format, times) in &self.metrics.deserialize_times {
            if !times.is_empty() {
                let avg_time = times.iter().sum::<Duration>() / times.len() as u32;
                stats.avg_deserialize_time.insert(*format, avg_time);
            }
        }
        
        for (format, ratios) in &self.metrics.size_ratios {
            if !ratios.is_empty() {
                let avg_ratio = ratios.iter().sum::<f64>() / ratios.len() as f64;
                stats.avg_size_ratio.insert(*format, avg_ratio);
            }
        }
        
        stats.format_usage = self.metrics.format_usage.clone();
        stats
    }
    
    /// Clear performance metrics
    pub fn clear_metrics(&mut self) {
        self.metrics = SerializationMetrics::default();
    }
}

/// Performance statistics
#[derive(Debug, Default)]
pub struct SerializationStatistics {
    pub avg_serialize_time: HashMap<SerializationFormat, Duration>,
    pub avg_deserialize_time: HashMap<SerializationFormat, Duration>,
    pub avg_size_ratio: HashMap<SerializationFormat, f64>,
    pub format_usage: HashMap<SerializationFormat, u64>,
}

impl SerializationStatistics {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn print_report(&self) {
        println!("=== Serialization Performance Report ===");
        
        for format in [SerializationFormat::Bincode, SerializationFormat::Postcard, SerializationFormat::Cbor, SerializationFormat::MessagePack] {
            println!("\n{} Statistics:", format.as_str().to_uppercase());
            
            if let Some(usage) = self.format_usage.get(&format) {
                println!("  Usage count: {}", usage);
            }
            
            if let Some(time) = self.avg_serialize_time.get(&format) {
                println!("  Avg serialize time: {:?}", time);
            }
            
            if let Some(time) = self.avg_deserialize_time.get(&format) {
                println!("  Avg deserialize time: {:?}", time);
            }
            
            if let Some(ratio) = self.avg_size_ratio.get(&format) {
                println!("  Avg size ratio: {:.3}", ratio);
            }
        }
    }
}

/// Demo and test function
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Multi-format Serialization Service Demo");
    println!("==========================================");
    
    let mut service = SerializationService::with_defaults();
    
    // Test different content types
    let test_cases = vec![
        (b"short_dht_key_32bytes_exactly__".to_vec(), Some(ContentType::DhtKey)),
        (b"This is some API data that needs schema evolution support".to_vec(), Some(ContentType::ApiData)),
        (b"Cross-language data".to_vec(), Some(ContentType::CrossLanguage)),
        (vec![0u8; 2048], Some(ContentType::Binary)), // Large binary data
        (b"Simple text".to_vec(), None), // Auto-detect
    ];
    
    println!("\n📊 Testing automatic format selection:");
    for (i, (data, content_hint)) in test_cases.iter().enumerate() {
        println!("\n--- Test Case {} ---", i + 1);
        
        // Analyze content
        let analysis = service.analyze_content(data, *content_hint);
        println!("Content type: {:?}", analysis.content_type);
        println!("Size: {} bytes", analysis.size);
        println!("Complexity: {:?}", analysis.complexity);
        println!("Performance priority: {:?}", analysis.performance_priority);
        
        // Get format recommendation
        let format = service.recommend_format(&analysis);
        println!("Recommended format: {}", format.as_str());
        
        // Test serialization
        match service.serialize_auto(data, *content_hint) {
            Ok(result) => {
                println!("✅ Serialization successful:");
                println!("  Format used: {}", result.format.as_str());
                println!("  Original size: {} bytes", result.original_size);
                println!("  Serialized size: {} bytes", result.data.len());
                println!("  Compressed: {}", result.compressed);
                println!("  Duration: {:?}", result.duration);
                
                // Test deserialization
                match service.deserialize_auto(&result.data) {
                    Ok((deserialized, meta)) => {
                        println!("✅ Deserialization successful:");
                        println!("  Format detected: {}", meta.format.as_str());
                        println!("  Was compressed: {}", meta.compressed);
                        println!("  Decompressed size: {} bytes", meta.decompressed_size);
                        println!("  Duration: {:?}", meta.duration);
                        
                        // Verify round-trip
                        if deserialized == *data {
                            println!("✅ Round-trip verification passed");
                        } else {
                            println!("❌ Round-trip verification failed");
                        }
                    }
                    Err(e) => println!("❌ Deserialization failed: {:?}", e),
                }
            }
            Err(e) => println!("❌ Serialization failed: {:?}", e),
        }
    }
    
    // Print performance statistics
    println!("\n📈 Performance Statistics:");
    let stats = service.get_statistics();
    stats.print_report();
    
    // Test format characteristics
    println!("\n🔍 Format Characteristics:");
    for format in [SerializationFormat::Bincode, SerializationFormat::Postcard, SerializationFormat::Cbor, SerializationFormat::MessagePack] {
        let chars = format.characteristics();
        println!("\n{} ({}):", format.as_str().to_uppercase(), format.as_str());
        println!("  Speed: {:?}", chars.speed);
        println!("  Size: {:?}", chars.size);
        println!("  Deterministic: {}", chars.deterministic);
        println!("  Schema evolution: {}", chars.schema_evolution);
        println!("  Cross-language: {}", chars.cross_language);
        println!("  Use cases: {:?}", chars.use_cases);
    }
    
    println!("\n✨ Multi-format serialization service demonstration completed!");
    println!("🎯 The service successfully:");
    println!("   • Analyzed content characteristics");
    println!("   • Recommended optimal formats");
    println!("   • Performed automatic serialization/deserialization");
    println!("   • Applied compression when beneficial");
    println!("   • Tracked performance metrics");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_content_analysis() {
        let service = SerializationService::with_defaults();
        
        // Test DHT key detection
        let dht_key = vec![0u8; 32];
        let analysis = service.analyze_content(&dht_key, Some(ContentType::DhtKey));
        assert_eq!(analysis.content_type, ContentType::DhtKey);
        assert!(analysis.requires_deterministic);
        
        // Test text content
        let text_data = b"Hello world";
        let analysis = service.analyze_content(text_data, None);
        assert_eq!(analysis.content_type, ContentType::Text);
    }
    
    #[test]
    fn test_format_recommendation() {
        let mut service = SerializationService::with_defaults();
        
        // Test DHT key recommendation
        let analysis = ContentAnalysis {
            content_type: ContentType::DhtKey,
            size: 32,
            complexity: DataComplexity::Simple,
            requires_deterministic: true,
            requires_schema_evolution: false,
            requires_cross_language: false,
            performance_priority: PerformancePriority::Speed,
        };
        
        let format = service.recommend_format(&analysis);
        assert_eq!(format, SerializationFormat::Postcard);
    }
    
    #[test]
    fn test_format_characteristics() {
        let bincode_chars = SerializationFormat::Bincode.characteristics();
        assert_eq!(bincode_chars.speed, SpeedRating::VeryFast);
        assert!(!bincode_chars.deterministic);
        
        let postcard_chars = SerializationFormat::Postcard.characteristics();
        assert!(postcard_chars.deterministic);
        assert_eq!(postcard_chars.size, SizeRating::VeryCompact);
    }
    
    #[test]
    fn test_serialization_round_trip() {
        let mut service = SerializationService::with_defaults();
        let test_data = b"Test data for round-trip verification";
        
        // Test with each format
        for format in [SerializationFormat::Bincode, SerializationFormat::Postcard, SerializationFormat::Cbor, SerializationFormat::MessagePack] {
            let serialized = service.serialize_with_format(test_data, format).unwrap();
            let deserialized = service.deserialize_with_format(&serialized, format).unwrap();
            assert_eq!(test_data.as_slice(), deserialized.as_slice());
        }
    }
    
    #[test]
    fn test_compression() {
        let service = SerializationService::with_defaults();
        
        // Test compression with repeated data
        let data = vec![0xAB; 1000];
        let compressed = service.compress_data(&data).unwrap();
        assert!(compressed.len() < data.len());
        
        let decompressed = service.decompress_data(&compressed).unwrap();
        assert_eq!(data, decompressed);
    }
    
    #[test]
    fn test_format_detection() {
        let mut service = SerializationService::with_defaults();
        
        let test_data = b"test";
        
        for format in [SerializationFormat::Bincode, SerializationFormat::Postcard, SerializationFormat::Cbor, SerializationFormat::MessagePack] {
            let serialized = service.serialize_with_format(test_data, format).unwrap();
            let detected = service.detect_format(&serialized).unwrap();
            assert_eq!(format, detected);
        }
    }
}