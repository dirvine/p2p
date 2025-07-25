# Licensing Guide

## Dual License Structure

P2P Foundation is dual-licensed to support both open-source and commercial use:

### Open Source License (AGPL-3.0)
- **License**: GNU Affero General Public License v3.0 or later
- **File**: [LICENSE-AGPL-3.0](LICENSE-AGPL-3.0)
- **Use Cases**:
  - Open source projects
  - Personal use
  - Educational purposes
  - Non-commercial applications
- **Requirements**:
  - Source code disclosure for all modifications
  - Network use provisions apply (Section 13)
  - Must include original copyright notices

### Commercial License
- **Contact**: saorsalabs@gmail.com
- **Use Cases**:
  - Proprietary applications
  - Commercial products
  - Organizations with >$1M annual revenue
  - When AGPL requirements cannot be met
- **Benefits**:
  - No source code disclosure required
  - Professional support included
  - Priority feature requests
  - Custom integration assistance

## Quick Decision Guide

**You need a commercial license if**:
- You're building a proprietary product
- You're charging users for your service
- Your organization has >$1M annual revenue
- You cannot comply with AGPL source disclosure requirements
- You need professional support

**You can use AGPL license if**:
- Your project is open source
- You're willing to share all source code modifications
- You're using it for personal/educational purposes
- You're a non-profit organization

## License Headers

All source files should include the following header:

```rust
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
```

## Compliance Requirements

### AGPL Compliance
1. Include LICENSE-AGPL-3.0 file in distributions
2. Provide source code access to all users
3. Include copyright notices
4. Document all modifications
5. Apply AGPL to derivative works

### Commercial License Compliance
1. Valid license agreement required
2. Annual renewal for continued use
3. Usage reporting as specified in agreement
4. Confidentiality of proprietary features

## FAQ

**Q: Can I use this in my startup?**
A: Yes, under AGPL if you comply with source disclosure, or with a commercial license.

**Q: What about internal corporate use?**
A: Internal use requires commercial license for organizations >$1M revenue.

**Q: Can I contribute while using commercial license?**
A: Yes! Contributions are welcomed under our CLA.

**Q: How much does commercial license cost?**
A: Contact saorsalabs@gmail.com for pricing based on your use case.

## Contact

For licensing questions:
- Email: saorsalabs@gmail.com
- Include: Organization name, use case, expected scale

## License Feature Flags

The codebase supports license-specific features:

```toml
# For AGPL builds (default)
cargo build

# For commercial builds
cargo build --features commercial

# To ensure AGPL compliance
cargo build --features agpl-compliance
```

## Third-Party Licenses

See [THIRD-PARTY-LICENSES.md](THIRD-PARTY-LICENSES.md) for dependencies.