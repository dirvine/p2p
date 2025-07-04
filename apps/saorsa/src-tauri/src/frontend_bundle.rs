
/// Frontend bundle with actual application files
/// This module contains the complete frontend application embedded as static strings

pub const INDEX_HTML: &str = include_str!("../../src/index.html");
pub const STYLES_CSS: &str = include_str!("../../src/styles.css");
// Note: When updating main.js, remember to update this module
pub const MAIN_JS: &str = include_str!("../../src/main.js");
pub const TEST_HTML: &str = include_str!("../../src/test.html");