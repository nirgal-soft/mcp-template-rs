# MCP Rust Server Template

A production-ready template for building [Model Context Protocol (MCP)](https://modelcontextprotocol.io) servers in Rust. This template provides a solid foundation with best practices, deployment configurations, and a clean architecture for creating MCP servers that AI assistants can use to interact with external tools and data sources.

## Features

- **Production Ready** - Structured logging, error handling, and graceful shutdown
- **Easy to Extend** - Clear patterns for adding new tools, resources, and prompts
- **Multiple Deployment Options** - Docker, systemd, or standalone binary
- **Flexible Configuration** - Environment variables and config files
- **Testing Infrastructure** - Unit and integration test setup
- **Telemetry Built-in** - Structured logging with tracing
- **Modular Architecture** - Clean separation of concerns

## Quick Start

### Using as a Template

1. **Clone this repository**
   ```bash
   git clone https://github.com/yourusername/mcp-rust-template my-mcp-server
   cd my-mcp-server
   rm -rf .git  # Remove template git history
   git init     # Start fresh
   ```

2. **Update project information**
   - Edit `Cargo.toml` and change the package name
   - Update `config.toml.example` with your defaults
   - Modify `src/lib.rs` to add your server name

3. **Run the example**
   ```bash
   cp config.toml.example config.toml
   cargo run
   ```

4. **Test with an MCP client**
   ```bash
   # In another terminal
   echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | cargo run
   ```

## Project Structure

```
.
├── src/
│   ├── main.rs          # Entry point
│   ├── lib.rs           # Server implementation
│   ├── config.rs        # Configuration management
│   ├── error.rs         # Error types
│   ├── state.rs         # Shared server state
│   ├── telemetry.rs     # Logging setup
│   └── tools/           # Your MCP tools
│       ├── mod.rs       # Tool registry
│       └── example.rs   # Example tool implementation
├── tests/
│   └── integration_test.rs  # Integration tests
├── deploy/
│   ├── Dockerfile       # Container deployment
│   ├── docker-compose.yml
│   └── systemd/         # Linux service files
├── config.toml.example  # Example configuration
└── ci/                  # CI/CD templates (optional)
```

## Adding Tools

Tools are the core of MCP - they're functions that AI assistants can call. Here's how to add a new tool:

1. **Create a new file** in `src/tools/` (e.g., `src/tools/weather.rs`):

```rust
use rmcp::{tool, tool_box};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use anyhow::Result;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WeatherRequest {
    #[schemars(description = "City name")]
    pub city: String,
}

#[derive(Clone)]
pub struct WeatherTool;

#[tool_box]
impl WeatherTool {
    #[tool(description = "Get current weather for a city")]
    pub async fn get_weather(&self, #[tool(aggr)] req: WeatherRequest) -> Result<String> {
        // Your implementation here
        Ok(format!("Weather in {}: Sunny, 72°F", req.city))
    }
}
```

2. **Register the tool** in `src/tools/mod.rs`:

```rust
pub mod weather;
pub use weather::WeatherTool;
```

3. **Add to server** in `src/lib.rs`:

```rust
#[tool_box]
impl Server {
    // Delegate to weather tool
    #[tool(description = "Get current weather for a city")]
    pub async fn get_weather(&self, #[tool(aggr)] req: WeatherRequest) -> Result<String> {
        WeatherTool.get_weather(req).await
    }
}
```

## Configuration

The server can be configured via:

1. **Config file** (`config.toml`):
   ```toml
   [server]
   name = "my-mcp-server"
   transport = "stdio"  # or { type = "http", port = 8080 }

   [telemetry]
   level = "info"
   format = "pretty"  # or "json"
   file = "server.log"  # optional, defaults to stdout
   ```

2. **Environment variables** (prefixed with `MCP_`):
   ```bash
   MCP_SERVER_NAME=my-server
   MCP_TELEMETRY_LEVEL=debug
   cargo run
   ```

## Deployment

### Docker

```bash
# Build
docker build -t my-mcp-server .

# Run
docker run -it my-mcp-server

# With config
docker run -it -v $(pwd)/config.toml:/config.toml my-mcp-server
```

### Docker Compose

```bash
docker-compose up
```

### Systemd (Linux)

```bash
# Copy binary
sudo cp target/release/my-mcp-server /usr/local/bin/

# Install service
sudo cp deploy/systemd/mcp-server.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable mcp-server
sudo systemctl start mcp-server
```

### Standalone

```bash
cargo build --release
./target/release/my-mcp-server
```

## Testing

```bash
# Run all tests
cargo test

# Run with logging
RUST_LOG=debug cargo test -- --nocapture

# Integration tests only
cargo test --test integration_test
```

## Development Tips

1. **Enable debug logging**:
   ```bash
   RUST_LOG=debug cargo run
   ```

2. **Test individual tools**:
   ```bash
   # Create a test script
   echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"server_info"},"id":1}' | cargo run
   ```

3. **Use the MCP Inspector**:
   ```bash
   npx @modelcontextprotocol/inspector cargo run
   ```

## Common Patterns

### Stateful Tools

For tools that need to maintain state:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct CounterTool {
    count: Arc<RwLock<i64>>,
}

#[tool_box]
impl CounterTool {
    #[tool(description = "Increment counter")]
    async fn increment(&self) -> Result<String> {
        let mut count = self.count.write().await;
        *count += 1;
        Ok(format!("Count: {}", *count))
    }
}
```

### External Services

For tools that call external APIs:

```rust
pub struct ApiTool {
    client: reqwest::Client,
    api_key: String,
}

#[tool_box]
impl ApiTool {
    #[tool(description = "Call external API")]
    async fn call_api(&self, #[tool(param)] endpoint: String) -> Result<String> {
        let response = self.client
            .get(&endpoint)
            .header("Authorization", &self.api_key)
            .send()
            .await?;
        Ok(response.text().await?)
    }
}
```

## Troubleshooting

### Server won't start
- Check logs: `RUST_LOG=debug cargo run`
- Verify config file exists and is valid TOML
- Ensure no other process is using the port (for HTTP transport)

### Tools not showing up
- Ensure tools are properly registered in `src/tools/mod.rs`
- Check that `tool_box!(@impl self)` is in your `ServerHandler` impl
- Verify tool methods are marked with `#[tool(...)]`

### Client connection issues
- For stdio: ensure client is piping correctly
- For HTTP: check firewall and port availability
- Enable debug logging to see raw JSON-RPC messages

## Contributing

This is a template repository - fork it and make it your own! If you have improvements to the template itself:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

## Resources

- [MCP Specification](https://spec.modelcontextprotocol.io)
- [MCP Documentation](https://modelcontextprotocol.io/docs)
- [Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [MCP Inspector](https://github.com/modelcontextprotocol/inspector)

## License

This template is released under the MIT License. See `license.txt` for details.
