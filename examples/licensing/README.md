# P2P Foundation Licensing Examples

This directory contains examples demonstrating different licensing scenarios for P2P Foundation.

## Examples

### 1. AGPL Compliance (`agpl-compliance.rs`)

Demonstrates how to build an AGPL-3.0 compliant application:
- Source code disclosure (Section 13)
- License notices and attribution
- Web server with compliance endpoints
- Best practices for open source

**Run:**
```bash
cargo run --bin agpl-compliance
```

Then visit:
- http://localhost:8080 - Main application
- http://localhost:8080/source - Source code access
- http://localhost:8080/license - License text
- http://localhost:8080/api - API documentation

### 2. Commercial Integration (`commercial-integration.rs`)

Shows how to integrate P2P Foundation with a commercial license:
- Loading license from file or environment
- Feature detection and gating
- Commercial-only features demo
- License enforcement examples

**Run:**
```bash
# With license file
P2P_LICENSE_PATH=/path/to/license.json cargo run --bin commercial-integration

# With license key
P2P_LICENSE_KEY=XXXX-XXXX-XXXX-XXXX cargo run --bin commercial-integration

# Demo mode (no license)
cargo run --bin commercial-integration
```

### 3. License Migration (`license-migration.rs`)

Interactive example showing license migration scenarios:
- AGPL → Commercial migration
- Commercial → AGPL migration
- Commercial tier upgrades
- License status checking

**Run:**
```bash
cargo run --bin license-migration
```

## License Files

### Creating a Demo License

For testing, you can create a demo license file:

```json
{
  "key": "DEMO-1234-5678-90AB-CDEF",
  "organization": "Your Company",
  "tier": "Enterprise",
  "issued_at": "2024-01-01T00:00:00Z",
  "expires_at": "2025-01-01T00:00:00Z",
  "max_users": null,
  "contact_email": "admin@example.com",
  "metadata": {}
}
```

Save this as `license.json` and use with:
```bash
P2P_LICENSE_PATH=license.json cargo run --bin commercial-integration
```

### License File Locations

The examples check for licenses in this order:
1. `P2P_LICENSE_PATH` environment variable
2. Default location: `~/.config/p2p-foundation/license.json`
3. `P2P_LICENSE_KEY` environment variable
4. Demo license (if none found)

## Features by License Type

| Feature | AGPL | SMB | Enterprise | OEM |
|---------|------|-----|------------|-----|
| Core P2P | ✅ | ✅ | ✅ | ✅ |
| User Limit | ∞ | 50 | ∞ | ∞ |
| Priority Bootstrap | ❌ | ✅ | ✅ | ✅ |
| Analytics | ❌ | ❌ | ✅ | ✅ |
| API Access | ❌ | ✅ | ✅ | ✅ |
| Premium Support | ❌ | ❌ | ✅ | ✅ |
| White Label | ❌ | ❌ | ❌ | ✅ |

## Common Use Cases

### SaaS Application
If building a SaaS application:
- Use `commercial-integration.rs` as a starting point
- Implement license checking on startup
- Gate features based on license tier
- Add usage tracking for compliance

### Open Source Project
For open source projects:
- Follow `agpl-compliance.rs` example
- Ensure source code availability
- Add proper attribution
- Include license notices

### Internal Enterprise Tool
For internal corporate use:
- Obtain commercial license
- Use environment variables for license distribution
- Implement user counting if on SMB tier
- Consider Enterprise tier for unlimited users

## Testing

Run all examples:
```bash
# Test AGPL compliance
cargo test --bin agpl-compliance

# Test commercial features
cargo test --bin commercial-integration --features commercial

# Test migrations
cargo test --bin license-migration
```

## Support

- **Licensing questions**: saorsalabs@gmail.com
- **Technical issues**: GitHub issues
- **Commercial support**: Based on your license tier

## License

These examples are dual-licensed:
- AGPL-3.0 for open source use
- Commercial license for proprietary use

See the main project LICENSE files for details.