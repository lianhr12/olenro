//! Deep link utilities

use crate::error::AppResult;

/// Validate a deep link URL
pub fn is_valid_deeplink(url: &str) -> bool {
    url.starts_with("olenro://") || url.starts_with("ccswitch://")
}
