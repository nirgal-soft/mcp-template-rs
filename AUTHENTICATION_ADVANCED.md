# Authentication Implementation Guide

This document describes how to implement session-based authentication for your MCP server using the provided authentication module.

## Overview

The authentication system provides:

1. **Session Resolution**: Converting session IDs to user IDs via Redis lookup
2. **OAuth Token Management**: Retrieving and using user OAuth tokens for API calls
3. **Security Validation**: Validating session IDs and rejecting invalid/expired sessions
4. **Token Protection**: OAuth tokens never leave Redis except for direct API calls

## Architecture

```
MCP Client -> Session ID -> MCP Server -> Redis -> OAuth Token -> External API
```

### Authentication Flow

1. **Session Validation**: Validate session ID format (UUID4)
2. **Session Resolution**: `redis.get(format!("mcp_session:{}", session_id))` -> user_id
3. **Token Retrieval**: `redis.get(format!("linked_account:{}:{}", user_id, provider))` -> OAuth token data
4. **Token Validation**: Check if OAuth token is expired
5. **API Call**: Use access token for external API requests

## Setup Instructions

### 1. Enable Authentication Feature

Add the `auth` feature to your `Cargo.toml`:

```toml
[features]
default = ["auth"]
auth = ["redis", "uuid", "zeroize", "chrono"]
```

### 2. Configure Redis

Add Redis configuration to your `config.toml`:

```toml
[redis]
url = "redis://localhost:6379"
```

Or set via environment variable:
```bash
export MCP_REDIS_URL="redis://localhost:6379"
```

### 3. Initialize Authentication Service

In your server initialization:

```rust
use crate::auth::RedisAuthService;

// Create auth service
let redis_url = config.redis.as_ref()
    .map(|r| r.url.as_str())
    .unwrap_or("redis://localhost:6379");
let auth_service = RedisRedisAuthService::new(redis_url)?;

// Pass to your tools
let my_tool = MyAuthenticatedTool::new(auth_service);
```

## Redis Schema

### Session Mappings (5-10 min TTL)
```
mcp_session:{uuid4} -> {
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "user_id": "user123",
  "created_at": "2023-12-01T10:00:00Z",
  "expires_at": "2023-12-01T10:10:00Z"
}
```

### OAuth Tokens (persistent until revoked)
```
linked_account:{user_id}:{provider} -> {
  "user_id": "user123",
  "provider": "google",
  "provider_user_id": "google_user_456",
  "email": "user@example.com",
  "display_name": "John Doe",
  "access_token": "ya29.xxx",
  "refresh_token": "1//xxx",
  "expires_at": "2023-12-01T11:00:00Z",
  "scopes": [
    "https://www.googleapis.com/auth/userinfo.email",
    "https://www.googleapis.com/auth/userinfo.profile"
  ],
  "linked_at": "2023-12-01T09:00:00Z"
}
```

## Implementing Authenticated Tools

### Basic Pattern

```rust
use crate::auth::RedisAuthService;
use rmcp::{Error as McpError, model::{CallToolResult, Content}};
use serde::Deserialize;
use schemars::JsonSchema;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MyToolRequest {
    #[schemars(description = "Session ID for authenticated user")]
    pub session_id: String,
    // ... other parameters
}

#[derive(Clone)]
pub struct MyAuthenticatedTool {
    auth_service: AuthService,
}

impl MyAuthenticatedTool {
    pub fn new(auth_service: AuthService) -> Self {
        Self { auth_service }
    }

    pub async fn my_tool_method(&self, req: MyToolRequest) -> Result<CallToolResult, McpError> {
        // 1. Validate session ID format
        if let Err(e) = RedisAuthService::validate_session_format(&req.session_id) {
            return Err(McpError::invalid_params(
                format!("Invalid session ID: {}", e), 
                None
            ));
        }

        // 2. Authenticate and get OAuth token
        let token_data = self.auth_service
            .authenticate(&req.session_id, "google") // or your provider
            .await
            .map_err(|e| McpError::invalid_params(
                format!("Authentication failed: {}", e), 
                None
            ))?;

        // 3. Use the access token for API calls
        let access_token = &token_data.access_token;
        
        // Your tool logic here...
        
        Ok(CallToolResult::success(vec![Content::text("Success!")]))
    }
}
```

### HTTP Client Pattern

```rust
use reqwest::Client;

#[derive(Clone)]
pub struct HttpAuthenticatedTool {
    client: Client,
    auth_service: AuthService,
}

impl HttpAuthenticatedTool {
    pub fn new(auth_service: AuthService) -> Self {
        Self {
            client: Client::new(),
            auth_service,
        }
    }

    pub async fn api_call(&self, req: ApiRequest) -> Result<CallToolResult, McpError> {
        // Authenticate
        let token_data = self.auth_service
            .authenticate(&req.session_id, &req.provider)
            .await?;

        // Make authenticated HTTP request
        let response = self.client
            .get(&req.url)
            .bearer_auth(&token_data.access_token)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("HTTP request failed: {}", e), None))?;

        // Handle response...
    }
}
```

