use async_trait::async_trait;
use std::collections::HashMap;
use crate::error::ServerError;
use super::{AuthProvider, AuthData};

/// Simple API key authentication provider
#[derive(Clone)]
pub struct ApiKeyAuthService {
    /// Map of API key to user ID
    api_keys: HashMap<String, String>,
}

impl ApiKeyAuthService {
    /// Create a new API key auth service from environment variables
    /// Expects API_KEYS env var in format: "key1:user1,key2:user2"
    pub fn from_env() -> Result<Self, anyhow::Error> {
        let mut api_keys = HashMap::new();
        
        if let Ok(keys_str) = std::env::var("API_KEYS") {
            for pair in keys_str.split(',') {
                let parts: Vec<&str> = pair.split(':').collect();
                if parts.len() == 2 {
                    api_keys.insert(parts[0].to_string(), parts[1].to_string());
                }
            }
        }
        
        Ok(Self { api_keys })
    }
    
    /// Create a new API key auth service with predefined keys
    pub fn new(api_keys: HashMap<String, String>) -> Self {
        Self { api_keys }
    }
}

#[async_trait]
impl AuthProvider for ApiKeyAuthService {
    async fn authenticate(&self, credential: &str) -> Result<AuthData, ServerError> {
        // For API key auth, the credential is the API key itself
        let user_id = self.api_keys.get(credential)
            .ok_or_else(|| ServerError::InvalidSession("Invalid API key".to_string()))?;
        
        Ok(AuthData {
            user_id: user_id.clone(),
            metadata: serde_json::json!({
                "auth_type": "api_key"
            }),
        })
    }
    
    fn validate_credential_format(&self, credential: &str) -> Result<(), ServerError> {
        if credential.is_empty() {
            return Err(ServerError::InvalidSession("API key cannot be empty".to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_api_key_auth() {
        let mut keys = HashMap::new();
        keys.insert("test-key-123".to_string(), "user123".to_string());
        keys.insert("admin-key-456".to_string(), "admin456".to_string());
        
        let auth = ApiKeyAuthService::new(keys);
        
        // Test valid key
        let result = auth.authenticate("test-key-123").await;
        assert!(result.is_ok());
        let auth_data = result.unwrap();
        assert_eq!(auth_data.user_id, "user123");
        
        // Test invalid key
        let result = auth.authenticate("invalid-key").await;
        assert!(result.is_err());
    }
    
    #[test]
    fn test_validate_format() {
        let auth = ApiKeyAuthService::new(HashMap::new());
        
        assert!(auth.validate_credential_format("valid-key").is_ok());
        assert!(auth.validate_credential_format("").is_err());
    }
}