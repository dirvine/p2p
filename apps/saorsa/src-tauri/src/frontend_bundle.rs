// Copyright 2024 MaidSafe Limited
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

/// Frontend bundle with actual application files
/// This module contains the complete frontend application embedded as static strings

pub const INDEX_HTML: &str = include_str!("../../src/index.html");
pub const STYLES_CSS: &str = include_str!("../../src/styles.css");
// Note: When updating main.js, remember to update this module
pub const MAIN_JS: &str = include_str!("../../src/main.js");
pub const TEST_HTML: &str = include_str!("../../src/test.html");