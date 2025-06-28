# P2P Foundation AGPL Compliance Guide

This guide helps you ensure compliance with the GNU Affero General Public License v3.0 (AGPL-3.0) when using P2P Foundation.

## Overview

If you're using P2P Foundation under the AGPL-3.0 license, you must comply with all terms of the license. This guide explains the key requirements and how to meet them.

## Key AGPL-3.0 Requirements

### 1. Source Code Disclosure

**Requirement**: You must provide the complete source code of your application to all users.

**How to comply**:
- Make your source code publicly available (e.g., GitHub, GitLab)
- Include all modifications you've made
- Provide build instructions
- Include all dependencies

### 2. Network Use Provision (Section 13)

**Requirement**: If users interact with your software over a network, you must provide them access to the source code.

**How to comply**:
- Add a prominent "Source Code" link in your application
- Link directly to your repository
- Ensure the link is always visible and accessible

**Example implementation**:
```rust
// In your web application
fn render_footer() -> Html {
    html! {
        <footer>
            <p>
                {"This application uses P2P Foundation under AGPL-3.0. "}
                <a href="https://github.com/yourorg/yourapp">{"View Source Code"}</a>
            </p>
        </footer>
    }
}
```

### 3. License Notices

**Requirement**: Preserve all copyright and license notices.

**How to comply**:
- Keep all original copyright notices in P2P Foundation files
- Add your own copyright for modifications
- Include the full AGPL-3.0 license text
- Display license information in your application

### 4. Modification Documentation

**Requirement**: Document all changes you make to P2P Foundation.

**How to comply**:
- Maintain a CHANGES.md or similar file
- Use clear commit messages
- Document the purpose of each modification
- Include dates of modifications

## Compliance Checklist

Use this checklist to ensure full AGPL-3.0 compliance:

### Repository Setup
- [ ] Source code is publicly accessible
- [ ] Repository includes LICENSE file with AGPL-3.0 text
- [ ] README mentions P2P Foundation and AGPL-3.0 license
- [ ] All P2P Foundation copyright notices preserved

### Code Requirements
- [ ] Source code link visible in application UI
- [ ] No attempt to circumvent AGPL requirements
- [ ] All modifications clearly marked
- [ ] Build instructions included

### Documentation
- [ ] Installation guide provided
- [ ] Dependencies listed
- [ ] Configuration documentation
- [ ] API documentation (if applicable)

### Distribution
- [ ] Source provided with binary distributions
- [ ] Download page mentions AGPL-3.0
- [ ] License included in packages
- [ ] No additional restrictions imposed

## Example Compliant Project Structure

```
your-project/
├── LICENSE                 # AGPL-3.0 license text
├── LICENSE-THIRD-PARTY    # Licenses of dependencies
├── README.md              # Mentions P2P Foundation and license
├── NOTICE                 # Copyright notices
├── CHANGES.md            # Modification log
├── src/
│   └── main.rs           # Your application code
├── Cargo.toml            # Specifies license = "AGPL-3.0"
└── docs/
    ├── installation.md   # How to build/install
    └── architecture.md   # Technical documentation
```

## Example Notices

### In Source Files
```rust
// Copyright 2024 Your Organization
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of [Your Project Name]
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
```

### In README
```markdown
## License

This project is licensed under the GNU Affero General Public License v3.0 or later - see the [LICENSE](LICENSE) file for details.

This project uses [P2P Foundation](https://github.com/dirvine/p2p) which is dual-licensed under AGPL-3.0 and Commercial licenses.
```

### In Web UI
```html
<div class="license-notice">
    <p>
        This service is free software licensed under 
        <a href="https://www.gnu.org/licenses/agpl-3.0.html">AGPL-3.0</a>.
        <a href="https://github.com/yourorg/yourproject">Source code available here</a>.
    </p>
</div>
```

## Common Compliance Mistakes to Avoid

### ❌ Don't:
- Hide or obfuscate source code
- Add proprietary features without sharing code
- Remove copyright notices
- Impose additional restrictions
- Forget the network use provision
- Mix proprietary and AGPL code

### ✅ Do:
- Share all source code openly
- Document all modifications
- Preserve all notices
- Make source easily accessible
- Include build instructions
- Keep AGPL and proprietary code separate

## Handling Mixed Licensing

If you need to combine AGPL and proprietary code:

1. **Best Practice**: Keep them in separate processes
   - AGPL service with public API
   - Proprietary service communicates via API
   - Clear boundary between components

2. **Alternative**: Purchase commercial license
   - Allows proprietary development
   - No source disclosure required
   - Contact: saorsalabs@gmail.com

## Frequently Asked Questions

### Q: Can I use P2P Foundation in a commercial product under AGPL?
**A**: Yes, but you must release your entire application under AGPL-3.0 and provide source code to all users.

### Q: What if I only use P2P Foundation internally?
**A**: Internal use without distribution may not trigger AGPL requirements, but if employees access it over a network, you may need to provide them source access.

### Q: Can I charge for my AGPL application?
**A**: Yes, AGPL allows charging for the software, but buyers must receive source code and AGPL rights.

### Q: How do I handle user modifications?
**A**: Users who receive your AGPL software can modify and redistribute it under AGPL. You cannot prevent this.

## Compliance Tools

### License Scanning
```bash
# Use license scanners to verify compliance
cargo license
cargo deny check licenses
```

### Header Checking
```bash
# Ensure all files have proper headers
find src -name "*.rs" -exec grep -L "SPDX-License-Identifier" {} \;
```

### Automated Compliance
Consider using:
- GitHub Actions for license checking
- Pre-commit hooks for headers
- Automated NOTICE file generation
- Dependency license validation

## Getting Help

### Legal Resources
- [GNU AGPL-3.0 FAQ](https://www.gnu.org/licenses/agpl-3.0.html)
- [Software Freedom Conservancy](https://sfconservancy.org/)
- [Free Software Foundation](https://www.fsf.org/)

### Community Support
- P2P Foundation GitHub Discussions
- Open source legal mailing lists
- AGPL compliance forums

### Professional Help
- Consider legal counsel for complex cases
- License compliance consultants
- Open source program offices

## Conclusion

AGPL-3.0 compliance is straightforward if you:
1. Make source code publicly available
2. Include proper notices and documentation
3. Provide source access for network users
4. Preserve the freedom of the software

If these requirements don't fit your use case, consider purchasing a commercial license at saorsalabs@gmail.com.

---

*Last updated: [Current Date]*  
*This guide is informational only and not legal advice.*