## Security Features

### Session ID Validation
- Must be valid UUID4 format
- Early validation before Redis lookup
- Prevents injection attacks

### OAuth Token Security
- Uses `zeroize` crate to securely clear tokens from memory
- Implements `ZeroizeOnDrop` for automatic cleanup
- Never logs OAuth tokens (only session IDs and user IDs)

### Error Handling
- Clear error messages for debugging
- Proper handling of Redis connection failures
- OAuth token expiration detection

## Backend Integration

### Session Management

Create sessions in Redis when users authenticate:

```python
import redis
import uuid
import json
from datetime import datetime, timedelta

r = redis.Redis(host='localhost', port=6379, db=0)

# When user authenticates successfully
session_id = str(uuid.uuid4())
user_id = "user123"

session_data = {
    "session_id": session_id,
    "user_id": user_id,
    "created_at": datetime.utcnow().isoformat() + "Z",
    "expires_at": (datetime.utcnow() + timedelta(minutes=10)).isoformat() + "Z"
}

# Store session with TTL (10 minutes)
r.setex(f"mcp_session:{session_id}", 600, json.dumps(session_data))
```

### OAuth Token Storage

Store OAuth tokens in Redis:

```python
oauth_data = {
    "user_id": user_id,
    "provider": "google",
    "provider_user_id": "google_123",
    "email": "user@example.com",
    "display_name": "John Doe",
    "access_token": "ya29.xxx",
    "refresh_token": "1//xxx",
    "expires_at": "2023-12-01T11:00:00Z",
    "scopes": [
        "https://www.googleapis.com/auth/userinfo.email",
        "https://www.googleapis.com/auth/userinfo.profile"
    ],
    "linked_at": datetime.utcnow().isoformat() + "Z"
}

r.set(f"linked_account:{user_id}:google", json.dumps(oauth_data))
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_validation() {
        // Valid UUID4
        assert!(RedisAuthService::validate_session_format("550e8400-e29b-41d4-a716-446655440000").is_ok());
        
        // Invalid formats
        assert!(RedisAuthService::validate_session_format("invalid-uuid").is_err());
    }

    #[tokio::test]
    async fn test_authentication_flow() {
        // This would require a test Redis instance
        // See integration tests for full examples
    }
}
```

### Integration Tests

Set up a test Redis instance and populate with test data:

```rust
#[tokio::test]
async fn test_full_authentication_flow() {
    // Setup test Redis with mock data
    // Test session resolution
    // Test OAuth token retrieval
    // Test token expiration handling
}
```

## Error Handling

### Common Error Types

```rust
// Invalid session ID format
Err(ServerError::InvalidSession("Invalid session ID format".to_string()))

// Session not found
Err(ServerError::InvalidSession("Session not found or expired".to_string()))

// OAuth token expired
Err(ServerError::InvalidSession("OAuth token expired".to_string()))

// Redis connection failure
Err(ServerError::Redis("Failed to connect to Redis".to_string()))
```

### Error Responses

```json
{
  "error": {
    "code": -32602,
    "message": "Invalid session ID: Invalid session ID format"
  }
}
```

## Security Considerations

1. **Redis Security**: Secure your Redis instance with authentication and network restrictions
2. **Session TTL**: Use short session timeouts (5-10 minutes) for security
3. **OAuth Scopes**: Request minimal required scopes for your use case
4. **Network Security**: Run MCP server in isolated network environment
5. **Token Rotation**: Implement OAuth token refresh in your backend
6. **Audit Logging**: Monitor session creation and usage patterns

## Troubleshooting

### Common Issues

**Authentication failures**:
- Verify session exists in Redis: `redis-cli get mcp_session:your-session-id`
- Check OAuth token format and expiration
- Ensure proper UUID4 format for session IDs

**Redis connection errors**:
- Check Redis connectivity: `redis-cli ping`
- Verify Redis URL in configuration
- Check network connectivity and firewall rules

**Token expiration**:
- Implement token refresh in your backend
- Monitor token expiration times
- Handle refresh token rotation

## Example Tools

The template includes example tools that demonstrate authentication patterns:

1. **`session_example.rs`** - Basic session validation and user info retrieval
2. **`http_client_example.rs`** - HTTP client with OAuth bearer token authentication

These examples show how to:
- Validate session IDs
- Handle authentication errors
- Make authenticated API calls
- Work with and without the auth feature enabled

## Migration Guide

If you have an existing MCP server and want to add authentication:

1. Add the `auth` feature to your `Cargo.toml`
2. Add Redis configuration to your config
3. Update your tools to accept `session_id` parameters
4. Replace direct token usage with `AuthService` calls
5. Update your backend to create sessions in Redis
6. Test the authentication flow end-to-end

## Resources

- [MCP Specification](https://spec.modelcontextprotocol.io)
- [Redis Documentation](https://redis.io/documentation)
- [OAuth 2.0 RFC](https://tools.ietf.org/html/rfc6749)
- [UUID4 Specification](https://tools.ietf.org/html/rfc4122)