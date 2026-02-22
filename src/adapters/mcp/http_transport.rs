//! MCP HTTP transport for communicating with MCP servers over HTTP/HTTPS

use super::types::*;
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tokio::sync::Mutex;

/// MCP HTTP transport client
pub struct McpHttpTransport {
    /// HTTP client
    client: Client,
    /// Server URL
    server_url: String,
    /// Request ID counter
    next_id: Arc<Mutex<i64>>,
}

impl McpHttpTransport {
    /// Create a new HTTP transport connected to the given URL
    pub fn new(url: String) -> Result<Self> {
        // Validate URL
        let parsed = url::Url::parse(&url)
            .context("Invalid MCP server URL")?;

        // Ensure it's http or https
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            bail!("MCP HTTP transport only supports http:// and https:// URLs, got: {}", parsed.scheme());
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            server_url: url,
            next_id: Arc::new(Mutex::new(1i64)),
        })
    }

    /// Send a request and wait for response
    pub async fn send_request(&self, method: &str, params: Option<JsonValue>) -> Result<JsonValue> {
        // Generate request ID
        let id = {
            let mut next_id = self.next_id.lock().await;
            let id = *next_id;
            *next_id += 1;
            id
        };

        // Build JSON-RPC request
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: RequestId::Number(id),
        };

        tracing::debug!("Sending MCP HTTP request: {} to {}", method, self.server_url);

        // Send HTTP POST request
        let response = self.client
            .post(&self.server_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send HTTP request to MCP server")?;

        // Check HTTP status
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error body".to_string());
            bail!(
                "MCP server returned HTTP error: {} - {}",
                status,
                error_body
            );
        }

        // Parse response
        let json_response: JsonRpcResponse = response
            .json()
            .await
            .context("Failed to parse MCP server response")?;

        // Check for JSON-RPC error
        if let Some(error) = json_response.error {
            bail!(
                "MCP server returned error: {} - {}",
                error.code,
                error.message
            );
        }

        // Return result
        json_response
            .result
            .context("MCP server response missing result field")
    }

    /// Initialize the MCP session
    pub async fn initialize(&self) -> Result<InitializeResult> {
        tracing::info!("Initializing MCP HTTP session with {}", self.server_url);

        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "roots": {
                    "listChanged": true
                }
            },
            "clientInfo": {
                "name": "uxc",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        let result = self.send_request("initialize", Some(params)).await?;

        serde_json::from_value(result).context("Failed to parse initialize result")
    }

    /// List available tools
    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let result = self.send_request("tools/list", None).await?;

        let response: ToolsListResponse = serde_json::from_value(result)
            .context("Failed to parse tools/list response")?;

        Ok(response.tools)
    }

    /// Call a tool
    pub async fn call_tool(&self, name: &str, arguments: Option<JsonValue>) -> Result<ToolCallResult> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });

        let result = self.send_request("tools/call", Some(params)).await?;

        serde_json::from_value(result).context("Failed to parse tools/call result")
    }

    /// List available resources
    pub async fn list_resources(&self) -> Result<Vec<Resource>> {
        let result = self.send_request("resources/list", None).await?;

        let response: ResourcesListResponse = serde_json::from_value(result)
            .context("Failed to parse resources/list response")?;

        Ok(response.resources)
    }

    /// Read a resource
    pub async fn read_resource(&self, uri: &str) -> Result<ResourceContents> {
        let params = serde_json::json!({
            "uri": uri
        });

        let result = self.send_request("resources/read", Some(params)).await?;

        serde_json::from_value(result).context("Failed to parse resources/read result")
    }

    /// List available prompts
    pub async fn list_prompts(&self) -> Result<Vec<Prompt>> {
        let result = self.send_request("prompts/list", None).await?;

        let response: PromptsListResponse = serde_json::from_value(result)
            .context("Failed to parse prompts/list response")?;

        Ok(response.prompts)
    }

    /// Get a prompt
    pub async fn get_prompt(&self, name: &str, arguments: Option<JsonValue>) -> Result<GetPromptResult> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });

        let result = self.send_request("prompts/get", Some(params)).await?;

        serde_json::from_value(result).context("Failed to parse prompts/get result")
    }
}
