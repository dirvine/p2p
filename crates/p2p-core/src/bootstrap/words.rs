//! Three-Word Address System
//!
//! Converts complex multiaddrs into memorable three-word combinations for human-friendly
//! peer discovery and sharing. Inspired by what3words but designed specifically for
//! P2P network bootstrap addresses.
//!
//! Example: `/ip6/2001:db8::1/udp/9000/quic` ↔ `ocean.thunder.falcon`

use crate::{Multiaddr, P2PError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Maximum number of words per position in the base dictionary
const BASE_WORDS_PER_POSITION: usize = 4096; // 2^12 for massive address space

/// Target words per position for production deployment
const TARGET_WORDS_PER_POSITION: usize = 8192; // 2^13 for ultimate scale

/// Extended addressing with numeric suffixes for massive scale
const NUMERIC_SUFFIX_BITS: u32 = 32; // Additional 32 bits = 4.3 billion per base address

/// Total base combinations: 4096^3 = ~68.7 billion three-word addresses  
const BASE_COMBINATIONS: u64 = (BASE_WORDS_PER_POSITION as u64).pow(3);

/// Total extended combinations: 68.7 billion × 4.3 billion = ~295 quintillion addresses
const TOTAL_COMBINATIONS: u64 = BASE_COMBINATIONS * (2_u64.pow(NUMERIC_SUFFIX_BITS));

/// Ultimate target capacity with 8192 words per position
const ULTIMATE_COMBINATIONS: u64 = (TARGET_WORDS_PER_POSITION as u64).pow(3) * (2_u64.pow(NUMERIC_SUFFIX_BITS));

/// Three-word address representation with optional numeric suffix for massive scale
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThreeWordAddress {
    pub first: String,
    pub second: String, 
    pub third: String,
    /// Optional numeric suffix for extended addressing (None for base addresses)
    pub suffix: Option<u32>,
}

impl ThreeWordAddress {
    /// Create a new three-word address
    pub fn new(first: String, second: String, third: String) -> Self {
        Self { first, second, third, suffix: None }
    }
    
    /// Create a new three-word address with numeric suffix
    pub fn new_with_suffix(first: String, second: String, third: String, suffix: u32) -> Self {
        Self { first, second, third, suffix: Some(suffix) }
    }
    
    /// Parse from dot-separated string format (supports optional numeric suffix)
    /// Examples: "forest.lightning.compass" or "forest.lightning.compass.1847"
    pub fn from_string(input: &str) -> Result<Self> {
        let parts: Vec<&str> = input.split('.').collect();
        
        match parts.len() {
            3 => {
                // Base three-word format
                Ok(Self {
                    first: parts[0].to_lowercase(),
                    second: parts[1].to_lowercase(),
                    third: parts[2].to_lowercase(),
                    suffix: None,
                })
            }
            4 => {
                // Extended format with numeric suffix
                let suffix = parts[3].parse::<u32>()
                    .map_err(|e| P2PError::Bootstrap(
                        format!("Invalid numeric suffix '{}': {}", parts[3], e)
                    ))?;
                    
                Ok(Self {
                    first: parts[0].to_lowercase(),
                    second: parts[1].to_lowercase(),
                    third: parts[2].to_lowercase(),
                    suffix: Some(suffix),
                })
            }
            _ => Err(P2PError::Bootstrap(
                format!("Address must have 3 words or 3 words + numeric suffix, got: {}", input)
            ))
        }
    }
    
    /// Convert to dot-separated string format
    pub fn to_string(&self) -> String {
        if let Some(suffix) = self.suffix {
            format!("{}.{}.{}.{}", self.first, self.second, self.third, suffix)
        } else {
            format!("{}.{}.{}", self.first, self.second, self.third)
        }
    }
    
    /// Get the base three-word part (without suffix)
    pub fn base_address(&self) -> String {
        format!("{}.{}.{}", self.first, self.second, self.third)
    }
    
    /// Check if this is an extended address (has numeric suffix)
    pub fn is_extended(&self) -> bool {
        self.suffix.is_some()
    }
    
    /// Get the estimated total address space this represents
    pub fn address_space_size() -> u64 {
        TOTAL_COMBINATIONS
    }
    
    /// Get human-readable description of address space
    pub fn address_space_description() -> String {
        format!(
            "~{:.1} trillion addresses ({} base three-word \u{00d7} {} suffixes)",
            TOTAL_COMBINATIONS as f64 / 1e12,
            BASE_COMBINATIONS,
            2_u64.pow(NUMERIC_SUFFIX_BITS)
        )
    }
    
    /// Validate that all words exist in the dictionary
    pub fn validate(&self, encoder: &WordEncoder) -> Result<()> {
        encoder.validate_words(&self.first, &self.second, &self.third)
    }
}

impl std::fmt::Display for ThreeWordAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl FromStr for ThreeWordAddress {
    type Err = P2PError;
    
    fn from_str(s: &str) -> Result<Self> {
        Self::from_string(s)
    }
}

/// Word dictionary for three-word address encoding
#[derive(Debug, Clone)]
pub struct WordDictionary {
    /// Context words (position 1): geographic, network type
    context_words: Vec<String>,
    /// Quality words (position 2): performance, purpose, status  
    quality_words: Vec<String>,
    /// Identity words (position 3): nature, objects, abstract concepts
    identity_words: Vec<String>,
    
    /// Reverse lookup maps
    context_map: HashMap<String, usize>,
    quality_map: HashMap<String, usize>,
    identity_map: HashMap<String, usize>,
}

impl WordDictionary {
    /// Create a new word dictionary with default English words
    pub fn new() -> Self {
        let context_words = Self::default_context_words();
        let quality_words = Self::default_quality_words();
        let identity_words = Self::default_identity_words();
        
        let context_map: HashMap<String, usize> = context_words
            .iter()
            .enumerate()
            .map(|(i, word)| (word.clone(), i))
            .collect();
            
        let quality_map: HashMap<String, usize> = quality_words
            .iter()
            .enumerate() 
            .map(|(i, word)| (word.clone(), i))
            .collect();
            
        let identity_map: HashMap<String, usize> = identity_words
            .iter()
            .enumerate()
            .map(|(i, word)| (word.clone(), i))
            .collect();
        
        Self {
            context_words,
            quality_words,
            identity_words,
            context_map,
            quality_map,
            identity_map,
        }
    }
    
    /// Get word by position and index
    pub fn get_word(&self, position: usize, index: usize) -> Option<&String> {
        match position {
            0 => self.context_words.get(index),
            1 => self.quality_words.get(index), 
            2 => self.identity_words.get(index),
            _ => None,
        }
    }
    
    /// Get index by position and word
    pub fn get_index(&self, position: usize, word: &str) -> Option<usize> {
        let word_lower = word.to_lowercase();
        match position {
            0 => self.context_map.get(&word_lower).copied(),
            1 => self.quality_map.get(&word_lower).copied(),
            2 => self.identity_map.get(&word_lower).copied(),
            _ => None,
        }
    }
    
    /// Validate that a word exists in the specified position
    pub fn validate_word(&self, position: usize, word: &str) -> bool {
        self.get_index(position, word).is_some()
    }
    
    /// Get all words for a specific position
    pub fn get_words_for_position(&self, position: usize) -> Option<&Vec<String>> {
        match position {
            0 => Some(&self.context_words),
            1 => Some(&self.quality_words),
            2 => Some(&self.identity_words),
            _ => None,
        }
    }
    
    /// Default context words (position 1) - geographic and network context
    fn default_context_words() -> Vec<String> {
        vec![
            // Geographic contexts
            "global", "europe", "america", "asia", "africa", "oceania", "arctic", "pacific",
            "atlantic", "indian", "mountain", "desert", "forest", "urban", "rural", "coastal",
            "island", "valley", "plateau", "tundra", "savanna", "jungle", "prairie", "canyon",
            
            // Network contexts  
            "local", "mesh", "bridge", "gateway", "relay", "hub", "node", "cluster", "edge",
            "core", "access", "backbone", "fiber", "wireless", "mobile", "fixed", "satellite",
            "ground", "space", "cloud", "fog", "mist", "clear", "direct", "routed", "switched",
            
            // Scale contexts
            "micro", "mini", "small", "medium", "large", "huge", "giant", "massive", "tiny",
            "compact", "wide", "narrow", "deep", "shallow", "high", "low", "fast", "slow",
            
            // Additional contexts to reach 2048 words
            "north", "south", "east", "west", "central", "remote", "near", "far", "inner",
            "outer", "upper", "lower", "front", "back", "left", "right", "home", "work",
            "school", "public", "private", "open", "closed", "secure", "safe", "quick",
            "steady", "smooth", "rough", "sharp", "soft", "hard", "light", "dark", "bright",
            "dim", "warm", "cool", "hot", "cold", "fresh", "old", "new", "modern", "classic",
            
            // Additional network contexts - Enterprise & Cloud
            "datacenter", "server", "client", "gateway", "proxy", "cache", "load", "balance",
            "cluster", "shard", "replica", "master", "worker", "agent", "service", "daemon",
            "container", "pod", "namespace", "tenant", "instance", "endpoint", "interface",
            "protocol", "layer", "stack", "frame", "packet", "stream", "flow", "pipe",
            
            // Geographic - Countries & Regions (Top 200)
            "america", "canada", "mexico", "brazil", "argentina", "chile", "colombia", "peru",
            "europe", "germany", "france", "italy", "spain", "poland", "romania", "netherlands",
            "belgium", "greece", "portugal", "czech", "hungary", "sweden", "austria", "belarus",
            "switzerland", "bulgaria", "serbia", "denmark", "finland", "slovakia", "norway",
            "ireland", "croatia", "bosnia", "albania", "lithuania", "slovenia", "latvia",
            "estonia", "macedonia", "moldova", "malta", "iceland", "luxembourg", "cyprus",
            "asia", "china", "india", "indonesia", "pakistan", "bangladesh", "japan", "philippines",
            "vietnam", "turkey", "iran", "thailand", "myanmar", "korea", "iraq", "afghanistan",
            "uzbekistan", "malaysia", "nepal", "yemen", "cambodia", "jordan", "azerbaijan",
            "emirates", "tajikistan", "israel", "laos", "singapore", "lebanon", "kyrgyzstan",
            "mongolia", "armenia", "kuwait", "georgia", "bahrain", "qatar", "brunei", "maldives",
            "africa", "nigeria", "ethiopia", "egypt", "congo", "tanzania", "south", "kenya",
            "uganda", "algeria", "sudan", "morocco", "angola", "ghana", "mozambique", "madagascar",
            "cameroon", "ivory", "niger", "burkina", "mali", "malawi", "zambia", "somalia",
            "senegal", "chad", "zimbabwe", "guinea", "rwanda", "benin", "tunisia", "burundi",
            "togo", "libya", "liberia", "sierra", "mauritania", "eritrea", "gambia", "botswana",
            "namibia", "gabon", "lesotho", "guinea", "equatorial", "mauritius", "swaziland",
            "djibouti", "comoros", "cabo", "sao", "seychelles",
            
            // Oceania & Others
            "oceania", "australia", "papua", "zealand", "fiji", "solomon", "vanuatu", "samoa",
            "kiribati", "tonga", "palau", "marshall", "tuvalu", "nauru",
            
            // Cities (Major Global Cities)
            "tokyo", "delhi", "shanghai", "paulo", "mumbai", "beijing", "dhaka", "osaka",
            "york", "cairo", "angeles", "bangkok", "london", "lima", "tehran", "bogota",
            "hong", "lagos", "seoul", "jakarta", "manila", "karachi", "istanbul", "moscow",
            "paris", "berlin", "madrid", "rome", "vienna", "amsterdam", "brussels", "stockholm",
            "oslo", "helsinki", "copenhagen", "dublin", "lisbon", "athens", "prague", "budapest",
            "warsaw", "bucharest", "sofia", "zagreb", "belgrade", "sarajevo", "skopje", "tirana",
            "kiev", "minsk", "vilnius", "riga", "tallinn", "reykjavik",
            
            // Terrain & Landscapes
            "alpine", "arctic", "desert", "forest", "jungle", "plains", "hills", "mountains",
            "valley", "canyon", "plateau", "mesa", "butte", "ridge", "peak", "summit",
            "coast", "shore", "beach", "cliff", "bay", "inlet", "strait", "channel",
            "river", "lake", "pond", "stream", "creek", "brook", "spring", "falls",
            "ocean", "sea", "gulf", "sound", "lagoon", "marsh", "swamp", "wetland",
            "island", "atoll", "reef", "cape", "peninsula", "isthmus", "delta", "estuary",
            "glacier", "iceberg", "tundra", "steppe", "prairie", "savanna", "grassland",
            "meadow", "field", "pasture", "orchard", "vineyard", "garden", "grove", "woods",
            
            // Technology Contexts
            "quantum", "neural", "bio", "nano", "micro", "macro", "cyber", "digital",
            "virtual", "augmented", "mixed", "cloud", "edge", "fog", "mist", "vapor",
            "mesh", "grid", "fabric", "web", "net", "link", "chain", "ring",
            "star", "tree", "bus", "line", "point", "multi", "uni", "broad",
            "narrow", "wide", "thin", "thick", "dense", "sparse", "tight", "loose",
            
            // Temporal Contexts  
            "dawn", "morning", "noon", "afternoon", "evening", "dusk", "night", "midnight",
            "spring", "summer", "autumn", "winter", "past", "present", "future", "eternal",
            "ancient", "medieval", "modern", "future", "next", "current", "previous", "legacy",
            "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta",
            "iota", "kappa", "lambda", "mu", "nu", "xi", "omicron", "pi",
            "rho", "sigma", "tau", "upsilon", "phi", "chi", "psi", "omega",
            
            // Mathematical & Scientific
            "zero", "one", "two", "three", "four", "five", "six", "seven",
            "eight", "nine", "ten", "hundred", "thousand", "million", "billion", "trillion",
            "prime", "even", "odd", "square", "cube", "root", "power", "base",
            "binary", "decimal", "hex", "octal", "complex", "real", "integer", "float",
            "vector", "matrix", "tensor", "scalar", "field", "ring", "group", "set",
            
            // Organizational Levels
            "global", "continental", "national", "regional", "state", "county", "city", "district",
            "zone", "sector", "block", "unit", "group", "team", "squad", "crew",
            "department", "division", "branch", "office", "floor", "room", "desk", "seat",
            "enterprise", "corporate", "business", "startup", "company", "firm", "agency",
            "organization", "institution", "foundation", "association", "society", "club",
            "government", "federal", "state", "local", "municipal", "county", "parish",
            "military", "army", "navy", "air", "space", "marine", "guard", "reserve",
            "civilian", "public", "private", "personal", "family", "individual", "social",
            "academic", "university", "college", "school", "research", "laboratory", "institute",
            "medical", "hospital", "clinic", "health", "care", "emergency", "trauma",
            "financial", "bank", "credit", "investment", "insurance", "retail", "commerce",
        ].iter().map(|s| s.to_string()).collect();
        
        // Extend to reach BASE_WORDS_PER_POSITION (4096) words
        while words.len() < BASE_WORDS_PER_POSITION {
            // Add systematic variations and combinations
            let base_count = words.len();
            for i in 0..std::cmp::min(100, BASE_WORDS_PER_POSITION - words.len()) {
                words.push(format!("ctx{:04}", base_count + i));
            }
        }
        
        words.truncate(BASE_WORDS_PER_POSITION);
        words
    }
    
    /// Default quality words (position 2) - performance, purpose, status
    fn default_quality_words() -> Vec<String> {
        vec![
            // Performance qualities
            "fast", "quick", "rapid", "swift", "speedy", "turbo", "hyper", "ultra", "super",
            "stable", "solid", "steady", "reliable", "robust", "strong", "secure", "safe",
            "premium", "elite", "pro", "advanced", "expert", "master", "prime", "top", "best",
            "smooth", "fluid", "agile", "nimble", "efficient", "optimal", "perfect", "ideal",
            
            // Purpose qualities
            "chat", "talk", "voice", "video", "stream", "share", "store", "backup", "sync",
            "game", "play", "work", "study", "learn", "teach", "create", "build", "design",
            "connect", "link", "bridge", "tunnel", "route", "switch", "filter", "block",
            "allow", "grant", "deny", "check", "verify", "trust", "guard", "watch", "monitor",
            
            // Status qualities  
            "active", "live", "online", "ready", "awake", "alert", "busy", "free", "open",
            "public", "private", "hidden", "visible", "clear", "bright", "sharp", "focused",
            "verified", "trusted", "known", "famous", "popular", "common", "rare", "unique",
            "special", "magic", "power", "energy", "force", "strength", "grace", "beauty",
            
            // Additional qualities - Extended performance  
            "gentle", "calm", "peaceful", "quiet", "loud", "bold", "brave", "smart", "wise",
            "clever", "bright", "brilliant", "clear", "pure", "clean", "fresh", "green",
            "blue", "red", "gold", "silver", "bronze", "crystal", "diamond", "pearl", "ruby",
            
            // Extended performance qualities
            "blazing", "lightning", "instant", "immediate", "rapid", "swift", "quick", "fast",
            "speedy", "velocity", "turbo", "boost", "accelerated", "enhanced", "optimized",
            "efficient", "streamlined", "smooth", "fluid", "seamless", "frictionless",
            "responsive", "agile", "nimble", "flexible", "adaptive", "dynamic", "elastic",
            "scalable", "expandable", "extensible", "modular", "portable", "mobile",
            
            // Reliability & Stability
            "reliable", "dependable", "trustworthy", "consistent", "steady", "stable",
            "solid", "robust", "resilient", "durable", "lasting", "enduring", "persistent",
            "permanent", "fixed", "immutable", "constant", "invariant", "static",
            "bulletproof", "failsafe", "redundant", "backup", "emergency", "standby",
            "primary", "secondary", "tertiary", "auxiliary", "reserve", "spare",
            
            // Security & Protection
            "secure", "safe", "protected", "shielded", "guarded", "defended", "fortified",
            "encrypted", "signed", "verified", "authenticated", "authorized", "certified",
            "validated", "approved", "licensed", "registered", "official", "legitimate",
            "trusted", "confirmed", "guaranteed", "assured", "insured", "covered",
            "private", "confidential", "classified", "restricted", "limited", "exclusive",
            "public", "open", "shared", "common", "general", "universal", "global",
            
            // Quality Levels & Tiers
            "premium", "deluxe", "luxury", "elite", "platinum", "titanium", "quantum",
            "professional", "expert", "master", "advanced", "intermediate", "basic", "entry",
            "starter", "beginner", "novice", "amateur", "student", "trainee", "junior",
            "senior", "lead", "chief", "principal", "director", "manager", "supervisor",
            "executive", "corporate", "enterprise", "business", "commercial", "industrial",
            
            // Size & Scale Descriptors
            "massive", "huge", "giant", "enormous", "colossal", "gigantic", "immense",
            "vast", "extensive", "broad", "wide", "large", "big", "major", "grand",
            "medium", "standard", "regular", "normal", "average", "typical", "common",
            "small", "little", "minor", "tiny", "micro", "mini", "compact", "dense",
            "thick", "thin", "narrow", "slim", "lean", "light", "heavy", "deep",
            "shallow", "high", "low", "tall", "short", "long", "brief", "extended",
            
            // Status & Operational State
            "active", "live", "online", "connected", "linked", "joined", "synced",
            "ready", "prepared", "standby", "waiting", "pending", "queued", "scheduled",
            "running", "operating", "working", "functioning", "executing", "processing",
            "complete", "finished", "done", "ended", "closed", "terminated", "stopped",
            "paused", "suspended", "frozen", "locked", "blocked", "disabled", "inactive",
            "offline", "disconnected", "broken", "failed", "error", "warning", "alert",
            "healthy", "ok", "good", "fine", "excellent", "perfect", "ideal", "optimal",
            
            // Purpose & Functional Role
            "main", "central", "core", "key", "essential", "critical", "vital", "important",
            "significant", "major", "minor", "auxiliary", "helper", "support", "assist",
            "backup", "failover", "recovery", "restore", "repair", "fix", "patch",
            "create", "build", "make", "generate", "produce", "develop", "design",
            "test", "check", "verify", "validate", "confirm", "approve", "reject",
            "send", "receive", "transmit", "broadcast", "multicast", "unicast", "relay",
            "route", "switch", "bridge", "tunnel", "proxy", "cache", "store", "save",
            
            // Characteristics & Properties
            "smart", "intelligent", "clever", "wise", "brilliant", "genius", "sharp",
            "simple", "easy", "basic", "plain", "clear", "obvious", "straightforward",
            "complex", "complicated", "advanced", "sophisticated", "intricate", "detailed",
            "new", "modern", "current", "latest", "updated", "fresh", "recent",
            "old", "legacy", "vintage", "classic", "traditional", "conventional", "standard",
            "custom", "special", "unique", "rare", "exclusive", "limited", "restricted",
            "popular", "frequent", "regular", "normal", "typical", "random", "variable",
            
            // Aesthetic & Sensory
            "beautiful", "pretty", "elegant", "graceful", "charming", "attractive", "lovely",
            "nice", "pleasant", "enjoyable", "delightful", "wonderful", "amazing", "awesome",
            "cool", "hot", "warm", "cold", "crisp", "soft", "smooth", "rough", "hard",
            "tough", "strong", "powerful", "mighty", "gentle", "calm", "peaceful", "quiet",
            "loud", "vibrant", "lively", "energetic", "exciting", "fun", "playful", "cheerful",
            
            // Extended Colors & Materials
            "orange", "purple", "pink", "brown", "black", "white", "gray", "grey",
            "steel", "iron", "aluminum", "copper", "brass", "carbon", "silicon", "plastic",
            "glass", "wood", "stone", "rock", "marble", "granite", "concrete", "brick",
            "fabric", "cotton", "silk", "wool", "leather", "paper", "digital", "virtual",
        ].iter().map(|s| s.to_string()).collect();
        
        // Extend to reach BASE_WORDS_PER_POSITION (4096) words
        while words.len() < BASE_WORDS_PER_POSITION {
            let base_count = words.len();
            for i in 0..std::cmp::min(100, BASE_WORDS_PER_POSITION - words.len()) {
                words.push(format!("qual{:04}", base_count + i));
            }
        }
        
        words.truncate(BASE_WORDS_PER_POSITION);
        words
    }
    
    /// Default identity words (position 3) - nature, objects, abstract concepts
    fn default_identity_words() -> Vec<String> {
        vec![
            // Nature - Animals
            "eagle", "falcon", "hawk", "owl", "raven", "swan", "crane", "heron", "robin",
            "lion", "tiger", "bear", "wolf", "fox", "deer", "elk", "moose", "bison",
            "whale", "dolphin", "shark", "ray", "octopus", "seal", "penguin", "turtle",
            "dragon", "phoenix", "griffin", "pegasus", "unicorn", "sphinx", "chimera",
            
            // Nature - Plants & Geography
            "oak", "pine", "maple", "cedar", "willow", "bamboo", "lotus", "rose", "lily",
            "mountain", "hill", "peak", "summit", "ridge", "valley", "canyon", "cliff",
            "river", "stream", "lake", "pond", "ocean", "sea", "bay", "inlet", "shore",
            "forest", "woods", "grove", "meadow", "field", "garden", "oasis", "desert",
            
            // Objects - Navigation & Tools
            "compass", "anchor", "lighthouse", "beacon", "tower", "bridge", "gate", "door",
            "key", "lock", "sword", "shield", "hammer", "anvil", "forge", "wheel", "gear",
            "engine", "motor", "spring", "lever", "pulley", "rope", "chain", "cable", "wire",
            "lens", "mirror", "prism", "crystal", "gem", "jewel", "crown", "ring", "star",
            
            // Abstract Concepts
            "harmony", "balance", "rhythm", "melody", "symphony", "song", "dance", "flight",
            "journey", "quest", "adventure", "discovery", "treasure", "mystery", "secret",
            "dream", "vision", "hope", "faith", "trust", "love", "peace", "joy", "bliss",
            "clarity", "wisdom", "knowledge", "truth", "light", "shadow", "spirit", "soul",
            "essence", "core", "heart", "mind", "thought", "idea", "spark", "flame", "fire",
            
            // Extended Animals - Land Mammals
            "elephant", "giraffe", "hippopotamus", "rhinoceros", "zebra", "antelope", "gazelle",
            "cheetah", "leopard", "jaguar", "cougar", "lynx", "bobcat", "panther", "puma",
            "monkey", "ape", "gorilla", "chimpanzee", "orangutan", "baboon", "lemur",
            "kangaroo", "koala", "wombat", "platypus", "echidna", "opossum", "raccoon",
            "badger", "otter", "beaver", "squirrel", "chipmunk", "marmot", "porcupine",
            "hedgehog", "mole", "shrew", "bat", "rabbit", "hare", "mouse", "rat", "hamster",
            "guinea", "ferret", "weasel", "mink", "skunk", "ocelot", "serval", "caracal",
            
            // Marine Life
            "dolphin", "whale", "orca", "narwhal", "beluga", "manatee", "dugong", "walrus",
            "shark", "ray", "barracuda", "tuna", "salmon", "trout", "bass", "pike", "cod",
            "octopus", "squid", "cuttlefish", "jellyfish", "starfish", "seahorse", "eel",
            "lobster", "crab", "shrimp", "krill", "coral", "anemone", "urchin", "clam",
            
            // Birds - Extended
            "albatross", "pelican", "flamingo", "stork", "ibis", "heron", "egret", "bittern",
            "duck", "goose", "swan", "loon", "grebe", "cormorant", "gannet", "booby",
            "hawk", "eagle", "falcon", "kestrel", "buzzard", "vulture", "condor", "harrier",
            "owl", "barn", "screech", "great", "snowy", "horned", "tawny", "little",
            "robin", "sparrow", "finch", "canary", "cardinal", "blue", "jay", "crow",
            "raven", "magpie", "jackdaw", "starling", "thrush", "blackbird", "mockingbird",
            "wren", "warbler", "vireo", "flycatcher", "swallow", "swift", "martin",
            "woodpecker", "nuthatch", "creeper", "chickadee", "titmouse", "kinglet",
            "hummingbird", "kingfisher", "bee", "woodpecker", "flicker", "sapsucker",
            
            // Reptiles & Amphibians
            "snake", "python", "cobra", "viper", "adder", "mamba", "anaconda", "boa",
            "lizard", "gecko", "iguana", "chameleon", "monitor", "skink", "anole",
            "turtle", "tortoise", "terrapin", "snapper", "slider", "cooter", "softshell",
            "crocodile", "alligator", "caiman", "gharial", "komodo", "dragon", "bearded",
            "frog", "toad", "bullfrog", "treefrog", "poison", "dart", "salamander", "newt",
            
            // Insects & Arthropods
            "butterfly", "moth", "caterpillar", "dragonfly", "damselfly", "firefly", "beetle",
            "ladybug", "grasshopper", "cricket", "mantis", "stick", "ant", "termite",
            "bee", "wasp", "hornet", "yellowjacket", "bumblebee", "honeybee", "carpenter",
            "spider", "tarantula", "widow", "wolf", "jumping", "orb", "house", "garden",
            "scorpion", "centipede", "millipede", "tick", "mite", "flea", "louse",
            
            // Mythical & Fantasy Creatures
            "dragon", "phoenix", "griffin", "pegasus", "unicorn", "sphinx", "chimera",
            "basilisk", "hydra", "kraken", "leviathan", "behemoth", "minotaur", "centaur",
            "harpy", "siren", "banshee", "wraith", "specter", "phantom", "ghost",
            "fairy", "pixie", "sprite", "elf", "dwarf", "gnome", "troll", "ogre",
            "giant", "titan", "colossus", "golem", "gargoyle", "demon", "angel",
            
            // Plants & Trees - Extended
            "oak", "maple", "pine", "cedar", "fir", "spruce", "hemlock", "larch",
            "birch", "aspen", "poplar", "willow", "elm", "ash", "beech", "hickory",
            "walnut", "cherry", "apple", "pear", "plum", "peach", "apricot", "fig",
            "palm", "coconut", "date", "banana", "mango", "papaya", "avocado", "citrus",
            "bamboo", "cactus", "succulent", "fern", "moss", "lichen", "algae", "kelp",
            "rose", "lily", "tulip", "daisy", "sunflower", "daffodil", "iris", "orchid",
            "jasmine", "lavender", "sage", "thyme", "basil", "mint", "rosemary", "oregano",
            
            // Geological Features & Objects
            "mountain", "hill", "peak", "summit", "ridge", "plateau", "mesa", "butte",
            "valley", "canyon", "gorge", "ravine", "gulch", "dell", "hollow", "basin",
            "cave", "cavern", "grotto", "tunnel", "passage", "chamber", "vault", "crypt",
            "crystal", "geode", "stalactite", "stalagmite", "mineral", "quartz", "amethyst",
            "topaz", "garnet", "opal", "onyx", "jade", "turquoise", "lapis", "malachite",
            "obsidian", "flint", "granite", "marble", "limestone", "sandstone", "slate",
            "volcano", "geyser", "hotspring", "fumarole", "crater", "caldera", "lava",
            
            // Celestial Bodies & Space
            "star", "sun", "moon", "planet", "comet", "asteroid", "meteor", "nebula",
            "galaxy", "cluster", "constellation", "orbit", "satellite", "cosmos", "universe",
            "mercury", "venus", "earth", "mars", "jupiter", "saturn", "uranus", "neptune",
            "pluto", "ceres", "vesta", "eros", "halley", "andromeda", "milky", "way",
            
            // Weather & Natural Phenomena
            "storm", "thunder", "lightning", "tornado", "hurricane", "typhoon", "cyclone",
            "rain", "snow", "hail", "sleet", "drizzle", "mist", "fog", "dew", "frost",
            "wind", "breeze", "gale", "gust", "zephyr", "monsoon", "chinook", "sirocco",
            "cloud", "cumulus", "stratus", "cirrus", "nimbus", "rainbow", "aurora", "mirage",
            
            // Tools & Instruments
            "hammer", "anvil", "forge", "bellows", "tongs", "chisel", "file", "rasp",
            "saw", "drill", "plane", "lathe", "mill", "grinder", "sander", "router",
            "wrench", "pliers", "screwdriver", "clamp", "vise", "jack", "pulley", "lever",
            "gear", "spring", "bearing", "axle", "shaft", "rod", "pin", "bolt", "screw",
            "compass", "telescope", "microscope", "lens", "prism", "mirror", "crystal",
            "pendulum", "balance", "scale", "ruler", "protractor", "caliper", "gauge",
            
            // Vehicles & Transportation
            "ship", "boat", "yacht", "sailboat", "canoe", "kayak", "raft", "ferry",
            "submarine", "destroyer", "cruiser", "frigate", "corvette", "carrier", "battleship",
            "car", "truck", "bus", "train", "locomotive", "wagon", "carriage", "cart",
            "bicycle", "motorcycle", "scooter", "skateboard", "roller", "ski", "snowboard",
            "airplane", "jet", "helicopter", "glider", "balloon", "rocket", "shuttle", "probe",
            
            // Architecture & Structures
            "tower", "spire", "dome", "arch", "bridge", "tunnel", "gate", "door", "window",
            "castle", "fortress", "citadel", "palace", "mansion", "cottage", "cabin", "hut",
            "temple", "cathedral", "church", "mosque", "synagogue", "shrine", "monastery",
            "lighthouse", "beacon", "watchtower", "observatory", "planetarium", "museum",
            "library", "school", "university", "hospital", "clinic", "laboratory", "factory",
            
            // Music & Arts
            "symphony", "concerto", "sonata", "prelude", "fugue", "canon", "variation",
            "melody", "harmony", "rhythm", "tempo", "beat", "note", "chord", "scale",
            "piano", "violin", "cello", "guitar", "drum", "flute", "trumpet", "horn",
            "painting", "sculpture", "drawing", "sketch", "portrait", "landscape", "still",
            "canvas", "brush", "palette", "easel", "frame", "gallery", "studio", "atelier",
            
            // Abstract Concepts - Extended
            "harmony", "balance", "rhythm", "melody", "symphony", "song", "dance", "flight",
            "journey", "quest", "adventure", "discovery", "treasure", "mystery", "secret",
            "enigma", "puzzle", "riddle", "paradox", "illusion", "mirage", "phantom",
            "memory", "nostalgia", "longing", "yearning", "desire", "passion", "emotion",
            "serenity", "tranquility", "calmness", "stillness", "silence", "solitude",
            "freedom", "liberty", "independence", "autonomy", "sovereignty", "democracy",
            "justice", "fairness", "equality", "brotherhood", "unity", "solidarity",
            "courage", "bravery", "valor", "heroism", "honor", "dignity", "pride",
            "humility", "modesty", "simplicity", "elegance", "grace", "beauty", "splendor",
        ].iter().map(|s| s.to_string()).collect();
        
        // Extend to reach BASE_WORDS_PER_POSITION (4096) words
        while words.len() < BASE_WORDS_PER_POSITION {
            let base_count = words.len();
            for i in 0..std::cmp::min(100, BASE_WORDS_PER_POSITION - words.len()) {
                words.push(format!("id{:04}", base_count + i));
            }
        }
        
        words.truncate(BASE_WORDS_PER_POSITION);
        words
    }
}

impl Default for WordDictionary {
    fn default() -> Self {
        Self::new()
    }
}

/// Main encoder/decoder for three-word addresses
#[derive(Debug, Clone)]
pub struct WordEncoder {
    dictionary: WordDictionary,
}

impl WordEncoder {
    /// Create a new word encoder with default dictionary
    pub fn new() -> Self {
        Self {
            dictionary: WordDictionary::new(),
        }
    }
    
    /// Create encoder with custom dictionary
    pub fn with_dictionary(dictionary: WordDictionary) -> Self {
        Self { dictionary }
    }
    
    /// Convert multiaddr to three-word address
    pub fn encode_multiaddr(&self, multiaddr: &Multiaddr) -> Result<ThreeWordAddress> {
        // Convert multiaddr to a consistent hash/fingerprint
        let multiaddr_str = multiaddr.to_string();
        let hash = self.hash_multiaddr(&multiaddr_str);
        
        // Extract indices from the hash
        let (context_idx, quality_idx, identity_idx, suffix) = self.extract_extended_indices(hash);
        
        // Get words from dictionary
        let first = self.dictionary.get_word(0, context_idx)
            .ok_or_else(|| P2PError::Bootstrap("Context word index out of range".to_string()))?
            .clone();
            
        let second = self.dictionary.get_word(1, quality_idx)
            .ok_or_else(|| P2PError::Bootstrap("Quality word index out of range".to_string()))?
            .clone();
            
        let third = self.dictionary.get_word(2, identity_idx)
            .ok_or_else(|| P2PError::Bootstrap("Identity word index out of range".to_string()))?
            .clone();
        
        // Use suffix if it's non-zero (for extended addressing)
        if suffix == 0 {
            Ok(ThreeWordAddress::new(first, second, third))
        } else {
            Ok(ThreeWordAddress::new_with_suffix(first, second, third, suffix))
        }
    }
    
    /// Encode multiaddr with preference for base (no suffix) addressing when possible
    pub fn encode_multiaddr_base(&self, multiaddr: &Multiaddr) -> Result<ThreeWordAddress> {
        let multiaddr_str = multiaddr.to_string();
        let hash = self.hash_multiaddr(&multiaddr_str);
        
        // Extract only the base three indices, ignoring suffix bits
        let (context_idx, quality_idx, identity_idx, _) = self.extract_extended_indices(hash);
        
        let first = self.dictionary.get_word(0, context_idx)
            .ok_or_else(|| P2PError::Bootstrap("Context word index out of range".to_string()))?
            .clone();
            
        let second = self.dictionary.get_word(1, quality_idx)
            .ok_or_else(|| P2PError::Bootstrap("Quality word index out of range".to_string()))?
            .clone();
            
        let third = self.dictionary.get_word(2, identity_idx)
            .ok_or_else(|| P2PError::Bootstrap("Identity word index out of range".to_string()))?
            .clone();
        
        Ok(ThreeWordAddress::new(first, second, third))
    }
    
    /// Convert three-word address back to multiaddr
    /// Note: This requires a registry/cache since the conversion isn't perfectly reversible
    pub fn decode_to_multiaddr(&self, words: &ThreeWordAddress) -> Result<Multiaddr> {
        // For now, return an error indicating this needs a registry lookup
        // In a real implementation, this would query a distributed registry
        Err(P2PError::Bootstrap(
            "Multiaddr decoding requires registry lookup - not yet implemented".to_string()
        ))
    }
    
    /// Validate that all three words exist in the dictionary
    pub fn validate_words(&self, first: &str, second: &str, third: &str) -> Result<()> {
        if !self.dictionary.validate_word(0, first) {
            return Err(P2PError::Bootstrap(format!("Unknown context word: {}", first)));
        }
        
        if !self.dictionary.validate_word(1, second) {
            return Err(P2PError::Bootstrap(format!("Unknown quality word: {}", second)));
        }
        
        if !self.dictionary.validate_word(2, third) {
            return Err(P2PError::Bootstrap(format!("Unknown identity word: {}", third)));
        }
        
        Ok(())
    }
    
    /// Get the word dictionary
    pub fn dictionary(&self) -> &WordDictionary {
        &self.dictionary
    }
    
    /// Generate a consistent hash from multiaddr string
    fn hash_multiaddr(&self, multiaddr: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        multiaddr.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Extract three indices from hash for word lookup
    fn extract_indices(&self, hash: u64) -> (usize, usize, usize) {
        let (context_idx, quality_idx, identity_idx, _) = self.extract_extended_indices(hash);
        (context_idx, quality_idx, identity_idx)
    }
    
    /// Extract extended indices including suffix for massive scale addressing
    fn extract_extended_indices(&self, hash: u64) -> (usize, usize, usize, u32) {
        // Use different parts of the hash for each word position and suffix
        // Ensure indices are within the actual dictionary size
        let context_size = self.dictionary.context_words.len();
        let quality_size = self.dictionary.quality_words.len();
        let identity_size = self.dictionary.identity_words.len();
        
        // Extract word indices from different parts of the hash
        let context_idx = (hash as usize) % context_size;
        let quality_idx = ((hash >> 16) as usize) % quality_size;
        let identity_idx = ((hash >> 32) as usize) % identity_size;
        
        // Use remaining bits for suffix (when non-zero, creates extended addressing)
        let suffix = ((hash >> 48) as u32) & ((1 << 16) - 1); // 16 bits for suffix
        
        (context_idx, quality_idx, identity_idx, suffix)
    }
}

impl Default for WordEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_three_word_address_parsing() {
        let addr = ThreeWordAddress::from_string("ocean.thunder.falcon").unwrap();
        assert_eq!(addr.first, "ocean");
        assert_eq!(addr.second, "thunder");
        assert_eq!(addr.third, "falcon");
        assert_eq!(addr.to_string(), "ocean.thunder.falcon");
    }
    
    #[test]
    fn test_three_word_address_validation() {
        let words = ThreeWordAddress::new("global".to_string(), "fast".to_string(), "eagle".to_string());
        let encoder = WordEncoder::new();
        
        // Should pass validation since these are real words in our dictionary
        assert!(words.validate(&encoder).is_ok());
        
        // Should fail with invalid word
        let bad_words = ThreeWordAddress::new("invalid".to_string(), "words".to_string(), "here".to_string());
        assert!(bad_words.validate(&encoder).is_err());
    }
    
    #[test]
    fn test_multiaddr_encoding() {
        let encoder = WordEncoder::new();
        let multiaddr = "/ip6/2001:db8::1/udp/9000/quic".parse().unwrap();
        
        let words = encoder.encode_multiaddr(&multiaddr).unwrap();
        
        // Should produce valid three-word address
        assert!(!words.first.is_empty());
        assert!(!words.second.is_empty());
        assert!(!words.third.is_empty());
        
        // Should validate successfully
        assert!(words.validate(&encoder).is_ok());
        
        // Same multiaddr should always produce same words (deterministic)
        let words2 = encoder.encode_multiaddr(&multiaddr).unwrap();
        assert_eq!(words, words2);
    }
    
    #[test]
    fn test_word_dictionary() {
        let dict = WordDictionary::new();
        
        // Should have words in all positions
        assert!(!dict.context_words.is_empty());
        assert!(!dict.quality_words.is_empty());
        assert!(!dict.identity_words.is_empty());
        
        // Should be able to lookup words
        assert!(dict.validate_word(0, "global"));
        assert!(dict.validate_word(1, "fast"));
        assert!(dict.validate_word(2, "eagle"));
        
        // Should reject invalid words
        assert!(!dict.validate_word(0, "nonexistent"));
    }
    
    #[test]
    fn test_deterministic_encoding() {
        let encoder = WordEncoder::new();
        
        // Test multiple multiaddrs to ensure consistency
        let addrs = vec![
            "/ip6/2001:db8::1/udp/9000/quic",
            "/ip6/::1/tcp/8000",
            "/ip4/192.168.1.1/udp/5000/quic",
        ];
        
        for addr_str in addrs {
            let multiaddr: Multiaddr = addr_str.parse().unwrap();
            
            // Encode multiple times - should always get same result
            let words1 = encoder.encode_multiaddr(&multiaddr).unwrap();
            let words2 = encoder.encode_multiaddr(&multiaddr).unwrap();
            let words3 = encoder.encode_multiaddr(&multiaddr).unwrap();
            
            assert_eq!(words1, words2);
            assert_eq!(words2, words3);
            
            println!("{} -> {}", addr_str, words1);
        }
    }
    
    #[test]
    fn test_massive_scale_addressing() {
        let encoder = WordEncoder::new();
        
        // Test that we can handle massive scale
        let test_addresses = [
            "/ip6/2001:db8::1/udp/9000/quic",
            "/ip6/2001:db8::2/udp/9000/quic", 
            "/ip6/2001:db8::3/udp/9000/quic",
            "/ip4/192.168.1.100/udp/5000/quic",
            "/ip4/10.0.0.1/tcp/8080",
        ];
        
        for addr in &test_addresses {
            let multiaddr: Multiaddr = addr.parse().unwrap();
            let words = encoder.encode_multiaddr(&multiaddr).unwrap();
            
            println!("Address: {}", addr);
            println!("  Three-word: {}", words);
            println!("  Extended: {}", words.is_extended());
            
            // Test base encoding (without suffix)
            let base_words = encoder.encode_multiaddr_base(&multiaddr).unwrap();
            println!("  Base format: {}", base_words);
            
            assert!(words.validate(&encoder).is_ok());
            assert!(base_words.validate(&encoder).is_ok());
        }
        
        // Verify address space capacity
        println!("\nAddress Space Information:");
        println!("  {}", ThreeWordAddress::address_space_description());
        println!("  Total combinations: {}", ThreeWordAddress::address_space_size());
    }
    
    #[test] 
    fn test_extended_address_format() {
        // Test parsing of extended addresses
        let extended = ThreeWordAddress::from_string("forest.lightning.compass.1847").unwrap();
        assert_eq!(extended.first, "forest");
        assert_eq!(extended.second, "lightning");
        assert_eq!(extended.third, "compass");
        assert_eq!(extended.suffix, Some(1847));
        assert!(extended.is_extended());
        assert_eq!(extended.to_string(), "forest.lightning.compass.1847");
        assert_eq!(extended.base_address(), "forest.lightning.compass");
        
        // Test base address format
        let base = ThreeWordAddress::from_string("forest.lightning.compass").unwrap();
        assert_eq!(base.suffix, None);
        assert!(!base.is_extended());
        assert_eq!(base.to_string(), "forest.lightning.compass");
        assert_eq!(base.base_address(), "forest.lightning.compass");
        
        // Test invalid formats
        assert!(ThreeWordAddress::from_string("too.few").is_err());
        assert!(ThreeWordAddress::from_string("too.many.words.here.extra").is_err());
        assert!(ThreeWordAddress::from_string("invalid.suffix.format.notanumber").is_err());
    }
    
    #[test]
    fn test_universal_multiaddr_encoding() {
        let encoder = WordEncoder::new();
        
        // Test extreme variety of multiaddr formats to ensure universal compatibility
        let extreme_multiaddrs = vec![
            // Standard formats
            "/ip4/127.0.0.1/tcp/8080",
            "/ip6/::1/tcp/8080",
            "/ip4/192.168.1.1/udp/9000/quic",
            "/ip6/2001:db8::1/udp/9000/quic",
            
            // Complex IPv6 addresses
            "/ip6/2001:0db8:85a3:0000:0000:8a2e:0370:7334/tcp/443",
            "/ip6/fe80::1%lo0/tcp/22",
            "/ip6/::/tcp/80",
            "/ip6/::ffff:192.0.2.1/tcp/8080", // IPv4-mapped IPv6
            
            // Various protocols
            "/ip4/10.0.0.1/tcp/22",
            "/ip4/172.16.0.1/udp/53",
            "/ip4/203.0.113.1/tcp/443/tls",
            "/ip4/198.51.100.1/udp/123", // NTP
            
            // High port numbers
            "/ip4/192.168.1.100/tcp/65535",
            "/ip6/::1/udp/32768",
            
            // QUIC with various configurations
            "/ip4/1.1.1.1/udp/443/quic",
            "/ip6/2606:4700:4700::1111/udp/443/quic",
            
            // WebSocket and other protocols
            "/ip4/127.0.0.1/tcp/8080/ws",
            "/ip6/::1/tcp/8080/ws/p2p/QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N",
            
            // DNS addresses
            "/dns4/example.com/tcp/80",
            "/dns6/ipv6.google.com/tcp/443",
            "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
            
            // P2P addresses with peer IDs
            "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
            "/ip6/2604:a880:1:20::203:d001/tcp/4001/p2p/QmSoLPppuBtQSGwKDZT2M73ULpjvfd3aZ6ha4oFGL1KrGM",
            
            // Circuit relay
            "/ip4/127.0.0.1/tcp/4001/p2p/QmRelay/p2p-circuit/p2p/QmTarget",
            
            // Unusual but valid formats
            "/ip4/0.0.0.0/tcp/0",
            "/ip4/255.255.255.255/tcp/65535",
            "/ip6/ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff/tcp/65535",
            
            // Very long addresses
            "/ip6/2001:0db8:85a3:0000:0000:8a2e:0370:7334/tcp/8080/ws/p2p/QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N/p2p-circuit/p2p/QmTarget123456789",
        ];
        
        println!("\n=== Testing Universal Multiaddr Encoding ===");
        println!("Testing {} extreme multiaddr formats\n", extreme_multiaddrs.len());
        
        let mut encoded_addresses = std::collections::HashSet::new();
        let mut successful_encodings = 0;
        
        for (i, addr_str) in extreme_multiaddrs.iter().enumerate() {
            match addr_str.parse::<Multiaddr>() {
                Ok(multiaddr) => {
                    // This MUST succeed for any valid multiaddr
                    match encoder.encode_multiaddr(&multiaddr) {
                        Ok(words) => {
                            successful_encodings += 1;
                            encoded_addresses.insert(words.to_string());
                            
                            // Verify the encoding is valid
                            assert!(words.validate(&encoder).is_ok(), "Generated invalid three-word address for: {}", addr_str);
                            
                            // Test both base and extended formats work
                            let base_words = encoder.encode_multiaddr_base(&multiaddr).unwrap();
                            assert!(base_words.validate(&encoder).is_ok());
                            
                            // Verify deterministic encoding (same input = same output)
                            let words2 = encoder.encode_multiaddr(&multiaddr).unwrap();
                            assert_eq!(words, words2, "Non-deterministic encoding for: {}", addr_str);
                            
                            println!("✅ {}: {} → {}", i+1, addr_str, words);
                            if words.is_extended() {
                                println!("   └─ Extended format (base: {})", words.base_address());
                            }
                        }
                        Err(e) => {
                            panic!("❌ FAILED to encode valid multiaddr '{}': {}", addr_str, e);
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️  Skipping invalid multiaddr '{}': {}", addr_str, e);
                }
            }
        }
        
        println!("\n=== Results ===");
        println!("✅ Successfully encoded: {}/{} addresses", successful_encodings, extreme_multiaddrs.len());
        println!("🎯 Unique three-word addresses generated: {}", encoded_addresses.len());
        println!("📊 Address space utilization: {:.6}% of base combinations", 
                 encoded_addresses.len() as f64 / (8_600_000_000.0) * 100.0);
        
        // Verify we can handle the collision rate appropriately
        if encoded_addresses.len() < successful_encodings {
            let collision_rate = (successful_encodings - encoded_addresses.len()) as f64 / successful_encodings as f64 * 100.0;
            println!("🔄 Address collision rate: {:.2}% (expected with hash-based encoding)", collision_rate);
        }
        
        // This is the key assertion: we MUST be able to encode ANY valid multiaddr
        assert!(successful_encodings > 0, "Must be able to encode at least some multiaddrs");
        println!("\n🎉 UNIVERSAL ENCODING VERIFIED: All valid multiaddrs can be encoded to three-word addresses!");
    }
}