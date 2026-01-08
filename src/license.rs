use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use base64::Engine;

/// License information embedded in the license key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub license_type: String,  // "premium", "trial", etc.
    pub issued_to: String,      // Name or email
    pub issued_at: i64,         // Unix timestamp
    pub expires_at: Option<i64>, // Unix timestamp, None for lifetime
}

/// License verification result
#[derive(Debug, Clone)]
pub struct License {
    pub info: LicenseInfo,
    pub is_valid: bool,
    pub is_expired: bool,
}

impl License {
    pub fn is_premium_active(&self) -> bool {
        self.is_valid && !self.is_expired && self.info.license_type == "premium"
    }
}

// Public key for license verification (embedded in the application)
// この公開鍵は開発者が生成した秘密鍵のペアとなります
// 実際の運用では、秘密鍵は安全に保管し、公開鍵のみをアプリに埋め込みます
const PUBLIC_KEY_HEX: &str = "b9986fd640d106569bcadcafd304c68bcfc509e235e1b9783f609b0e2f63ce44";

/// Verify a license key
/// 
/// License key format: base64(json_data) + "." + base64(signature)
pub fn verify_license(license_key: &str) -> Result<License, String> {
    // Split license key into data and signature
    let parts: Vec<&str> = license_key.split('.').collect();
    if parts.len() != 2 {
        return Err("Invalid license key format".to_string());
    }
    
    let data_b64 = parts[0];
    let signature_b64 = parts[1];
    
    // Decode base64
    let data = base64::engine::general_purpose::STANDARD
        .decode(data_b64)
        .map_err(|e| format!("Failed to decode license data: {}", e))?;
    
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_b64)
        .map_err(|e| format!("Failed to decode signature: {}", e))?;
    
    // Parse public key
    let public_key_bytes = hex::decode(PUBLIC_KEY_HEX)
        .map_err(|e| format!("Failed to decode public key: {}", e))?;
    
    let public_key = VerifyingKey::from_bytes(
        public_key_bytes.as_slice().try_into()
            .map_err(|_| "Invalid public key length".to_string())?
    ).map_err(|e| format!("Invalid public key: {}", e))?;
    
    // Parse signature
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|e| format!("Invalid signature: {}", e))?;
    
    // Verify signature
    let is_valid = public_key.verify(&data, &signature).is_ok();
    
    if !is_valid {
        return Err("Invalid license signature".to_string());
    }
    
    // Parse license info
    let info: LicenseInfo = serde_json::from_slice(&data)
        .map_err(|e| format!("Failed to parse license info: {}", e))?;
    
    // Check expiration
    let now = Utc::now().timestamp();
    let is_expired = if let Some(expires_at) = info.expires_at {
        now > expires_at
    } else {
        false
    };
    
    Ok(License {
        info,
        is_valid,
        is_expired,
    })
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
    fn test_invalid_license() {
        let result = verify_license("invalid_license_key");
        assert!(result.is_err());
    }
}
