#!/bin/bash
# Fix missing Debug implementations in four-word-networking

cd /Users/davidirvine/Desktop/Devel/projects/four-word-networking

# Fix ipv6_compression.rs line 75
sed -i '' '75s/pub struct IPv6Compressor;/#[derive(Debug)]\npub struct IPv6Compressor;/' src/ipv6_compression.rs

# Fix ipv6_pattern_feistel.rs line 75
sed -i '' '75s/pub struct IPv6PatternFeistel;/#[derive(Debug)]\npub struct IPv6PatternFeistel;/' src/ipv6_pattern_feistel.rs

# Fix ipv6_perfect_patterns.rs line 160
sed -i '' '160s|// pub struct|#[derive(Debug)]\npub struct|' src/ipv6_perfect_patterns.rs

# Fix ipv6_perfect_patterns.rs line 605
sed -i '' '605s|// pub struct|#[derive(Debug)]\npub struct|' src/ipv6_perfect_patterns.rs

# Fix pure_ip_compression.rs line 13
sed -i '' '13s/pub struct PureIpCompressor;/#[derive(Debug)]\npub struct PureIpCompressor;/' src/pure_ip_compression.rs

# Fix pure_ip_compression.rs line 248
sed -i '' '248s/pub struct MathematicalCompressor;/#[derive(Debug)]\npub struct MathematicalCompressor;/' src/pure_ip_compression.rs

# Fix universal_ip_compression.rs line 15
sed -i '' '15s|// pub struct|#[derive(Debug)]\npub struct|' src/universal_ip_compression.rs

echo "Fixed Debug implementations in four-word-networking"