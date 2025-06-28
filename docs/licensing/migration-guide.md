# License Migration Guide

This guide helps you migrate between different licensing options for P2P Foundation.

## Migration Scenarios

### 1. AGPL to Commercial License

**When to migrate**:
- You need to keep modifications proprietary
- You're building a commercial product
- Your organization exceeds $1M annual revenue
- You cannot comply with AGPL source disclosure

**Migration steps**:

#### Step 1: Assess Current Usage
```bash
# Check your current integration points
grep -r "use p2p" src/
grep -r "use ant_core" src/

# List modifications to P2P Foundation
git log --oneline -- vendor/p2p/
```

#### Step 2: Purchase Commercial License
1. Contact saorsalabs@gmail.com
2. Provide organization details
3. Select appropriate tier (SMB/Enterprise/OEM)
4. Complete purchase process

#### Step 3: Update License References
```toml
# Cargo.toml
[package]
license = "Proprietary"  # Your license

[dependencies]
# P2P Foundation now used under commercial license
ant-core = "0.1.8"
```

#### Step 4: Remove AGPL Obligations
- Remove public source code repository (or make private)
- Remove source code links from application
- Update documentation to reflect commercial licensing
- Remove AGPL license files

#### Step 5: Configure Commercial Features
```rust
use p2p_foundation::licensing::{LicenseChecker, CommercialLicense};

fn initialize_app() -> Result<()> {
    // Load commercial license
    let license = CommercialLicense::load_from_file("license.json")?;
    
    // Initialize with commercial features
    let checker = LicenseChecker::with_status(license.to_status(0));
    
    // Now you can use commercial-only features
    Ok(())
}
```

### 2. Commercial to AGPL License

**When to migrate**:
- You're open-sourcing your project
- Commercial license no longer needed
- Cost reduction initiative
- Philosophical alignment with open source

**Migration steps**:

#### Step 1: Understand AGPL Requirements
- Must release ALL source code
- Must provide source to network users
- Cannot keep modifications private
- Must license entire project under AGPL

#### Step 2: Prepare Source Code
```bash
# Remove any proprietary code that can't be open-sourced
git rm src/proprietary/*

# Add AGPL headers to all files
find src -name "*.rs" -exec sed -i '1i\
// SPDX-License-Identifier: AGPL-3.0-or-later' {} \;
```

#### Step 3: Update Licensing
```toml
# Cargo.toml
[package]
license = "AGPL-3.0-or-later"
```

#### Step 4: Publish Source Code
```bash
# Create public repository
git remote add public https://github.com/yourorg/project.git
git push public main

# Add source link to application
echo "Source: https://github.com/yourorg/project" >> README.md
```

#### Step 5: Add Compliance Features
```rust
// Add source code link to UI
fn render_footer() -> Html {
    html! {
        <footer>
            <a href="https://github.com/yourorg/project">
                {"Source Code (AGPL-3.0)"}
            </a>
        </footer>
    }
}
```

### 3. Upgrading Commercial Tiers

**SMB to Enterprise**:

#### Step 1: Contact Sales
```
Email: saorsalabs@gmail.com
Subject: License Upgrade Request - SMB to Enterprise
```

#### Step 2: Update License File
```rust
// The new license will have updated tier and limits
let new_license = CommercialLicense::load_from_file("new-license.json")?;
checker.update_status(new_license.to_status(current_users))?;
```

#### Step 3: Enable New Features
```rust
// Now you can use Enterprise features
if checker.is_feature_available(Feature::Analytics)? {
    enable_analytics_dashboard();
}
```

**Enterprise to OEM**:

Similar process, but includes:
- White-label configuration
- Redistribution rights setup
- Custom branding implementation

### 4. Handling Existing Deployments

#### For AGPL → Commercial Migration

**Development Environments**:
```bash
# Update all developer machines
for dev in $(cat developers.txt); do
    scp license.json $dev:~/.config/p2p-foundation/
done
```

**Production Systems**:
```yaml
# Kubernetes example
apiVersion: v1
kind: Secret
metadata:
  name: p2p-license
data:
  license.json: <base64-encoded-license>
```

**CI/CD Pipeline**:
```yaml
# GitHub Actions example
- name: Configure Commercial License
  run: |
    echo "${{ secrets.P2P_LICENSE }}" > license.json
    cargo build --features commercial
```

#### For Commercial → AGPL Migration

**Source Code Disclosure**:
```nginx
# Add to nginx config
location /source {
    return 301 https://github.com/yourorg/project;
}
```

