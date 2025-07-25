use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::error::ServerError;

#[cfg(feature = "auth-redis")]
pub mod redis;

#[cfg(feature = "auth-redis")]
pub use redis::{RedisAuthService, SessionData, OAuthTokenData};

#[cfg(feature = "auth-apikey")]
pub mod apikey;

#[cfg(feature = "auth-apikey")]
pub use apikey::ApiKeyAuthService;

/// Generic authentication data that different auth providers can return
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthData {
    pub user_id: String,
    pub metadata: serde_json::Value,
}

/// Trait for authentication providers
#[async_trait]
pub trait AuthProvider: Clone + Send + Sync + 'static {
    /// Authenticate a credential and return user data
    async fn authenticate(&self, credential: &str) -> Result<AuthData, ServerError>;
    
    /// Validate credential format (optional early validation)
    fn validate_credential_format(&self, credential: &str) -> Result<(), ServerError> {
        Ok(())
    }
}

/// A no-op auth provider for when authentication is disabled
#[derive(Clone)]
pub struct NoOpAuthProvider;

#[async_trait]
impl AuthProvider for NoOpAuthProvider {
    async fn authenticate(&self, _credential: &str) -> Result<AuthData, ServerError> {
        Err(ServerError::InvalidSession("Authentication is disabled".to_string()))
    }
}