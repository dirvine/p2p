# Task 10: Fix Remaining unwrap() Calls

## Overview
Systematic replacement of all remaining unwrap() calls and implement preventive measures.

## Context
- **Phase**: Quality Assurance (Week 4-5)
- **Priority**: MEDIUM
- **Impact**: Potential panic points
- **Scope**: All remaining unwrap() after high-risk fixes

## Requirements
1. Systematic replacement of all unwrap()
2. Add clippy rules to prevent new ones
3. Update contributor guidelines
4. Add pre-commit hooks

## Approach
```bash
# Find all remaining unwrap()
grep -r "unwrap()" crates/p2p-core/src/ --include="*.rs"

# Categories:
# 1. Test code - can keep unwrap()
# 2. Example code - replace with expect()
# 3. Production code - must eliminate
```

## Clippy Configuration
```toml
# .clippy.toml
disallowed-methods = [
    { path = "Result::unwrap", reason = "Use ? or proper error handling" },
    { path = "Option::unwrap", reason = "Use if let or match" }
]
```

## Pre-commit Hook
```bash
#!/bin/bash
# Check for unwrap() in non-test code
if grep -r "unwrap()" --include="*.rs" src/ | grep -v test; then
    echo "Error: unwrap() found in production code"
    exit 1
fi
```

## Acceptance Criteria
- [ ] Zero unwrap() in production code
- [ ] Clippy rule configured
- [ ] Pre-commit hook installed
- [ ] Contributor guide updated
- [ ] CI/CD checks for unwrap()
- [ ] Team trained on policy

## Dependencies
- Task 1: Error Handling Framework
- Task 2: High-risk unwrap fixes

## Testing
- Verify no unwrap() in src/
- Test pre-commit hook
- Verify clippy catches new unwrap()
- Documentation review