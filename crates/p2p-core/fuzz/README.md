# Fuzzing Tests for saorsa-core

This directory contains fuzzing tests for security-critical input validation and parsing functions.

## Prerequisites

Install cargo-fuzz:
```bash
cargo install cargo-fuzz
```

## Running Fuzz Tests

To run a specific fuzz target:
```bash
cd crates/p2p-core
cargo fuzz run fuzz_validation
```

Available fuzz targets:
- `fuzz_validation` - Tests all input validation functions
- `fuzz_address_parsing` - Tests three-word address parsing
- `fuzz_network_messages` - Tests network message parsing and validation
- `fuzz_dht_operations` - Tests DHT key/value operations

## Running with Options

Run for a specific duration:
```bash
cargo fuzz run fuzz_validation -- -max_total_time=60
```

Run with more workers:
```bash
cargo fuzz run fuzz_validation -- -workers=4
```

## Analyzing Crashes

If fuzzing finds a crash, it will be saved in `fuzz/artifacts/`. To reproduce:
```bash
cargo fuzz run fuzz_validation fuzz/artifacts/fuzz_validation/crash-<hash>
```

## Coverage

To generate coverage report:
```bash
cargo fuzz coverage fuzz_validation
cargo cov -- show target/x86_64-unknown-linux-gnu/release/fuzz_validation \
    --instr-profile=fuzz/coverage/fuzz_validation/coverage.profdata
```

## Security Focus

These fuzz tests specifically target:
1. **Input Validation** - Ensuring validators handle malformed input gracefully
2. **Parser Safety** - Testing parsers with random/malformed data
3. **Bounds Checking** - Verifying size limits are enforced
4. **Error Handling** - Confirming no panics on invalid input

## Integration with CI

Add to CI pipeline:
```yaml
- name: Run Fuzz Tests
  run: |
    cargo install cargo-fuzz
    cd crates/p2p-core
    cargo fuzz run fuzz_validation -- -max_total_time=300
    cargo fuzz run fuzz_address_parsing -- -max_total_time=300
    cargo fuzz run fuzz_network_messages -- -max_total_time=300
    cargo fuzz run fuzz_dht_operations -- -max_total_time=300
```