use std::time::Instant;
use crate::config::Config;
use anyhow::Result;

#[cfg(feature = "database")]
use std::sync::Arc;

#[derive(Clone)]
pub struct ServerState {
  start_time: Instant,
  // Add your shared state here
  #[cfg(feature = "database")]
  pub db: Option<Arc<sqlx::SqlitePool>>,
}

impl ServerState {
  pub async fn new(_config: &Config) -> Result<Self> {
    #[cfg(feature = "database")]
    let mut state = Self {
      start_time: Instant::now(),
      db: None,
    };

    #[cfg(not(feature = "database"))]
    let state = Self {
      start_time: Instant::now(),
    };

    #[cfg(feature = "database")]
    if let Some(db_config) = &_config.database {
      let pool = sqlx::SqlitePool::connect(&db_config.url).await?;
      state.db = Some(Arc::new(pool));
    }

    Ok(state)
  }

  pub fn uptime(&self) -> std::time::Duration {
    self.start_time.elapsed()
  }
}
