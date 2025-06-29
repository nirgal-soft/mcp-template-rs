pub mod config;
pub mod error;
pub mod tools;
pub mod state;
pub mod telemetry;

use std::future::Future;
use std::net::SocketAddr;
use rmcp::{
  ServerHandler, ServiceExt, Error as McpError,
  schemars, tool, tool_handler, tool_router
};
use rmcp::transport::{stdio, streamable_http_server::{StreamableHttpService, StreamableHttpServerConfig}};
use rmcp::model::*;
use rmcp::handler::server::{router::tool::ToolRouter, tool::Parameters, wrapper::Json};
use tower::Service;

use crate::config::Config;
use crate::state::ServerState;
use crate::tools::dice_example::{DiceToolExample, RollRequestExample};

#[derive(Clone)]
pub struct Server {
  config: Config,
  #[allow(dead_code)]
  state: ServerState,
  tool_router: ToolRouter<Self>,
}

#[tool_router]
impl Server {
  // Replace with your own tools, these are for example
  #[tool(description = "Roll dice with specified number of sides")]
  pub async fn roll(&self, Parameters(RollRequestExample{count, sides}): Parameters<RollRequestExample>) -> Result<CallToolResult, McpError>{
    let req = RollRequestExample{count, sides};
    DiceToolExample.roll(req).await
  }

  #[tool(description = "Roll a standard six-sided die (d6)")]
  pub async fn roll_d6(&self) -> Result<CallToolResult, McpError>{
    self.roll(Parameters(RollRequestExample{count: 1, sides: 6})).await
  }

  #[tool(description = "Roll a standard twenty-sided die (d20)")]
  pub async fn roll_d20(&self) -> Result<CallToolResult, McpError>{
    self.roll(Parameters(RollRequestExample{count: 1, sides: 20})).await
  }
}

impl Server {
  pub async fn new(config: Config) -> anyhow::Result<Self> {
    tracing::info!("Initializing MCP Server");
    tracing::info!("Loading server state and tools...");
    
    let state = ServerState::new(&config).await?;
    
    tracing::info!("Server initialization complete");
    Ok(Self { config, state, tool_router: Self::tool_router(), })
  }

  pub async fn run(self) -> anyhow::Result<()> {
    match &self.config.server.transport {
      config::TransportType::Stdio => {
        tracing::info!("MCP Server ready!");
        tracing::info!("Transport: STDIO (Standard Input/Output)");
        
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
      }
      config::TransportType::HttpStreaming { port } => {
        tracing::info!("MCP Server ready!");
        tracing::info!("Transport: HTTP Streaming (using rmcp StreamableHttpService)");
        tracing::info!("Server URL: http://localhost:{}", port);
        
        let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
        
        // Create the rmcp StreamableHttpService
        use std::sync::Arc;
        use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
        
        let session_manager = Arc::new(LocalSessionManager::default());
        let config = StreamableHttpServerConfig::default();
        
        let service = StreamableHttpService::new(
          move || Ok(self.clone()),
          session_manager,
          config,
        );
        
        // Create HTTP server using axum
        let app = axum::Router::new()
          .fallback_service(tower::service_fn(move |req| {
            let mut service = service.clone();
            async move { service.call(req).await }
          }));
        
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let server = axum::serve(listener, app);
        
        // Set up graceful shutdown using the same pattern as STDIO
        let shutdown = tokio::spawn(async move {
          if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!("Failed to listen for shutdown signal: {}", e);
          }
          tracing::info!("Shutdown signal received");
        });

        tokio::select! {
          result = server => {
            match result {
              Ok(_) => tracing::info!("HTTP server stopped normally"),
              Err(e) => tracing::error!("HTTP server stopped with error: {}", e),
            }
          }
          _ = shutdown => {
            tracing::info!("Shutting down gracefully");
          }
        }
      }
    }

    Ok(())
  }
}

#[tool_handler]
impl ServerHandler for Server {
  fn get_info(&self) -> ServerInfo {
    ServerInfo {
      protocol_version: ProtocolVersion::default(),
      server_info: Implementation {
        name: self.config.server.name.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
      },
      capabilities: ServerCapabilities::builder()
        .enable_tools()
        .build(),
      // replace with your own instructions
      instructions: Some("A dice rolling server. Use the 'roll' tool to roll dice.".to_string()),
    }
  }
}
