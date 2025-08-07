# Code Style and Conventions

## Rust Conventions

### Naming Conventions
- **Types**: `CamelCase`
- **Functions/methods**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Modules**: `snake_case`

### Error Handling
- Use `anyhow::Result` for applications
- Use custom error types for libraries
- Prefer `?` operator for error propagation
- Use `thiserror` for error types in libraries
- **NEVER use `unwrap()` or `panic!()` in production code**

### Documentation
- Every public item must have doc comments
- Include examples in doc comments where helpful
- Use `cargo test --doc` to test documentation examples

### Performance Guidelines
- Profile before optimizing
- Use `&str` instead of `String` when possible
- Prefer iterators over explicit loops
- Use `Cow<str>` for potentially-owned strings

### Safety Guidelines
- Minimize `unsafe` usage, document invariants when necessary
- Use `#[must_use]` for important return values
- Prefer safe abstractions over raw pointers

### Async Programming
- Always use Tokio runtime
- Use `#[tokio::main]` for main functions
- Use `#[tokio::test]` for async tests

## Strict Linting Rules

### Critical Production Rules (DENY)
```toml
unwrap_used = "deny"
expect_used = "deny" 
panic = "deny"
panic_in_result_fn = "deny"
unimplemented = "deny"
unreachable = "deny"
```

### Security Lints
```toml
arithmetic_overflow = "deny"
unwrap_in_result = "deny"
enum_glob_use = "deny"
mem_forget = "deny"
```

### Performance Lints
```toml
inefficient_to_string = "deny"
needless_collect = "deny"
```

## Testing Patterns

### Test Structure
- Use Arrange-Act-Assert pattern
- Use descriptive test names
- Group related tests in modules
- Use `#[should_panic]` for error testing

### Example Test Pattern
```rust
#[tokio::test]
async fn test_feature_name() {
    // Arrange
    let input = setup_test_data();
    
    // Act
    let result = function_under_test(input).await;
    
    // Assert
    assert_eq!(result.unwrap(), expected_value);
}
```

## Code Organization
- One major concept per file
- Use modules to group related functionality
- Export public APIs through `lib.rs`
- Keep implementation details private

## Documentation Standards
- All public APIs must be documented
- Include examples for complex APIs
- Document safety requirements for unsafe code
- Use `//!` for module-level documentation