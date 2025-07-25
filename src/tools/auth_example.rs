use rmcp::{ErrorData as McpError, model::{CallToolResult, Content}};
use serde::Deserialize;
use schemars::JsonSchema;
#[cfg(feature = "auth")]
use crate::auth::AuthProvider;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuthenticatedRequest {
    /// Authentication credential (API key, session ID, etc.)
    pub credential: String,
    /// The action to perform
    pub action: String,
}

/// Generic authenticated tool that works with any auth provider
#[derive(Clone)]
pub struct AuthExampleTool {
    #[cfg(feature = "auth")]
    auth_provider: Box<dyn AuthProvider>,
    #[cfg(not(feature = "auth"))]
    _phantom: std::marker::PhantomData<()>,
}

impl AuthExampleTool {
    #[cfg(feature = "auth")]
    pub fn new<A: AuthProvider + 'static>(auth_provider: A) -> Self {
        Self { 
            auth_provider: Box::new(auth_provider)
        }
    }

    #[cfg(not(feature = "auth"))]
    pub fn new<A>(_auth_provider: A) -> Self {
        Self { _phantom: std::marker::PhantomData }
    }

    /// Perform an authenticated action
    pub async fn authenticated_action(&self, req: AuthenticatedRequest) -> Result<CallToolResult, McpError> {
        tracing::info!("🔐 authenticated_action called");
        
        #[cfg(feature = "auth")]
        {
            // Validate credential format
            if let Err(e) = self.auth_provider.validate_credential_format(&req.credential) {
                tracing::error!("❌ Invalid credential format: {}", e);
                return Err(McpError::invalid_params(
                    format!("Invalid credential: {}", e), 
                    None
                ));
            }

            // Authenticate
            let auth_data = self.auth_provider
                .authenticate(&req.credential)
                .await
                .map_err(|e| {
                    tracing::error!("❌ Authentication failed: {}", e);
                    McpError::invalid_params(
                        format!("Authentication failed: {}", e), 
                        None
                    )
                })?;

            let result_text = format!(
                "Authenticated Action:\n\
                 • User ID: {}\n\
                 • Action: {}\n\
                 • Auth Type: {}\n\
                 • Status: Success",
                auth_data.user_id,
                req.action,
                auth_data.metadata.get("auth_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
            );

            tracing::info!("✅ Action performed for user: {}", auth_data.user_id);
            Ok(CallToolResult::success(vec![Content::text(result_text)]))
        }

        #[cfg(not(feature = "auth"))]
        {
            let result_text = format!(
                "Action Request:\n\
                 • Action: {}\n\
                 • Authentication: Disabled (auth feature not enabled)\n\
                 • Note: Enable 'auth' feature for authentication",
                req.action
            );

            tracing::info!("✅ Action processed (auth disabled)");
            Ok(CallToolResult::success(vec![Content::text(result_text)]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[cfg(feature = "auth-apikey")]
    #[tokio::test]
    async fn test_with_api_key_auth() {
        use crate::auth::ApiKeyAuthService;
        use std::collections::HashMap;
        
        let mut keys = HashMap::new();
        keys.insert("test-key".to_string(), "user123".to_string());
        
        let auth = ApiKeyAuthService::new(keys);
        let tool = AuthExampleTool::new(auth);
        
        let req = AuthenticatedRequest {
            credential: "test-key".to_string(),
            action: "test-action".to_string(),
        };
        
        let result = tool.authenticated_action(req).await;
        assert!(result.is_ok());
    }
    
    #[cfg(feature = "auth-redis")]
    #[tokio::test] 
    async fn test_with_redis_auth() {
        use crate::auth::RedisAuthService;
        
        // This would need a real Redis instance to work
        let auth = RedisAuthService::new("redis://localhost:6379").unwrap();
        let tool = AuthExampleTool::new(auth);
        
        let req = AuthenticatedRequest {
            credential: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            action: "test-action".to_string(),
        };
        
        // This would fail without a real session in Redis
        let _result = tool.authenticated_action(req).await;
    }
}