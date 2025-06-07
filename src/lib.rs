pub mod config;
pub mod error;
pub mod tools;
pub mod state;
pub mod telemetry;

use rmcp::{ServerHandler, ServiceExt, tool_box};
use rmcp::transport::stdio::StdioTransport;
use rmcp::protocol::{InitializeParams, InitializeResult, ServerInfo};
use anyhow::Result;

use crate::config::Config;
use crate::state::ServerState;
use crate::tools::ToolRegistry;

#[derive(Clone)]
pub struct Server {
  config: Config,
  state: ServerState,
}

impl Server {
  pub async fn new(config: Config) -> Result<Self> {
    let state = ServerState::new(&config).await?;
    Ok(Self { config, state })
  }

  pub async fn run(self) -> Result<()> {
    let transport = StdioTransport::new();
    let service = self.serve(transport).await?;

    // Set up graceful shutdown
    let shutdown = tokio::spawn(async move {
      tokio::signal::ctrl_c().await.ok();
      tracing::info!("Shutdown signal received");
    });

    tokio::select! {
      result = service.waiting() => {
        tracing::info!("Server stopped: {:?}", result);
      }
        _ = shutdown => {
          tracing::info!("Shutting down gracefully");
        }
    }

    Ok(())
  }
}

#[tool_box]
impl Server {
  // Example tool - replace with your own
  #[tool(description = "Get server information")]
  async fn server_info(&self) -> Result<String> {
    Ok(serde_json::json!({
      "name": self.config.server.name,
      "version": env!("CARGO_PKG_VERSION"),
      "uptime": self.state.uptime().as_secs(),
    }).to_string())
  }
}

#[async_trait::async_trait]
impl ServerHandler for Server {
  async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
    Ok(InitializeResult {
      protocol_version: "1.0".to_string(),
      server_info: ServerInfo {
        name: self.config.server.name.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
      },
      capabilities: Default::default(),
    })
  }

  tool_box!(@impl self);

  // Add resources and prompts implementations as needed
}
