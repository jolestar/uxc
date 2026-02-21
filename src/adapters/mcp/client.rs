//! MCP stdio client implementation

use super::transport::McpStdioTransport;
use super::types::*;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;

/// MCP stdio client
pub struct McpStdioClient {
    transport: McpStdioTransport,
    server_capabilities: Option<ServerCapabilities>,
}

impl McpStdioClient {
    /// Create a new MCP stdio client by spawning a server process
    pub async fn connect(command: &str, args: &[String]) -> Result<Self> {
        let mut transport = McpStdioTransport::connect(command, args).await?;

        // Initialize the session
        let client_info = ClientInfo {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let init_result = transport.initialize(client_info).await?;
        tracing::info!(
            "Connected to MCP server: {} v{}",
            init_result
                .serverInfo
                .as_ref()
                .map(|s| s.name.as_str())
                .unwrap_or("unknown"),
            init_result
                .serverInfo
                .as_ref()
                .map(|s| s.version.as_str())
                .unwrap_or("unknown")
        );

        // Send initialized notification
        transport.initialized().await?;

        Ok(Self {
            transport,
            server_capabilities: Some(init_result.capabilities),
        })
    }

    /// Check if the server supports tools
    pub fn supports_tools(&self) -> bool {
        self.server_capabilities
            .as_ref()
            .and_then(|c| c.tools.as_ref())
            .is_some()
    }

    /// Check if the server supports resources
    pub fn supports_resources(&self) -> bool {
        self.server_capabilities
            .as_ref()
            .and_then(|c| c.resources.as_ref())
            .is_some()
    }

    /// Check if the server supports prompts
    pub fn supports_prompts(&self) -> bool {
        self.server_capabilities
            .as_ref()
            .and_then(|c| c.prompts.as_ref())
            .is_some()
    }

    /// List available tools
    pub async fn list_tools(&mut self) -> Result<Vec<Tool>> {
        if !self.supports_tools() {
            bail!("Server does not support tools");
        }

        let result = self
            .transport
            .send_request("tools/list", None)
            .await
            .context("Failed to list tools")?;

        // Parse the response - tools are in result.tools
        let tools_value = result
            .get("tools")
            .context("Response missing 'tools' field")?
            .as_array()
            .context("'tools' is not an array")?;

        let mut tools = Vec::new();
        for tool_value in tools_value {
            let tool: Tool =
                serde_json::from_value(tool_value.clone()).context("Failed to parse tool")?;
            tools.push(tool);
        }

        Ok(tools)
    }

    /// Call a tool
    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: Option<JsonValue>,
    ) -> Result<CallToolResult> {
        if !self.supports_tools() {
            bail!("Server does not support tools");
        }

        let params = CallToolParams {
            name: name.to_string(),
            arguments,
        };

        let result = self
            .transport
            .send_request("tools/call", Some(serde_json::to_value(params)?))
            .await
            .context(format!("Failed to call tool '{}'", name))?;

        let call_result: CallToolResult =
            serde_json::from_value(result).context("Failed to parse tool call result")?;

        Ok(call_result)
    }

    /// List available resources
    pub async fn list_resources(&mut self) -> Result<Vec<Resource>> {
        if !self.supports_resources() {
            bail!("Server does not support resources");
        }

        let result = self
            .transport
            .send_request("resources/list", None)
            .await
            .context("Failed to list resources")?;

        let resources_value = result
            .get("resources")
            .context("Response missing 'resources' field")?
            .as_array()
            .context("'resources' is not an array")?;

        let mut resources = Vec::new();
        for resource_value in resources_value {
            let resource: Resource = serde_json::from_value(resource_value.clone())
                .context("Failed to parse resource")?;
            resources.push(resource);
        }

        Ok(resources)
    }

    /// Read a resource
    pub async fn read_resource(&mut self, uri: &str) -> Result<ResourceContents> {
        if !self.supports_resources() {
            bail!("Server does not support resources");
        }

        let params = json!({ "uri": uri });

        let result = self
            .transport
            .send_request("resources/read", Some(params))
            .await
            .context(format!("Failed to read resource '{}'", uri))?;

        let contents: ResourceContents =
            serde_json::from_value(result).context("Failed to parse resource contents")?;

        Ok(contents)
    }

    /// List available prompts
    pub async fn list_prompts(&mut self) -> Result<Vec<Prompt>> {
        if !self.supports_prompts() {
            bail!("Server does not support prompts");
        }

        let result = self
            .transport
            .send_request("prompts/list", None)
            .await
            .context("Failed to list prompts")?;

        let prompts_value = result
            .get("prompts")
            .context("Response missing 'prompts' field")?
            .as_array()
            .context("'prompts' is not an array")?;

        let mut prompts = Vec::new();
        for prompt_value in prompts_value {
            let prompt: Prompt =
                serde_json::from_value(prompt_value.clone()).context("Failed to parse prompt")?;
            prompts.push(prompt);
        }

        Ok(prompts)
    }

    /// Get a prompt
    pub async fn get_prompt(
        &mut self,
        name: &str,
        arguments: Option<HashMap<String, String>>,
    ) -> Result<GetPromptResult> {
        if !self.supports_prompts() {
            bail!("Server does not support prompts");
        }

        let params = json!({
            "name": name,
            "arguments": arguments
        });

        let result = self
            .transport
            .send_request("prompts/get", Some(params))
            .await
            .context(format!("Failed to get prompt '{}'", name))?;

        let prompt_result: GetPromptResult =
            serde_json::from_value(result).context("Failed to parse prompt result")?;

        Ok(prompt_result)
    }
}
