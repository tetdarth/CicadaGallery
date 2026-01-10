//! License module
//! 
//! This module provides license verification functionality.
//! In the free/open-source version, premium features are disabled.

use serde::{Deserialize, Serialize};
use chrono::DateTime;
#[cfg(not(feature = "premium"))]
use chrono::Utc;

/// License information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub license_type: String,
    pub issued_to: String,
    pub issued_at: i64,
    pub expires_at: Option<i64>,
}

/// License verification result
#[derive(Debug, Clone)]
pub struct License {
    pub info: LicenseInfo,
    pub is_valid: bool,
    pub is_expired: bool,
}

impl License {
    /// Check if premium features should be enabled
    /// In the open-source version, this always returns false
    pub fn is_premium_active(&self) -> bool {
        #[cfg(feature = "premium")]
        {
            self.is_valid && !self.is_expired && self.info.license_type == "premium"
        }
        #[cfg(not(feature = "premium"))]
        {
            false
        }
    }
}

/// Verify a license key
/// 
/// In the open-source version (without "premium" feature), this always fails.
/// Premium license verification is only available in the premium build.
#[cfg(feature = "premium")]
pub fn verify_license(license_key: &str) -> Result<License, String> {
    // Premium verification is in license_premium.rs
    crate::license_premium::verify_license(license_key)
}

#[cfg(not(feature = "premium"))]
pub fn verify_license(_license_key: &str) -> Result<License, String> {
    Err("License verification is not available in the free version. Please purchase a license to unlock premium features.".to_string())
}

/// Format timestamp for display
pub fn format_timestamp(timestamp: i64) -> String {
    if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
        dt.format("%Y-%m-%d").to_string()
    } else {
        "Invalid date".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_license_verification_disabled_in_free() {
        #[cfg(not(feature = "premium"))]
        {
            let result = verify_license("any_key");
            assert!(result.is_err());
        }
    }
}

