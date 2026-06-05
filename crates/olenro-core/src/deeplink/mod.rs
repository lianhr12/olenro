//! Deep link module
//!
//! Parsing and handling olenro:// URLs

pub mod parser;
pub mod utils;

pub use parser::{parse_deeplink_url, DeepLinkImportRequest};
