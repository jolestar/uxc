//! MCP HTTP transport for communicating with MCP servers over HTTP/HTTPS

#![allow(dead_code)]

use super::types::*;
use crate::auth::Profile;
use anyhow::{bail, Context, Result};
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
    /// Authentication profile
    auth_profile: Option<Profile>,
}

impl McpHttpTransport {
    /// Create a new HTTP transport connected to the given URL
    pub fn new(url: String) -> Result<Self> {
        Self::with_auth(url, None)
    }

    /// Create a new HTTP transport with authentication
    pub fn with_auth(url: String, auth_profile: Option<Profile>) -> Result<Self> {
        // Validate URL
        let parsed = url::Url::parse(&url).context("Invalid MCP server URL")?;

        // Ensure it's http or https
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            bail!(
                "MCP HTTP transport only supports http:// and https:// URLs, got: {}",
                parsed.scheme()
            );
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            server_url: url,
            next_id: Arc::new(Mutex::new(1i64)),
            auth_profile,
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

        tracing::debug!(
            "Sending MCP HTTP request: {} to {}",
            method,
            self.server_url
        );

        // Build request with authentication if profile is set
        let mut req = self
            .client
            .post(&self.server_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        if let Some(profile) = &self.auth_profile {
            req = crate::auth::apply_auth_to_request(req, &profile.auth_type, &profile.api_key);
        }

        let response = req
            .json(&request)
            .send()
            .await
            .context("Failed to send HTTP request to MCP server")?;

        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to read response body".to_string());

        // Check HTTP status
        if !status.is_success() {
            bail!(
                "MCP server returned HTTP error: {} - {}",
                status,
                body
            );
        }

        // Parse JSON or streamable HTTP (SSE) response
        let json_response =
            Self::parse_jsonrpc_response(content_type.as_deref(), &body)
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

    fn parse_jsonrpc_response(content_type: Option<&str>, body: &str) -> Result<JsonRpcResponse> {
        let content_type = content_type.unwrap_or_default().to_ascii_lowercase();

        if content_type.contains("text/event-stream") {
            return Self::parse_sse_response(body);
        }

        serde_json::from_str::<JsonRpcResponse>(body)
            .or_else(|_| Self::parse_sse_response(body))
            .context("Response is neither JSON-RPC JSON nor JSON-RPC SSE")
    }

    fn parse_sse_response(body: &str) -> Result<JsonRpcResponse> {
        for line in body.lines() {
            let trimmed = line.trim();
            if let Some(data) = trimmed.strip_prefix("data:") {
                let payload = data.trim();
                if payload.is_empty() || payload == "[DONE]" {
                    continue;
                }

                if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(payload) {
                    return Ok(response);
                }
            }
        }

        bail!("No JSON-RPC payload found in SSE response")
    }

    /// Lightweight MCP HTTP probe used for endpoint discovery.
    pub async fn probe_initialize(url: &str, auth_profile: Option<Profile>) -> Result<bool> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .context("Failed to create MCP probe HTTP client")?;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "initialize".to_string(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "uxc-probe",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
            id: RequestId::Number(1),
        };

        let mut req = client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        if let Some(profile) = &auth_profile {
            req = crate::auth::apply_auth_to_request(req, &profile.auth_type, &profile.api_key);
        }

        let response = match req.json(&request).send().await {
            Ok(response) => response,
            Err(_) => return Ok(false),
        };

        if !response.status().is_success() {
            return Ok(false);
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = response.text().await.unwrap_or_default();

        Ok(Self::parse_jsonrpc_response(content_type.as_deref(), &body).is_ok())
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

        let response: ToolsListResponse =
            serde_json::from_value(result).context("Failed to parse tools/list response")?;

        Ok(response.tools)
    }

    /// Call a tool
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<JsonValue>,
    ) -> Result<ToolCallResult> {
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

        let response: ResourcesListResponse =
            serde_json::from_value(result).context("Failed to parse resources/list response")?;

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

        let response: PromptsListResponse =
            serde_json::from_value(result).context("Failed to parse prompts/list response")?;

        Ok(response.prompts)
    }

    /// Get a prompt
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<JsonValue>,
    ) -> Result<GetPromptResult> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });

        let result = self.send_request("prompts/get", Some(params)).await?;

        serde_json::from_value(result).context("Failed to parse prompts/get result")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sse_jsonrpc_response() {
        let sse = r#"event: message
data: {"jsonrpc":"2.0","id":1,"result":{"tools":[]}}

"#;

        let response = McpHttpTransport::parse_jsonrpc_response(Some("text/event-stream"), sse)
            .unwrap();
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_some());
    }
}