**Update Documentation**:
```markdown
## License Change Notice

As of [DATE], this project has migrated from a commercial license 
to AGPL-3.0. Users now have the right to:
- Access complete source code
- Modify the software
- Redistribute under AGPL-3.0
```

## Migration Timelines

### Typical Migration Duration

| Migration Type | Planning | Implementation | Testing | Total |
|---------------|----------|----------------|---------|-------|
| AGPL → Commercial | 1 week | 2-3 weeks | 1 week | 4-5 weeks |
| Commercial → AGPL | 2 weeks | 3-4 weeks | 2 weeks | 7-8 weeks |
| Tier Upgrade | 1 day | 1-2 days | 1 day | 3-4 days |

### Critical Milestones

1. **License Agreement** (Day 1)
2. **Code Inventory** (Week 1)
3. **Compliance Check** (Week 2)
4. **Implementation** (Week 3-4)
5. **Testing** (Week 5)
6. **Deployment** (Week 6)

## Common Migration Challenges

### Technical Challenges

**Mixed Dependencies**:
```toml
# Problem: Some deps are GPL-incompatible
[dependencies]
proprietary-lib = "1.0"  # Incompatible with AGPL!

# Solution: Replace or isolate
[dependencies]
open-alternative = "2.0"  # AGPL-compatible
```

**Feature Flags**:
```rust
// Properly gate commercial features
#[cfg(feature = "commercial")]
fn premium_feature() {
    // Only available with commercial license
}

#[cfg(not(feature = "commercial"))]
fn premium_feature() {
    panic!("Premium feature requires commercial license");
}
```

### Legal Challenges

**Existing Contracts**:
- Review all customer agreements
- Update terms of service
- Notify users of license change
- Provide transition period

**IP Considerations**:
- Audit all code ownership
- Get contributor agreements
- Clear third-party licenses
- Document provenance

### Business Challenges

**Revenue Impact**:
- AGPL may affect commercial viability
- Plan for support/services revenue
- Consider dual licensing model
- Evaluate competitive implications

## Migration Tools

### License Auditing
```bash
# Scan for license compliance
cargo deny init
cargo deny check licenses

# Find proprietary code
git grep -i "proprietary\|confidential\|trade secret"
```

### Automated Migration
```rust
// Script to update license headers
use std::fs;
use std::path::Path;

fn update_headers(dir: &Path, old_license: &str, new_license: &str) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension() == Some("rs".as_ref()) {
            let content = fs::read_to_string(&path).unwrap();
            let updated = content.replace(old_license, new_license);
            fs::write(path, updated).unwrap();
        }
    }
}
```

### Validation Tools
```bash
# Verify AGPL compliance
./scripts/check-agpl-compliance.sh

# Test commercial features
cargo test --features commercial

# Benchmark performance impact
cargo bench --features commercial
```

## Support During Migration

### Documentation
- Migration checklist template
- License comparison matrix
- FAQ for common issues
- Best practices guide

### Technical Support
- **AGPL questions**: GitHub Discussions
- **Commercial support**: Based on tier
- **Migration assistance**: saorsalabs@gmail.com

### Legal Resources
- Template announcements
- License change notices
- Compliance checklists
- Third-party audit tools

## Post-Migration Tasks

### For Any Migration
- [ ] Update all documentation
- [ ] Notify all stakeholders
- [ ] Update CI/CD pipelines
- [ ] Train support team
- [ ] Monitor for issues

### AGPL-Specific
- [ ] Verify source availability
- [ ] Test source download links
- [ ] Update contribution guidelines
- [ ] Set up CLA if needed

### Commercial-Specific
- [ ] Distribute license files
- [ ] Configure license checking
- [ ] Enable premium features
- [ ] Set up usage tracking

## Rollback Procedures

If migration fails:

1. **Immediate Rollback** (< 24 hours)
   ```bash
   git revert --no-commit HEAD~n..HEAD
   git commit -m "Rollback license migration"
   ```

2. **Partial Rollback** (< 1 week)
   - Keep infrastructure changes
   - Revert only license files
   - Maintain dual setup temporarily

3. **Full Rollback** (> 1 week)
   - Formal change process
   - Customer notification
   - Gradual transition plan

## Success Metrics

Track these metrics post-migration:

- **Technical**: Build success rate, test coverage
- **Legal**: Compliance audit results
- **Business**: User retention, revenue impact
- **Operational**: Support ticket volume

## Conclusion

License migration requires careful planning and execution. Key points:

1. Understand requirements of both licenses
2. Plan thoroughly before starting
3. Test extensively in staging
4. Communicate clearly with users
5. Have rollback plan ready

For assistance, contact saorsalabs@gmail.com.

---

*Last updated: [Current Date]*