# Authentication Guide

This MCP server template supports multiple authentication strategies. Choose the one that best fits your needs:

## No Authentication (Default)

By default, the template runs without any authentication. This is suitable for:
- Local development
- Private network deployments
- Single-user applications

## Available Authentication Methods

### 1. API Key Authentication (Simple)

Best for: Single-tenant applications, webhooks, simple integrations

**Enable in Cargo.toml:**
```toml
[features]
default = ["auth-apikey"]
```

**Configure API keys:**
```bash
# Set environment variable with key:user_id pairs
export API_KEYS="secret-key-123:user1,another-key-456:user2"
```

**Use in your tools:**
```rust
use crate::auth::{AuthProvider, ApiKeyAuthService};

#[derive(Clone)]
pub struct MyTool {
    auth: Box<dyn AuthProvider>,
}

impl MyTool {
    pub fn new() -> Self {
        Self {
            auth: Box::new(ApiKeyAuthService::from_env().unwrap()),
        }
    }
    
    pub async fn my_method(&self, api_key: String) -> Result<CallToolResult, McpError> {
        let auth_data = self.auth.authenticate(&api_key).await?;
        // Use auth_data.user_id for the authenticated user
    }
}
```

### 2. Redis Session-Based OAuth (Advanced)

Best for: Multi-tenant SaaS applications, complex authentication flows

This method provides:
- Session management with TTL
- OAuth token storage
- Multi-provider support (Google, GitHub, etc.)

See [AUTHENTICATION_ADVANCED.md](AUTHENTICATION_ADVANCED.md) for full implementation details.

**Enable in Cargo.toml:**
```toml
[features]
default = ["auth-redis"]
```

## Implementing Custom Authentication

You can create your own authentication provider:

```rust
use async_trait::async_trait;
use crate::auth::{AuthProvider, AuthData};

#[derive(Clone)]
pub struct MyCustomAuth {
    // Your auth implementation
}

#[async_trait]
impl AuthProvider for MyCustomAuth {
    async fn authenticate(&self, credential: &str) -> Result<AuthData, ServerError> {
        // Your authentication logic
        Ok(AuthData {
            user_id: "authenticated_user".to_string(),
            metadata: serde_json::json!({}),
        })
    }
}
```

## Choosing an Authentication Strategy

| Method | Use Case | Complexity | Security |
|--------|----------|------------|----------|
| None | Local dev, private networks | ⭐ | ⚠️ |
| API Key | Simple integrations | ⭐⭐ | ⭐⭐ |
| Redis/OAuth | Multi-tenant SaaS | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| Custom | Special requirements | Varies | Varies |

## Security Best Practices

1. **Always use HTTPS** in production when using authentication
2. **Rotate credentials** regularly
3. **Use environment variables** for sensitive configuration
4. **Implement rate limiting** for authentication endpoints
5. **Log authentication attempts** for audit trails

## Examples

The template includes example tools demonstrating authentication:
- `src/tools/session_example.rs` - Shows session validation
- `src/tools/http_client_example.rs` - Shows authenticated HTTP requests

## Migration from Non-Authenticated

To add authentication to an existing MCP server:

1. Choose your authentication method
2. Enable the appropriate feature flag
3. Update your tools to accept credentials
4. Test thoroughly before deploying

For questions or issues, please refer to the [MCP documentation](https://modelcontextprotocol.io).