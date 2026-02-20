//! MCP (Model Context Protocol) adapter

use super::{Adapter, ProtocolType, Operation, Parameter, ExecutionResult, ExecutionMetadata};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;

pub struct McpAdapter {
    client: reqwest::Client,
}

impl McpAdapter {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for McpAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Adapter for McpAdapter {
    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::MCP
    }

    async fn can_handle(&self, url: &str) -> Result<bool> {
        // MCP detection strategy:
        // 1. Try MCP over HTTP: check for /mcp or MCP-specific endpoints
        // 2. Send MCP initialize request and check response
        // 3. Look for MCP-specific headers or response patterns

        // Create client with short timeout for fast detection
        let timeout_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .build()?;

        // Try common MCP endpoints
        let endpoints = [
            "/mcp",
            "/mcp/v1",
            "/api/mcp",
            "/.well-known/mcp",
        ];

        for endpoint in endpoints {
            let full_url = format!("{}{}", url.trim_end_matches('/'), endpoint);

            // Try to send an MCP initialize request
            let init_request = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "uxc",
                        "version": "0.1.0"
                    }
                }
            });

            match timeout_client.post(&full_url).json(&init_request).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        // Try to parse as JSON-RPC response
                        if let Ok(text) = resp.text().await {
                            if let Ok(json) = serde_json::from_str::<Value>(&text) {
                                // Check for valid JSON-RPC response with result
                                if json.get("jsonrpc").and_then(|v| v.as_str()) == Some("2.0") {
                                    if json.get("result").is_some() {
                                        return Ok(true);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        // Also try to detect MCP by checking the root URL with initialize request
        let init_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "uxc",
                    "version": "0.1.0"
                }
            }
        });

        match timeout_client.post(url).json(&init_request).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    if let Ok(text) = resp.text().await {
                        if let Ok(json) = serde_json::from_str::<Value>(&text) {
                            if json.get("jsonrpc").and_then(|v| v.as_str()) == Some("2.0") {
                                if json.get("result").is_some() {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {}
        }

        Ok(false)
    }

    async fn fetch_schema(&self, url: &str) -> Result<Value> {
        // Send MCP initialize request
        let init_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "uxc",
                    "version": "0.1.0"
                }
            }
        });

        let resp = self.client.post(url).json(&init_request).send().await?;
        let data: Value = resp.json().await?;

        // Get tools list
        let tools_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let tools_resp = self.client.post(url).json(&tools_request).send().await?;
        let tools_data: Value = tools_resp.json().await?;

        Ok(serde_json::json!({
            "initialize": data,
            "tools": tools_data
        }))
    }

    async fn list_operations(&self, url: &str) -> Result<Vec<Operation>> {
        // Initialize first
        let init_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "uxc",
                    "version": "0.1.0"
                }
            }
        });
        let _ = self.client.post(url).json(&init_request).send().await;

        // Get tools list
        let tools_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let resp = self.client.post(url).json(&tools_request).send().await?;
        let data: Value = resp.json().await?;

        let mut operations = Vec::new();

        if let Some(result) = data.get("result") {
            if let Some(tools) = result.get("tools").and_then(|t| t.as_array()) {
                for tool in tools {
                    let name = tool.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
                    let description = tool.get("description").and_then(|d| d.as_str()).map(|s| s.to_string());

                    let mut parameters = Vec::new();
                    if let Some(input_schema) = tool.get("inputSchema") {
                        if let Some(props) = input_schema.get("properties").and_then(|p| p.as_object()) {
                            for (param_name, param_info) in props {
                                let param_type = param_info.get("type")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("string")
                                    .to_string();

                                let required_list = input_schema.get("required")
                                    .and_then(|r| r.as_array())
                                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>());

                                let required = required_list
                                    .as_ref()
                                    .map(|list| list.iter().any(|s| *s == param_name.as_str()))
                                    .unwrap_or(false);

                                parameters.push(Parameter {
                                    name: param_name.to_string(),
                                    param_type,
                                    required,
                                    description: param_info.get("description")
                                        .and_then(|d| d.as_str())
                                        .map(|s| s.to_string()),
                                });
                            }
                        }
                    }

                    operations.push(Operation {
                        name: name.to_string(),
                        description,
                        parameters,
                        return_type: None,
                    });
                }
            }
        }

        Ok(operations)
    }

    async fn operation_help(&self, url: &str, operation: &str) -> Result<String> {
        let operations = self.list_operations(url).await?;
        let op = operations
            .iter()
            .find(|o| o.name == operation)
            .ok_or_else(|| anyhow::anyhow!("Operation not found: {}", operation))?;

        let mut help = format!("## {}\n", op.name);
        if let Some(desc) = &op.description {
            help.push_str(&format!("{}\n\n", desc));
        }

        if !op.parameters.is_empty() {
            help.push_str("### Parameters\n\n");
            for param in &op.parameters {
                help.push_str(&format!(
                    "- `{}` ({}){}\n",
                    param.name,
                    param.param_type,
                    if param.required { " **required**" } else { "" }
                ));
                if let Some(desc) = &param.description {
                    help.push_str(&format!("  - {}\n", desc));
                }
            }
        }

        Ok(help)
    }

    async fn execute(
        &self,
        url: &str,
        operation: &str,
        args: HashMap<String, Value>,
    ) -> Result<ExecutionResult> {
        let start = std::time::Instant::now();

        // Send MCP tools/call request
        let call_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": operation,
                "arguments": args
            }
        });

        let resp = self.client.post(url).json(&call_request).send().await?;
        let data: Value = resp.json().await?;

        Ok(ExecutionResult {
            data,
            metadata: ExecutionMetadata {
                duration_ms: start.elapsed().as_millis() as u64,
                operation: operation.to_string(),
            },
        })
    }
}
