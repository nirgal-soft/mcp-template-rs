pub mod config;
pub mod error;
pub mod tools;
pub mod state;
pub mod telemetry;

use rmcp::{ServerHandler, ServiceExt};
use rmcp::transport::stdio;
use rmcp::model::{InitializeRequestParam, InitializeResult, Implementation};
use rmcp::service::{RequestContext, RoleServer};
use anyhow::Result;

use crate::config::Config;
use crate::state::ServerState;

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
    let transport = stdio();
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

  // Example tool - replace with your own
  async fn server_info(&self) -> Result<String, rmcp::Error> {
    Ok(serde_json::json!({
      "name": self.config.server.name,
      "version": env!("CARGO_PKG_VERSION"),
      "uptime": self.state.uptime().as_secs(),
    }).to_string())
  }
}

impl ServerHandler for Server {
  fn initialize(
    &self, 
    _params: InitializeRequestParam,
    _context: RequestContext<RoleServer>
  ) -> impl std::future::Future<Output = Result<InitializeResult, rmcp::Error>> + Send + '_ {
    std::future::ready(Ok(InitializeResult {
      protocol_version: Default::default(),
      server_info: Implementation {
        name: self.config.server.name.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
      },
      capabilities: Default::default(),
      instructions: None,
    }))
  }

  fn list_tools(
    &self,
    _: rmcp::model::PaginatedRequestParam,
    _: RequestContext<RoleServer>,
  ) -> impl std::future::Future<Output = Result<rmcp::model::ListToolsResult, rmcp::Error>> + Send + '_ {
    std::future::ready(Ok(rmcp::model::ListToolsResult {
      next_cursor: None,
      tools: vec![
        // Add tools here manually for now
      ],
    }))
  }

  fn call_tool(
    &self,
    _call_tool_request_param: rmcp::model::CallToolRequestParam,
    _context: RequestContext<RoleServer>,
  ) -> impl std::future::Future<Output = Result<rmcp::model::CallToolResult, rmcp::Error>> + Send + '_ {
    std::future::ready(Err(rmcp::Error::method_not_found::<rmcp::model::CallToolRequestMethod>()))
  }
}
