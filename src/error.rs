use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
  #[error("Configuration error: {0}")]
  Config(#[from] config::ConfigError),

  #[error("Tool execution failed: {0}")]
  ToolExecution(String),

  #[error("Resource not found: {0}")]
  ResourceNotFound(String),

  #[error("Invalid input: {0}")]
  InvalidInput(String),

  #[cfg(feature = "database")]
  #[error("Database error: {0}")]
  Database(#[from] sqlx::Error),

  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),
}
