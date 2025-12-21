use crate::error::OtacError;
use jsonwebtoken::{DecodingKey, EncodingKey};
use serde::Deserialize;

/// JWK (JSON Web Key) structure following RFC 7517
/// Supports symmetric keys (kty: "oct") for HMAC signing
#[derive(Debug, Deserialize, Clone)]
pub struct Jwk {
    /// Key type - must be "oct" for symmetric keys
    pub kty: String,
    /// Key ID - used to identify which key signed a token (for rotation)
    pub kid: String,
    /// The actual key material, base64url-encoded
    pub k: String,
    /// Algorithm - should be "HS256", "HS384", or "HS512"
    #[serde(default = "default_alg")]
    pub alg: String,
}

fn default_alg() -> String {
    "HS256".to_string()
}

/// JWK Set structure following RFC 7517
/// Contains multiple keys to support key rotation
#[derive(Debug, Deserialize, Clone)]
pub struct JwkSet {
    pub keys: Vec<Jwk>,
}

impl Jwk {
    /// Decode the base64url-encoded key material
    pub fn decode_key(&self) -> Result<Vec<u8>, OtacError> {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        
        URL_SAFE_NO_PAD
            .decode(&self.k)
            .map_err(|e| OtacError::Config(format!("Invalid base64url in JWK: {}", e)))
    }

    /// Get the jsonwebtoken EncodingKey
    pub fn encoding_key(&self) -> Result<EncodingKey, OtacError> {
        let bytes = self.decode_key()?;
        Ok(EncodingKey::from_secret(&bytes))
    }

    /// Get the jsonwebtoken DecodingKey
    pub fn decoding_key(&self) -> Result<DecodingKey, OtacError> {
        let bytes = self.decode_key()?;
        Ok(DecodingKey::from_secret(&bytes))
    }
}

pub struct Config {
    /// JWK Set containing signing keys
    pub jwks: JwkSet,
    /// Storage connection URL (passed to keyv)
    pub storage_url: String,
    /// Token TTL in seconds (default: 3600)
    pub token_ttl_seconds: u64,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, OtacError> {
        // OTACIOUS_JWKS: JSON string containing the JWK Set
        let jwks_json = std::env::var("OTACIOUS_JWKS")
            .map_err(|_| OtacError::Config("OTACIOUS_JWKS environment variable is required".into()))?;
        
        let jwks: JwkSet = serde_json::from_str(&jwks_json)
            .map_err(|e| OtacError::Config(format!("Invalid JWKS JSON: {}", e)))?;

        if jwks.keys.is_empty() {
            return Err(OtacError::Config("JWKS must contain at least one key".into()));
        }

        // Validate all keys are symmetric
        for key in &jwks.keys {
            if key.kty != "oct" {
                return Err(OtacError::Config(format!(
                    "Key '{}' has unsupported type '{}'. Only 'oct' (symmetric) keys are supported.",
                    key.kid, key.kty
                )));
            }
        }

        // OTACIOUS_STORAGE: keyv connection string
        let storage_url = std::env::var("OTACIOUS_STORAGE")
            .unwrap_or_else(|_| "memory://".to_string());

        // OTACIOUS_TTL: Token TTL in seconds
        let token_ttl_seconds: u64 = std::env::var("OTACIOUS_TTL")
            .unwrap_or_else(|_| "3600".to_string())
            .parse()
            .map_err(|_| OtacError::Config("OTACIOUS_TTL must be a valid number".into()))?;

        Ok(Config {
            jwks,
            storage_url,
            token_ttl_seconds,
        })
    }

    /// Get the current signing key (first key in the set)
    pub fn current_signing_key(&self) -> &Jwk {
        &self.jwks.keys[0]
    }

    /// Find a key by its key ID
    pub fn find_key(&self, kid: &str) -> Option<&Jwk> {
        self.jwks.keys.iter().find(|k| k.kid == kid)
    }

    /// Display-safe version of storage URL (hides credentials)
    pub fn storage_url_display(&self) -> String {
        // Simple regex-free password masking for redis:// URLs
        // Pattern: redis://:password@host or redis://user:password@host
        if self.storage_url.contains('@') {
            if let Some(at_pos) = self.storage_url.find('@') {
                if let Some(scheme_end) = self.storage_url.find("://") {
                    let prefix = &self.storage_url[..scheme_end + 3];
                    let suffix = &self.storage_url[at_pos..];
                    return format!("{}***{}", prefix, suffix);
                }
            }
        }
        self.storage_url.clone()
    }
}

