// Template for fixing common unwrap() patterns

// Pattern 1: Simple unwrap() -> Result propagation
// Before: let value = something.unwrap();
// After:  let value = something?;

// Pattern 2: unwrap() with context
// Before: let value = something.unwrap();
// After:  let value = something.context("Failed to get value")?;

// Pattern 3: unwrap() in match/if let
// Before: if condition { value.unwrap() }
// After:  if condition { value? }

// Pattern 4: Default fallback
// Before: let value = option.unwrap();
// After:  let value = option.unwrap_or_default();
// Or:     let value = option.ok_or(Error::MissingValue)?;

// Pattern 5: Expect with proper error
// Before: let value = result.unwrap();
// After:  let value = result.map_err(|e| Error::Custom(format!("Failed: {}", e)))?;

// Pattern 6: In initialization (lazy_static, once_cell)
// Before: static VAR: Lazy<Type> = Lazy::new(|| create().unwrap());
// After:  static VAR: Lazy<Result<Type>> = Lazy::new(|| create());
//         // Then use VAR.as_ref()? when accessing

// Pattern 7: Test helpers in production code
// Move to test module or use Result type
