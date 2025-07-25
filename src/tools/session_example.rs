use rmcp::{ErrorData as McpError, model::{CallToolResult, Content}};
use serde::Deserialize;
use schemars::JsonSchema;
#[cfg(feature = "auth")]
use crate::auth::AuthService;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionInfoRequest {
    /// Session ID for authenticated user (UUID4 format)
    pub session_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UserProfileRequest {
    /// Session ID for authenticated user (UUID4 format)
    pub session_id: String,
    /// OAuth provider (e.g., 'google', 'github', 'microsoft')
    #[serde(default = "default_provider")]
    pub provider: String,
}

fn default_provider() -> String {
    "google".to_string()
}

/// Example tool that demonstrates session-based authentication patterns
#[derive(Clone)]
pub struct SessionExampleTool {
    #[cfg(feature = "auth")]
    auth_service: AuthService,
    #[cfg(not(feature = "auth"))]
    _phantom: std::marker::PhantomData<()>,
}

impl SessionExampleTool {
    #[cfg(feature = "auth")]
    pub fn new(auth_service: AuthService) -> Self {
        Self { auth_service }
    }

    #[cfg(not(feature = "auth"))]
    pub fn new(_auth_service: AuthService) -> Self {
        Self { _phantom: std::marker::PhantomData }
    }

    /// Get basic session information (demonstrates session validation)
    pub async fn get_session_info(&self, req: SessionInfoRequest) -> Result<CallToolResult, McpError> {
        tracing::info!("🔍 get_session_info called with session_id");
        
        // Validate session ID format first
        #[cfg(feature = "auth")]
        if let Err(e) = AuthService::validate_session_format(&req.session_id) {
            tracing::error!("❌ Invalid session ID format: {}", e);
            return Err(McpError::invalid_params(
                format!("Invalid session ID: {}", e), 
                None
            ));
        }

        #[cfg(feature = "auth")]
        {
            // Resolve session to user ID
            let user_id = self.auth_service
                .resolve_session(&req.session_id)
                .await
                .map_err(|e| {
                    tracing::error!("❌ Session resolution failed: {}", e);
                    McpError::invalid_params(
                        format!("Authentication failed: {}", e), 
                        None
                    )
                })?;

            let result_text = format!(
                "Session Information:\n\
                 • Session ID: {}\n\
                 • User ID: {}\n\
                 • Status: Valid\n\
                 • Authentication: Enabled",
                req.session_id, user_id
            );

            tracing::info!("✅ Session info retrieved for user: {}", user_id);
            Ok(CallToolResult::success(vec![Content::text(result_text)]))
        }

        #[cfg(not(feature = "auth"))]
        {
            let result_text = format!(
                "Session Information:\n\
                 • Session ID: {}\n\
                 • Status: Format Valid\n\
                 • Authentication: Disabled (auth feature not enabled)\n\
                 • Note: Enable 'auth' feature for full session validation",
                req.session_id
            );

            tracing::info!("✅ Session format validated (auth feature disabled)");
            Ok(CallToolResult::success(vec![Content::text(result_text)]))
        }
    }

    /// Get user profile information (demonstrates OAuth token usage)
    pub async fn get_user_profile(&self, req: UserProfileRequest) -> Result<CallToolResult, McpError> {
        tracing::info!("👤 get_user_profile called for provider: {}", req.provider);
        
        // Validate session ID format first
        #[cfg(feature = "auth")]
        if let Err(e) = AuthService::validate_session_format(&req.session_id) {
            tracing::error!("❌ Invalid session ID format: {}", e);
            return Err(McpError::invalid_params(
                format!("Invalid session ID: {}", e), 
                None
            ));
        }

        #[cfg(feature = "auth")]
        {
            // Authenticate and get OAuth token
            let token_data = self.auth_service
                .authenticate(&req.session_id, &req.provider)
                .await
                .map_err(|e| {
                    tracing::error!("❌ Authentication failed: {}", e);
                    McpError::invalid_params(
                        format!("Authentication failed: {}", e), 
                        None
                    )
                })?;

            let result_text = format!(
                "User Profile:\n\
                 • User ID: {}\n\
                 • Provider: {}\n\
                 • Email: {}\n\
                 • Display Name: {}\n\
                 • Provider User ID: {}\n\
                 • Scopes: {}\n\
                 • Linked At: {}\n\
                 • Token Status: {}\n\
                 • Authentication: Enabled",
                token_data.user_id,
                token_data.provider,
                token_data.email,
                token_data.display_name,
                token_data.provider_user_id,
                token_data.scopes.join(", "),
                token_data.linked_at,
                if token_data.is_expired() { "Expired" } else { "Valid" }
            );

            tracing::info!("✅ User profile retrieved for user: {}", token_data.user_id);
            Ok(CallToolResult::success(vec![Content::text(result_text)]))
        }

        #[cfg(not(feature = "auth"))]
        {
            let result_text = format!(
                "User Profile:\n\
                 • Session ID: {}\n\
                 • Provider: {}\n\
                 • Authentication: Disabled (auth feature not enabled)\n\
                 • Note: Enable 'auth' feature for full OAuth token access",
                req.session_id, req.provider
            );

            tracing::info!("✅ Profile request processed (auth feature disabled)");
            Ok(CallToolResult::success(vec![Content::text(result_text)]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_validation() {
        let auth_service = AuthService::new("redis://localhost:6379").unwrap();
        let tool = SessionExampleTool::new(auth_service);

        // Test valid UUID format
        let valid_req = SessionInfoRequest {
            session_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        };

        // This should not fail on format validation
        // (It may fail on Redis connection, but that's expected in tests)
        let result = tool.get_session_info(valid_req).await;
        
        // Test invalid UUID format
        let invalid_req = SessionInfoRequest {
            session_id: "invalid-uuid".to_string(),
        };

        let result = tool.get_session_info(invalid_req).await;
        assert!(result.is_err(), "Should fail with invalid UUID format");
    }

    #[test]
    fn test_default_provider() {
        assert_eq!(default_provider(), "google");
    }
}