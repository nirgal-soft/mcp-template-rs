# MCP Rust Server Template

A production-ready template for building [Model Context Protocol (MCP)](https://modelcontextprotocol.io) servers in Rust. This template provides a solid foundation with best practices, deployment configurations, and a clean architecture for creating MCP servers that AI assistants can use to interact with external tools and data sources.

## Features

- **Production Ready** - Structured logging, error handling, and graceful shutdown
- **Easy to Extend** - Clear patterns for adding new tools, resources, and prompts
- **Flexible Configuration** - Environment variables and config files
- **Telemetry Built-in** - Structured logging with tracing
- **Modular Architecture** - Clean separation of concerns
- **Testing Infrastructure** - Unit and integration test setup
- **Multiple Deployment Options** - Docker, systemd, or standalone binary
- **Multiple Transport Options** - stdio, HTTP streaming, and worker thread transports

## Quick Start

### Generate a New Project with cargo-generate

The easiest way to use this template is with [`cargo-generate`](https://github.com/cargo-generate/cargo-generate):

1. **Install cargo-generate** (if you haven't already):
   ```bash
   cargo install cargo-generate
   ```

2. **Generate your new MCP server**:
   ```bash
   cargo generate --git https://git.sr.ht/~nirgal/mcp-template
   ```
   
   You'll be prompted to enter:
   - **Project name**: This will be used as the crate name and binary name
   - The tool will automatically set up all the necessary files with your project name

3. **Navigate to your new project and run it**:
   ```bash
   cd your-project-name
   cp config.toml.example config.toml
   cargo run
   ```

4. **Test with an MCP client**:
   ```bash
   # In another terminal
   echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | cargo run
   ```

### Alternative: Using the --git flag directly

You can also generate a project without installing cargo-generate globally:

```bash
# Generate directly with git URL
cargo generate --git https://git.sr.ht/~nirgal/mcp-template --name my-awesome-mcp-server

# Or clone and generate locally
git clone https://git.sr.ht/~nirgal/mcp-template
cargo generate --path ./mcp-template --name my-awesome-mcp-server
```

## Project Structure

After generating your project, you'll have the following structure:

```
your-project-name/
├── src/
│   ├── main.rs              # Entry point and command-line interface
│   ├── lib.rs               # Server implementation and tool definitions
│   ├── config.rs            # Configuration management
│   ├── error.rs             # Error types and handling
│   ├── state.rs             # Shared server state
│   ├── telemetry.rs         # Logging and tracing setup
│   └── tools/               # Your MCP tools
│       ├── mod.rs           # Tool registry and exports
│       └── dice_example.rs  # Example dice rolling tool
├── tests/
│   └── integration_test.rs  # Integration tests
├── deploy/
│   ├── Dockerfile           # Container deployment
│   ├── docker-compose.yml   # Multi-container setup
│   └── systemd/             # Linux service files
│       └── mcp-server.service
├── ci/                      # Empty directory for your CI/CD files
├── Cargo.toml               # Project configuration (with your project name)
├── Cargo.lock               # Dependency lock file
├── config.toml.example      # Example configuration file
├── .env.example             # Example environment variables
├── .gitignore               # Git ignore rules
└── license.txt              # MIT license
```

## Adding Tools

Tools are the core of MCP - they're functions that AI assistants can call. The template includes a dice rolling example to get you started.

### Example: The Dice Tool

The template includes `src/tools/dice_example.rs` which demonstrates best practices:

```rust
use rand::Rng;
use rmcp::{Error as McpError, model::{CallToolResult, Content}};
use serde::Deserialize;
use schemars::JsonSchema;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RollRequestExample {
    #[schemars(description = "Number of sides on the dice (e.g. 6 for d6, 20 for d20)")]
    pub sides: u32,
    #[schemars(description = "Number of dice to roll")]
    #[serde(default = "default_count")]
    pub count: u32,
}

fn default_count() -> u32 { 1 }

#[derive(Clone)]
pub struct DiceToolExample;

impl DiceToolExample {
    pub async fn roll(&self, req: RollRequestExample) -> Result<CallToolResult, McpError> {
        // Input validation
        if req.sides == 0 {
            return Err(McpError::invalid_params("Dice must have at least 1 side", None));
        }
        if req.count == 0 || req.count > 100 {
            return Err(McpError::invalid_params("Count must be between 1 and 100", None));
        }

        // Business logic
        let mut rng = rand::rng();
        let rolls: Vec<u32> = (0..req.count)
            .map(|_| rng.random_range(1..=req.sides))
            .collect();

        let total: u32 = rolls.iter().sum();

        // Format results
        let result_text = if req.count == 1 {
            format!("Rolled a d{}: {}", req.sides, rolls[0])
        } else {
            format!("Rolled {}d{}: {} (total: {})", 
                    req.count, req.sides,
                    rolls.iter().map(|r| r.to_string()).collect::<Vec<String>>().join(", "),
                    total)
        };

        Ok(CallToolResult::success(vec![Content::text(result_text)]))
    }
}
```

### Adding Your Own Tools

1. **Create a new file** in `src/tools/` (e.g., `src/tools/weather.rs`):

```rust
use rmcp::{Error as McpError, model::{CallToolResult, Content}};
use serde::Deserialize;
use schemars::JsonSchema;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WeatherRequest {
    #[schemars(description = "City name")]
    pub city: String,
}

#[derive(Clone)]
pub struct WeatherTool;

impl WeatherTool {
    pub async fn get_weather(&self, req: WeatherRequest) -> Result<CallToolResult, McpError> {
        // Your implementation here
        let result = format!("Weather in {}: Sunny, 72°F", req.city);
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }
}
```

2. **Register the tool** in `src/tools/mod.rs`:

```rust
pub mod weather;
pub use weather::WeatherTool;
```

3. **Add to server** in `src/lib.rs` by adding to the match statement in the `call_tool` method.

## Configuration

The server can be configured via:

1. **Config file** (`config.toml`):
   ```toml
   [server]
   name = "my-mcp-server"
   # Choose your transport method:
   transport = "stdio"  # For direct stdio communication
   # OR
   transport = { http-streaming = { port = 8080 } }  # For HTTP streaming

   [telemetry]
   level = "info"        # debug, info, warn, error
   format = "pretty"     # pretty, json
   # file = "server.log" # optional, defaults to stdout
   ```

2. **Environment variables** (prefixed with `MCP_`):
   ```bash
   MCP_SERVER_NAME=my-server
   MCP_TELEMETRY_LEVEL=debug
   cargo run
   ```

3. **Command line arguments**:
   ```bash
   # Use stdio transport (default)
   cargo run

   # Use HTTP streaming on port 3000
   cargo run -- --http-port 3000

   # Enable debug logging
   RUST_LOG=debug cargo run
   ```

## Why cargo-generate?

Using `cargo generate --git` with this template gives you several advantages:

- **Automatic project setup** - No manual find-and-replace needed
- **Template variables** - Your project name is automatically configured throughout
- **Clean git history** - Start with a fresh repository
- **Latest version** - Always get the newest template version
- **No cloning overhead** - Only download what you need

### Supported cargo-generate variables:

- `project-name` - Used for crate name, binary name, and default server name

## Template Development Workflow

If you're contributing to or customizing this template:

```bash
# Clone the template repository
git clone https://git.sr.ht/~nirgal/mcp-template
cd mcp-template

# Test your changes locally
cargo generate --path . --name test-project
cd test-project
cargo run

# Or test with the git URL
cargo generate --git https://git.sr.ht/~nirgal/mcp-template --name test-project
```

```

## Deployment

### Docker

```bash
# Build your generated project
docker build -t my-mcp-server .

# Run with stdio transport
docker run -it my-mcp-server

# Run with HTTP transport
docker run -p 8080:8080 my-mcp-server

# With custom config
docker run -it -v $(pwd)/config.toml:/config.toml my-mcp-server
```

### Docker Compose

```bash
# Use the included docker-compose.yml
docker-compose up
```

### Systemd (Linux)

```bash
# Build release binary
cargo build --release

# Copy binary
sudo cp target/release/your-project-name /usr/local/bin/

# Install service (adjust paths in deploy/systemd/mcp-server.service first)
sudo cp deploy/systemd/mcp-server.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable mcp-server
sudo systemctl start mcp-server
```

### Standalone

```bash
# Build and run
cargo build --release
./target/release/your-project-name
```

## Testing

```bash
# Run all tests
cargo test

# Run with logging
RUST_LOG=debug cargo test -- --nocapture

# Integration tests only
cargo test --test integration_test

# Test with MCP Inspector
npx @modelcontextprotocol/inspector cargo run
```

## Development Tips

1. **Enable debug logging**:
   ```bash
   RUST_LOG=debug cargo run
   ```

2. **Test individual tools**:
   ```bash
   # List available tools
   echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | cargo run
   
   # Call the dice tool
   echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"roll","arguments":{"sides":20,"count":2}},"id":1}' | cargo run
   ```

3. **Use the MCP Inspector** for interactive testing:
   ```bash
   npx @modelcontextprotocol/inspector cargo run
   ```

## Common Patterns

### Stateful Tools

For tools that need to maintain state, use the server's state system:

```rust
// In your tool implementation
impl WeatherTool {
    pub async fn get_weather(&self, req: WeatherRequest) -> Result<CallToolResult, McpError> {
        // Access shared state if needed
        // self.state.some_data.read().await;
        
        // Your implementation
        Ok(CallToolResult::success(vec![Content::text("Weather data")]))
    }
}
```

### External Services

For tools that call external APIs, consider using the optional dependencies:

```rust
// Add to Cargo.toml: features = ["http-client"]
// This enables the reqwest dependency

use reqwest::Client;

pub struct ApiTool {
    client: Client,
}

impl ApiTool {
    pub async fn call_api(&self, endpoint: String) -> Result<CallToolResult, McpError> {
        let response = self.client
            .get(&endpoint)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("API call failed: {}", e), None))?;
            
        let text = response.text().await
            .map_err(|e| McpError::internal_error(format!("Failed to read response: {}", e), None))?;
            
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}
```

## Troubleshooting

### Server won't start
- Check logs: `RUST_LOG=debug cargo run`
- Verify config file exists and is valid TOML
- Ensure no other process is using the port (for HTTP transport)

### Tools not showing up
- Check that tools are properly registered in `src/tools/mod.rs`
- Verify tool methods match the expected signature
- Enable debug logging to see registration details

### Client connection issues
- For stdio: ensure client is piping JSON-RPC correctly
- For HTTP: check firewall and port availability
- Use MCP Inspector to debug JSON-RPC messages

## Template Repository

This template is maintained at:
- **Git Repository**: <https://git.sr.ht/~nirgal/mcp-template>
- **Use with**: `cargo generate --git https://git.sr.ht/~nirgal/mcp-template`

## Resources

- [MCP Specification](https://spec.modelcontextprotocol.io)
- [MCP Documentation](https://modelcontextprotocol.io/docs)
- [Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [MCP Inspector](https://github.com/modelcontextprotocol/inspector)
- [cargo-generate Documentation](https://github.com/cargo-generate/cargo-generate)

## License

This template is released under the MIT License. See `license.txt` for details.
