use serde::{Deserialize, Serialize};
use uuid::Uuid;
use anyhow::Result;
use crate::error::ServerError;
use redis::AsyncCommands;
use zeroize::{Zeroize, ZeroizeOnDrop};
use async_trait::async_trait;
use super::{AuthProvider, AuthData};

/// Session data structure stored in Redis
#[derive(Serialize, Deserialize, Debug)]
pub struct SessionData {
    pub session_id: String,
    pub user_id: String,
    pub created_at: String,
    pub expires_at: String,
}

/// OAuth token data structure stored in Redis
#[derive(Serialize, Deserialize, Clone, Zeroize, ZeroizeOnDrop)]
pub struct OAuthTokenData {
    pub user_id: String,
    pub provider: String,
    pub provider_user_id: String,
    pub email: String,
    pub display_name: String,
    #[zeroize(skip)]
    pub access_token: String,
    #[zeroize(skip)]
    pub refresh_token: Option<String>,
    pub expires_at: String, // ISO 8601 timestamp string
    pub scopes: Vec<String>, // Array of scope strings
    pub linked_at: String,
}

impl OAuthTokenData {
    /// Check if the access token is expired
    pub fn is_expired(&self) -> bool {
        // Parse the ISO 8601 timestamp string
        match chrono::DateTime::parse_from_rfc3339(&self.expires_at) {
            Ok(expires_at) => {
                let now = chrono::Utc::now();
                now >= expires_at.with_timezone(&chrono::Utc)
            }
            Err(_) => {
                // If we can't parse the timestamp, consider it expired for safety
                tracing::warn!("Failed to parse expires_at timestamp: {}", self.expires_at);
                true
            }
        }
    }

    /// Check if the token has a specific scope
    pub fn has_scope(&self, required_scope: &str) -> bool {
        self.scopes.iter().any(|scope| scope.contains(required_scope))
    }
}

/// Redis-based authentication service for handling session resolution and OAuth tokens
#[derive(Clone)]
pub struct RedisAuthService {
    redis_client: redis::Client,
}

impl RedisAuthService {
    pub fn new(redis_url: &str) -> Result<Self> {
        let redis_client = redis::Client::open(redis_url)?;
        Ok(Self { redis_client })
    }

    /// Resolve session ID to user ID via Redis lookup
    pub async fn resolve_session(&self, session_id: &str) -> Result<String, ServerError> {
        // Validate session ID format (should be UUID4)
        if Uuid::parse_str(session_id).is_err() {
            return Err(ServerError::InvalidSession("Invalid session ID format".to_string()));
        }

        let mut conn = self.redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ServerError::Redis(format!("Failed to connect to Redis: {}", e)))?;

        let session_key = format!("mcp_session:{}", session_id);
        
        let session_json: Option<String> = conn
            .get(&session_key)
            .await
            .map_err(|e| ServerError::Redis(format!("Failed to get session: {}", e)))?;

        let session_json = session_json
            .ok_or_else(|| ServerError::InvalidSession("Session not found or expired".to_string()))?;
        
        // Parse the session JSON to extract user_id
        let session_data: SessionData = serde_json::from_str(&session_json)
            .map_err(|e| ServerError::InvalidSession(format!("Invalid session data: {}", e)))?;
        
        Ok(session_data.user_id)
    }

    /// Retrieve OAuth tokens for a user
    pub async fn get_oauth_token(&self, user_id: &str, provider: &str) -> Result<OAuthTokenData, ServerError> {
        let mut conn = self.redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ServerError::Redis(format!("Failed to connect to Redis: {}", e)))?;

        let oauth_key = format!("linked_account:{}:{}", user_id, provider);
        
        let token_json: Option<String> = conn
            .get(&oauth_key)
            .await
            .map_err(|e| ServerError::Redis(format!("Failed to get OAuth token: {}", e)))?;

        let token_json = token_json
            .ok_or_else(|| ServerError::InvalidSession("OAuth token not found".to_string()))?;

        let token_data: OAuthTokenData = serde_json::from_str(&token_json)
            .map_err(|e| ServerError::InvalidSession(format!("Invalid OAuth token data: {}", e)))?;

        if token_data.is_expired() {
            return Err(ServerError::InvalidSession("OAuth token expired".to_string()));
        }

        Ok(token_data)
    }

    /// Complete authentication flow: session_id -> user_id -> oauth_token
    pub async fn authenticate(&self, session_id: &str, provider: &str) -> Result<OAuthTokenData, ServerError> {
        let user_id = self.resolve_session(session_id).await?;
        self.get_oauth_token(&user_id, provider).await
    }

    /// Validate session ID format without Redis lookup (for early validation)
    pub fn validate_session_format(session_id: &str) -> Result<(), ServerError> {
        Uuid::parse_str(session_id)
            .map_err(|_| ServerError::InvalidSession("Invalid session ID format".to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl AuthProvider for RedisAuthService {
    async fn authenticate(&self, credential: &str) -> Result<AuthData, ServerError> {
        // For Redis auth, the credential is the session ID
        let user_id = self.resolve_session(credential).await?;
        
        Ok(AuthData {
            user_id,
            metadata: serde_json::json!({
                "session_id": credential,
                "auth_type": "redis_session"
            }),
        })
    }
    
    fn validate_credential_format(&self, credential: &str) -> Result<(), ServerError> {
        Self::validate_session_format(credential)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_validation() {
        // Valid UUID4
        assert!(RedisAuthService::validate_session_format("550e8400-e29b-41d4-a716-446655440000").is_ok());
        
        // Invalid formats
        assert!(RedisAuthService::validate_session_format("invalid-uuid").is_err());
        assert!(RedisAuthService::validate_session_format("").is_err());
        assert!(RedisAuthService::validate_session_format("123").is_err());
    }

    #[cfg(feature = "auth")]
    #[test]
    fn test_token_expiration() {
        let expired_token = OAuthTokenData {
            user_id: "test-user".to_string(),
            provider: "google".to_string(),
            provider_user_id: "123".to_string(),
            email: "test@example.com".to_string(),
            display_name: "Test User".to_string(),
            access_token: "token".to_string(),
            refresh_token: None,
            expires_at: "2020-01-01T00:00:00Z".to_string(), // Past timestamp
            scopes: vec!["test".to_string()],
            linked_at: "2020-01-01T00:00:00Z".to_string(),
        };
        assert!(expired_token.is_expired());

        let future_time = chrono::Utc::now() + chrono::Duration::hours(1);
        let valid_token = OAuthTokenData {
            user_id: "test-user".to_string(),
            provider: "google".to_string(),
            provider_user_id: "123".to_string(),
            email: "test@example.com".to_string(),
            display_name: "Test User".to_string(),
            access_token: "token".to_string(),
            refresh_token: None,
            expires_at: future_time.to_rfc3339(), // 1 hour from now
            scopes: vec!["https://www.googleapis.com/auth/userinfo.email".to_string()],
            linked_at: "2020-01-01T00:00:00Z".to_string(),
        };
        assert!(!valid_token.is_expired());
        assert!(valid_token.has_scope("userinfo.email"));
    }
}