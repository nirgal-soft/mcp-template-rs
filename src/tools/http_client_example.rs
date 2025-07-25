use rmcp::{ErrorData as McpError, model::{CallToolResult, Content}};
use serde::Deserialize;
use schemars::JsonSchema;
#[cfg(feature = "auth")]
use crate::auth::AuthService;

#[cfg(feature = "http-client")]
use reqwest::Client;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuthenticatedApiRequest {
    /// Session ID for authenticated user (UUID4 format)
    pub session_id: String,
    /// API endpoint URL to call
    pub url: String,
    /// OAuth provider for token (e.g., 'google', 'github')
    #[serde(default = "default_provider")]
    pub provider: String,
    /// HTTP method (GET, POST, PUT, DELETE)
    #[serde(default = "default_method")]
    pub method: String,
    /// Optional JSON body for POST/PUT requests
    pub body: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PublicApiRequest {
    /// API endpoint URL to call
    pub url: String,
    /// HTTP method (GET, POST, PUT, DELETE)
    #[serde(default = "default_method")]
    pub method: String,
    /// Optional JSON body for POST/PUT requests
    pub body: Option<serde_json::Value>,
}

fn default_provider() -> String {
    "google".to_string()
}

fn default_method() -> String {
    "GET".to_string()
}

/// Example tool that demonstrates HTTP client patterns with authentication
#[derive(Clone)]
pub struct HttpClientExampleTool {
    #[cfg(feature = "http-client")]
    client: Client,
    #[cfg(feature = "auth")]
    auth_service: AuthService,
    #[cfg(not(feature = "auth"))]
    _auth_phantom: std::marker::PhantomData<AuthService>,
    #[cfg(not(feature = "http-client"))]
    _client_phantom: std::marker::PhantomData<()>,
}

impl HttpClientExampleTool {
    #[cfg(all(feature = "http-client", feature = "auth"))]
    pub fn new(auth_service: AuthService) -> Self {
        Self {
            client: Client::new(),
            auth_service,
        }
    }

    #[cfg(all(feature = "http-client", not(feature = "auth")))]
    pub fn new(_auth_service: AuthService) -> Self {
        Self {
            client: Client::new(),
            _auth_phantom: std::marker::PhantomData,
        }
    }

    #[cfg(all(not(feature = "http-client"), feature = "auth"))]
    pub fn new(auth_service: AuthService) -> Self {
        Self {
            auth_service,
            _client_phantom: std::marker::PhantomData,
        }
    }

    #[cfg(all(not(feature = "http-client"), not(feature = "auth")))]
    pub fn new(_auth_service: AuthService) -> Self {
        Self {
            _auth_phantom: std::marker::PhantomData,
            _client_phantom: std::marker::PhantomData,
        }
    }

    /// Make an authenticated API call using OAuth token
    pub async fn authenticated_api_call(&self, req: AuthenticatedApiRequest) -> Result<CallToolResult, McpError> {
        tracing::info!("🌐 authenticated_api_call to: {}", req.url);
        
        // Validate session ID format first
        #[cfg(feature = "auth")]
        if let Err(e) = AuthService::validate_session_format(&req.session_id) {
            tracing::error!("❌ Invalid session ID format: {}", e);
            return Err(McpError::invalid_params(
                format!("Invalid session ID: {}", e), 
                None
            ));
        }

        #[cfg(all(feature = "auth", feature = "http-client"))]
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

            // Build HTTP request
            let mut request_builder = match req.method.to_uppercase().as_str() {
                "GET" => self.client.get(&req.url),
                "POST" => self.client.post(&req.url),
                "PUT" => self.client.put(&req.url),
                "DELETE" => self.client.delete(&req.url),
                _ => return Err(McpError::invalid_params("Unsupported HTTP method", None)),
            };

            // Add OAuth bearer token
            request_builder = request_builder.bearer_auth(&token_data.access_token);

            // Add JSON body if provided
            if let Some(body) = req.body {
                request_builder = request_builder.json(&body);
            }

            // Make the request
            let response = request_builder
                .send()
                .await
                .map_err(|e| McpError::internal_error(format!("HTTP request failed: {}", e), None))?;

            let status = response.status();
            let response_text = response
                .text()
                .await
                .map_err(|e| McpError::internal_error(format!("Failed to read response: {}", e), None))?;

            let result_text = format!(
                "Authenticated API Call Results:\n\
                 • URL: {}\n\
                 • Method: {}\n\
                 • Status: {}\n\
                 • Provider: {}\n\
                 • User: {}\n\
                 • Response Length: {} bytes\n\
                 • Response Preview: {}\n\
                 • Authentication: Enabled",
                req.url,
                req.method,
                status,
                req.provider,
                token_data.email,
                response_text.len(),
                if response_text.len() > 200 {
                    format!("{}...", &response_text[..200])
                } else {
                    response_text.clone()
                }
            );

            tracing::info!("✅ Authenticated API call completed with status: {}", status);
            Ok(CallToolResult::success(vec![Content::text(result_text)]))
        }

        #[cfg(not(all(feature = "auth", feature = "http-client")))]
        {
            let missing_features = match (cfg!(feature = "auth"), cfg!(feature = "http-client")) {
                (false, false) => "auth and http-client features",
                (false, true) => "auth feature",
                (true, false) => "http-client feature",
                (true, true) => unreachable!(),
            };

            let result_text = format!(
                "Authenticated API Call:\n\
                 • URL: {}\n\
                 • Method: {}\n\
                 • Provider: {}\n\
                 • Status: Feature Not Available\n\
                 • Missing: {}\n\
                 • Note: Enable required features for full functionality",
                req.url, req.method, req.provider, missing_features
            );

            tracing::info!("⚠️ Authenticated API call requested but features not enabled");
            Ok(CallToolResult::success(vec![Content::text(result_text)]))
        }
    }

    /// Make a public API call (no authentication required)
    pub async fn public_api_call(&self, req: PublicApiRequest) -> Result<CallToolResult, McpError> {
        tracing::info!("🌍 public_api_call to: {}", req.url);

        #[cfg(feature = "http-client")]
        {
            // Build HTTP request
            let mut request_builder = match req.method.to_uppercase().as_str() {
                "GET" => self.client.get(&req.url),
                "POST" => self.client.post(&req.url),
                "PUT" => self.client.put(&req.url),
                "DELETE" => self.client.delete(&req.url),
                _ => return Err(McpError::invalid_params("Unsupported HTTP method", None)),
            };

            // Add JSON body if provided
            if let Some(body) = req.body {
                request_builder = request_builder.json(&body);
            }

            // Make the request
            let response = request_builder
                .send()
                .await
                .map_err(|e| McpError::internal_error(format!("HTTP request failed: {}", e), None))?;

            let status = response.status();
            let response_text = response
                .text()
                .await
                .map_err(|e| McpError::internal_error(format!("Failed to read response: {}", e), None))?;

            let result_text = format!(
                "Public API Call Results:\n\
                 • URL: {}\n\
                 • Method: {}\n\
                 • Status: {}\n\
                 • Response Length: {} bytes\n\
                 • Response Preview: {}\n\
                 • Authentication: Not Required",
                req.url,
                req.method,
                status,
                response_text.len(),
                if response_text.len() > 200 {
                    format!("{}...", &response_text[..200])
                } else {
                    response_text.clone()
                }
            );

            tracing::info!("✅ Public API call completed with status: {}", status);
            Ok(CallToolResult::success(vec![Content::text(result_text)]))
        }

        #[cfg(not(feature = "http-client"))]
        {
            let result_text = format!(
                "Public API Call:\n\
                 • URL: {}\n\
                 • Method: {}\n\
                 • Status: Feature Not Available\n\
                 • Missing: http-client feature\n\
                 • Note: Enable 'http-client' feature for HTTP functionality",
                req.url, req.method
            );

            tracing::info!("⚠️ Public API call requested but http-client feature not enabled");
            Ok(CallToolResult::success(vec![Content::text(result_text)]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        assert_eq!(default_provider(), "google");
        assert_eq!(default_method(), "GET");
    }

    #[tokio::test]
    async fn test_invalid_session_format() {
        let auth_service = AuthService::new("redis://localhost:6379").unwrap();
        let tool = HttpClientExampleTool::new(auth_service);

        let invalid_req = AuthenticatedApiRequest {
            session_id: "invalid-uuid".to_string(),
            url: "https://api.example.com/test".to_string(),
            provider: "google".to_string(),
            method: "GET".to_string(),
            body: None,
        };

        let result = tool.authenticated_api_call(invalid_req).await;
        assert!(result.is_err(), "Should fail with invalid UUID format");
    }

    #[cfg(feature = "http-client")]
    #[tokio::test]
    async fn test_public_api_call_invalid_method() {
        let auth_service = AuthService::new("redis://localhost:6379").unwrap();
        let tool = HttpClientExampleTool::new(auth_service);

        let invalid_req = PublicApiRequest {
            url: "https://httpbin.org/get".to_string(),
            method: "INVALID".to_string(),
            body: None,
        };

        let result = tool.public_api_call(invalid_req).await;
        assert!(result.is_err(), "Should fail with invalid HTTP method");
    }
}