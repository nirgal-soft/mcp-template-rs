# Agent Guidelines for MCP Template
THIS IS A TEMPLATE DIRECTORY
This means, do not attempt to build or test anything.

## Code Style
- Use `snake_case` for functions, variables, modules
- Use `PascalCase` for types, structs, enums
- Import order: std, external crates, local modules
- Use `thiserror` for custom errors, `anyhow` for general error handling
- Prefer `async/await` over manual futures
- Use `tracing` for logging (info, warn, error, debug)
- Add `#[derive(Debug)]` to structs when possible
- Use `serde` with `#[derive(Deserialize, Serialize)]` for JSON
- Add `schemars::JsonSchema` for MCP tool parameters

## Error Handling
- Return `Result<T, McpError>` for MCP tools
- Use `anyhow::Result<T>` for general functions
- Validate inputs early and return descriptive errors
- Use `McpError::invalid_params()` for parameter validation
