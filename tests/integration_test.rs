use {{crate_name}}::{config::Config, Server};
use rmcp::{Client, ServiceExt};
use rmcp::transport::stdio::testing::StdioPair;

#[tokio::test]
async fn test_server_info_tool() {
// Create test config
let config = Config {
server: {{crate_name}}::config::ServerConfig {
  name: "test-server".to_string(),
  transport: {{crate_name}}::config::TransportType::Stdio,
  },
  telemetry: {{crate_name}}::config::TelemetryConfig {
    level: "error".to_string(),
    format: {{crate_name}}::config::LogFormat::Pretty,
    file: None,
  },
};

  // Create transport pair
  let (server_transport, client_transport) = StdioPair::new();

  // Start server
  let server = Server::new(config).await.unwrap();
  let server_handle = tokio::spawn(async move {
    let service = server.serve(server_transport).await.unwrap();
    service.waiting().await
  });

  // Create client
  let client = Client::new("test-client", "1.0");
  let client_service = client.serve(client_transport).await.unwrap();

  // Initialize
  client_service.initialize().await.unwrap();

  // Call tool
  let result = client_service.call_tool("server_info", serde_json::json!({}))
    .await
    .unwrap();

  // Verify
  let content = result.content[0].as_str().unwrap();
  assert!(content.contains("test-server"));

  // Cleanup
  client_service.shutdown().await.unwrap();
  server_handle.abort();
}
