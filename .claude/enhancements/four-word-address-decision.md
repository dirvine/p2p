# Four-Word Address Implementation Decision

## Context
Task 4 was to integrate the four-word-networking crate v1.2. However, investigation revealed:

1. **Crate Purpose Mismatch**: The four-word-networking crate is designed specifically for encoding IP addresses into memorable words, not arbitrary bytes
2. **Our Requirements**: We need to encode 32-byte node IDs (from cryptographic keys) into four-word addresses
3. **Existing Code**: We have a working placeholder implementation that meets our needs

## Decision
Keep and enhance the existing placeholder implementation rather than forcing integration with an incompatible crate.

## Rationale
1. The four-word-networking crate uses a different encoding strategy optimized for IP addresses
2. Our use case (32-byte node IDs) is fundamentally different
3. The placeholder already works and just needs production hardening

## Action Items
1. ✅ Remove the TODO comments about integrating four-word-networking
2. ✅ Remove the commented dependency from Cargo.toml
3. ✅ Enhance the existing implementation with:
   - Larger, more carefully curated word list (4096 words like the real crate)
   - Better word selection algorithm
   - Proper decoding support
   - Comprehensive tests

## Future Consideration
If we need to encode actual IP addresses elsewhere in the codebase, we can add four-word-networking as an additional dependency specifically for that use case.