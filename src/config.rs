use serde::Deserialize;
use config::{Config as ConfigBuilder, ConfigError, Environment, File};
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
  pub server: ServerConfig,
  pub telemetry: TelemetryConfig,
  #[cfg(feature = "database")]
  pub database: Option<DatabaseConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
  pub name: String,
  pub transport: TransportType,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TransportType {
  Stdio,
  Http { port: u16 },
}

#[derive(Debug, Deserialize, Clone)]
pub struct TelemetryConfig {
  pub level: String,
  pub format: LogFormat,
  pub file: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
  Pretty,
  Json,
}

#[cfg(feature = "database")]
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
  pub url: String,
  pub max_connections: u32,
}

impl Config {
  pub fn load() -> Result<Self, ConfigError> {
    let config = ConfigBuilder::builder()
      // Start with default values
      .set_default("server.name", "mcp-server")?
      .set_default("server.transport", "stdio")?
      .set_default("telemetry.level", "info")?
      .set_default("telemetry.format", "pretty")?
      // Add config file
      .add_source(File::from(Path::new("config.toml")).required(false))
      // Add environment variables with MCP_ prefix
      .add_source(Environment::with_prefix("MCP").separator("_"))
      .build()?;

    config.try_deserialize()
  }
}
