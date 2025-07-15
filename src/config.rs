use serde::Deserialize;
use config::{Config as ConfigBuilder, ConfigError, Environment, File};
use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Context, Result};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
  pub server: ServerConfig,
  pub telemetry: TelemetryConfig,
  #[cfg(feature = "auth")]
  pub redis: Option<RedisConfig>,
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
  #[serde(rename = "http-streaming")]
  HttpStreaming { port: u16 },
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

#[cfg(feature = "auth")]
#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
  pub url: String,
}

#[cfg(feature = "database")]
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
  pub url: String,
  pub max_connections: u32,
}

impl Config {
  /// Get the config directory path: ~/.config/{project-name}/
  fn get_config_dir() -> Result<PathBuf> {
    let home_dir = dirs::home_dir()
      .context("Could not determine home directory")?;
    
    let project_name = env!("CARGO_PKG_NAME");
    Ok(home_dir.join(".config").join(project_name))
  }

  /// Get the config file path: ~/.config/{project-name}/config.toml
  fn get_config_path() -> Result<PathBuf> {
    Ok(Self::get_config_dir()?.join("config.toml"))
  }

  /// Ensure config directory exists and create config file if needed
  fn ensure_config_exists() -> Result<PathBuf> {
    let config_dir = Self::get_config_dir()?;
    let config_path = Self::get_config_path()?;

    // Create config directory if it doesn't exist
    if !config_dir.exists() {
      fs::create_dir_all(&config_dir)
        .with_context(|| format!("Failed to create config directory: {}", config_dir.display()))?;
      tracing::info!("Created config directory: {}", config_dir.display());
    }

    // Create config file if it doesn't exist
    if !config_path.exists() {
      // Check if user has prepared a config.toml file in current directory
      if Path::new("config.toml").exists() {
        let config_content = fs::read_to_string("config.toml")
          .context("Failed to read user-prepared config.toml")?;
        fs::write(&config_path, config_content)
          .with_context(|| format!("Failed to write config to: {}", config_path.display()))?;
        tracing::info!("Created config file from user-prepared config.toml: {}", config_path.display());
      } else {
        // Use built-in stdio default
        let default_config = Self::get_default_config_content();
        fs::write(&config_path, default_config)
          .with_context(|| format!("Failed to write config to: {}", config_path.display()))?;
        tracing::info!("Created default config file (stdio): {}", config_path.display());
      }
    }

    Ok(config_path)
  }

  /// Get the configuration file paths in priority order
  /// 1. ~/.config/{project-name}/config.toml (user config)
  /// 2. /config.toml (Docker mount)
  fn get_config_sources() -> Result<Vec<PathBuf>, ConfigError> {
    let mut sources = Vec::new();

    // First, ensure user config exists
    match Self::ensure_config_exists() {
      Ok(user_config) => sources.push(user_config),
      Err(e) => {
        tracing::warn!("Failed to ensure user config exists: {}", e);
        // Continue without user config in environments where home dir might not be available
      }
    }

    // Docker mount config (container environment)
    let docker_config = Path::new("/config.toml");
    if docker_config.exists() {
      sources.push(docker_config.to_path_buf());
      tracing::info!("Using Docker mounted config: {}", docker_config.display());
    }

    if sources.is_empty() {
      return Err(ConfigError::Message(
        format!("No configuration file found. Expected ~/.config/{}/config.toml", env!("CARGO_PKG_NAME"))
      ));
    }

    Ok(sources)
  }

  /// Get the default configuration content (stdio transport)
  fn get_default_config_content() -> String {
    let project_name = env!("CARGO_PKG_NAME");
    format!(r#"# Configuration for {}
# This file was auto-generated on first run
# Edit as needed and restart the service

[server]
name = "{}"
transport = "stdio"

[telemetry]
level = "info"
format = "pretty"
# Log to file when using stdio to avoid interfering with JSON-RPC
file = "/tmp/{}.log"

# Uncomment and configure if using auth feature
# [redis]
# url = "redis://localhost:6379"

# Uncomment and configure if using database feature
# [database]
# url = "sqlite:///tmp/{}.db"
# max_connections = 10
"#, project_name, project_name, project_name, project_name)
  }

  pub fn load() -> Result<Self, ConfigError> {
    // Get configuration sources in priority order
    let config_sources = Self::get_config_sources()?;
    
    let mut builder = ConfigBuilder::builder()
      // Start with default values
      .set_default("server.name", env!("CARGO_PKG_NAME"))?
      // NOTE: No default transport - let config file decide to avoid override issues
      .set_default("telemetry.level", "info")?
      .set_default("telemetry.format", "pretty")?;

    #[cfg(feature = "auth")]
    {
      builder = builder.set_default("redis.url", "redis://localhost:6379")?;
    }

    // Add config sources in reverse order (last one wins for overlapping keys)
    // This allows local/docker configs to override user config
    for config_path in config_sources.iter().rev() {
      tracing::info!("Loading config from: {}", config_path.display());
      builder = builder.add_source(File::from(config_path.as_path()).required(false));
    }

    // Add environment variables with MCP_ prefix (highest priority)
    builder = builder.add_source(Environment::with_prefix("MCP").separator("_"));

    let config = builder.build()?;
    let mut config: Config = config.try_deserialize()?;
    
    // Force logging to file for stdio transport to avoid interfering with JSON-RPC
    if matches!(config.server.transport, TransportType::Stdio) && config.telemetry.file.is_none() {
      let log_file = format!("/tmp/{}.log", env!("CARGO_PKG_NAME"));
      config.telemetry.file = Some(log_file);
    }

    Ok(config)
  }
}
