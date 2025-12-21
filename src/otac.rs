use crate::config::Config;
use crate::error::OtacError;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, Header, Validation};
use keyv::Keyv;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// JWT claims for OTAC tokens
#[derive(Debug, Serialize, Deserialize)]
pub struct OtacClaims {
    /// Payload - arbitrary JSON data stored in the token
    pub payload: Value,
    /// JWT ID - unique identifier for this token
    pub jti: String,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration (Unix timestamp)
    pub exp: i64,
}

/// Stored data for tracking token usage
#[derive(Debug, Serialize, Deserialize)]
struct StoredToken {
    payload: Value,
    created_at: i64,
}

pub struct OtacService {
    config: Config,
    store: Keyv,
}

impl OtacService {
    /// Create a new OTAC service with the given configuration
    pub async fn new(config: Config) -> Result<Self, OtacError> {
        let store = Self::create_store(&config.storage_url).await?;
        tracing::info!("Connected to storage backend");
        Ok(Self { config, store })
    }

    /// Create the appropriate keyv store based on the storage URL
    async fn create_store(storage_url: &str) -> Result<Keyv, OtacError> {
        // Parse the storage URL to determine the backend
        if storage_url.starts_with("redis://") || storage_url.starts_with("rediss://") {
            #[cfg(feature = "redis")]
            {
                use keyv::adapter::redis::RedisStoreBuilder;
                let store = RedisStoreBuilder::new()
                    .uri(storage_url)
                    .build()
                    .await
                    .map_err(|e| OtacError::Storage(format!("Failed to connect to Redis: {}", e)))?;
                
                Keyv::try_new(store)
                    .await
                    .map_err(|e| OtacError::Storage(format!("Failed to initialize keyv: {}", e)))
            }
            #[cfg(not(feature = "redis"))]
            {
                Err(OtacError::Config(format!(
                    "Redis storage requested but 'redis' feature not enabled. URL: {}",
                    storage_url
                )))
            }
        } else if storage_url == "memory://" || storage_url.is_empty() {
            Ok(Keyv::default())
        } else {
            Err(OtacError::Config(format!(
                "Unsupported storage URL: {}. Use 'redis://...' or 'memory://'",
                storage_url
            )))
        }
    }

    /// Create a new OTAC token
    /// 
    /// The `payload` is arbitrary JSON that will be stored in the token
    /// and returned when the token is consumed.
    pub async fn create(&self, payload: &Value) -> Result<String, OtacError> {
        let jti = Uuid::new_v4().to_string();
        let now = Utc::now();
        let exp = now + Duration::seconds(self.config.token_ttl_seconds as i64);

        let claims = OtacClaims {
            payload: payload.clone(),
            jti: jti.clone(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };

        // Get the current signing key
        let signing_key = self.config.current_signing_key();
        let encoding_key = signing_key.encoding_key()?;

        // Create JWT header with key ID
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some(signing_key.kid.clone());

        // Sign the token
        let token = encode(&header, &claims, &encoding_key)
            .map_err(|e| OtacError::TokenCreation(format!("Failed to sign token: {}", e)))?;

        // Store the token data for single-use enforcement
        let stored = StoredToken {
            payload: payload.clone(),
            created_at: now.timestamp(),
        };

        self.store
            .set_with_ttl(&jti, &stored, self.config.token_ttl_seconds)
            .await
            .map_err(|e| OtacError::Storage(format!("Failed to store token: {}", e)))?;

        tracing::info!(
            jti = %jti,
            kid = %signing_key.kid,
            ttl = self.config.token_ttl_seconds,
            "Token created"
        );

        Ok(token)
    }

    /// Consume (validate and delete) an OTAC token
    /// 
    /// Returns the payload that was stored in the token.
    /// The token can only be consumed once - subsequent calls will fail.
    pub async fn consume(&self, token: &str) -> Result<Value, OtacError> {
        // First, decode the header to get the key ID
        let header = jsonwebtoken::decode_header(token)
            .map_err(|e| OtacError::InvalidToken(format!("Invalid token header: {}", e)))?;

        let kid = header.kid
            .ok_or_else(|| OtacError::InvalidToken("Token missing key ID (kid)".into()))?;

        // Find the key that signed this token
        let jwk = self.config.find_key(&kid)
            .ok_or_else(|| OtacError::InvalidToken(format!("Unknown key ID: {}", kid)))?;

        let decoding_key = jwk.decoding_key()?;

        // Validate the token
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        let token_data = decode::<OtacClaims>(token, &decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => OtacError::TokenExpired,
                _ => OtacError::InvalidToken(format!("Token validation failed: {}", e)),
            })?;

        let jti = &token_data.claims.jti;

        // Attempt to get and delete atomically (check single-use)
        let stored_value = self.store
            .get(jti)
            .await
            .map_err(|e| OtacError::Storage(format!("Failed to check token: {}", e)))?;

        if stored_value.is_none() {
            tracing::warn!(jti = %jti, "Token not found or already consumed");
            return Err(OtacError::TokenNotFound);
        }

        // Delete the token to enforce single-use
        self.store
            .remove(jti)
            .await
            .map_err(|e| OtacError::Storage(format!("Failed to delete token: {}", e)))?;

        tracing::info!(
            jti = %jti,
            kid = %kid,
            "Token consumed"
        );

        Ok(token_data.claims.payload)
    }
}
