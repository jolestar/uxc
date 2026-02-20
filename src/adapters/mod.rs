//! Protocol adapters for different schema types
//!
//! Each adapter implements a common interface for:
//! - Protocol detection
//! - Schema retrieval
//! - Operation discovery
//! - Execution

pub mod openapi;
pub mod grpc;
pub mod mcp;
pub mod graphql;

use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;

/// Enum of all available adapters
pub enum AdapterEnum {
    OpenAPI(openapi::OpenAPIAdapter),
    gRPC(grpc::GrpcAdapter),
    MCP(mcp::McpAdapter),
    GraphQL(graphql::GraphQLAdapter),
}

#[async_trait]
impl Adapter for AdapterEnum {
    fn protocol_type(&self) -> ProtocolType {
        match self {
            AdapterEnum::OpenAPI(_) => ProtocolType::OpenAPI,
            AdapterEnum::gRPC(_) => ProtocolType::gRPC,
            AdapterEnum::MCP(_) => ProtocolType::MCP,
            AdapterEnum::GraphQL(_) => ProtocolType::GraphQL,
        }
    }

    async fn can_handle(&self, url: &str) -> Result<bool> {
        match self {
            AdapterEnum::OpenAPI(a) => a.can_handle(url).await,
            AdapterEnum::gRPC(a) => a.can_handle(url).await,
            AdapterEnum::MCP(a) => a.can_handle(url).await,
            AdapterEnum::GraphQL(a) => a.can_handle(url).await,
        }
    }

    async fn fetch_schema(&self, url: &str) -> Result<Value> {
        match self {
            AdapterEnum::OpenAPI(a) => a.fetch_schema(url).await,
            AdapterEnum::gRPC(a) => a.fetch_schema(url).await,
            AdapterEnum::MCP(a) => a.fetch_schema(url).await,
            AdapterEnum::GraphQL(a) => a.fetch_schema(url).await,
        }
    }

    async fn list_operations(&self, url: &str) -> Result<Vec<Operation>> {
        match self {
            AdapterEnum::OpenAPI(a) => a.list_operations(url).await,
            AdapterEnum::gRPC(a) => a.list_operations(url).await,
            AdapterEnum::MCP(a) => a.list_operations(url).await,
            AdapterEnum::GraphQL(a) => a.list_operations(url).await,
        }
    }

    async fn operation_help(&self, url: &str, operation: &str) -> Result<String> {
        match self {
            AdapterEnum::OpenAPI(a) => a.operation_help(url, operation).await,
            AdapterEnum::gRPC(a) => a.operation_help(url, operation).await,
            AdapterEnum::MCP(a) => a.operation_help(url, operation).await,
            AdapterEnum::GraphQL(a) => a.operation_help(url, operation).await,
        }
    }

    async fn execute(
        &self,
        url: &str,
        operation: &str,
        args: HashMap<String, Value>,
    ) -> Result<ExecutionResult> {
        match self {
            AdapterEnum::OpenAPI(a) => a.execute(url, operation, args).await,
            AdapterEnum::gRPC(a) => a.execute(url, operation, args).await,
            AdapterEnum::MCP(a) => a.execute(url, operation, args).await,
            AdapterEnum::GraphQL(a) => a.execute(url, operation, args).await,
        }
    }
}

/// Supported protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    OpenAPI,
    gRPC,
    MCP,
    GraphQL,
}

impl ProtocolType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtocolType::OpenAPI => "openapi",
            ProtocolType::gRPC => "grpc",
            ProtocolType::MCP => "mcp",
            ProtocolType::GraphQL => "graphql",
        }
    }
}

/// Operation metadata
#[derive(Debug, Clone)]
pub struct Operation {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
}

/// Parameter definition
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: Option<String>,
}

/// Execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub data: Value,
    pub metadata: ExecutionMetadata,
}

#[derive(Debug, Clone)]
pub struct ExecutionMetadata {
    pub duration_ms: u64,
    pub operation: String,
}

/// Adapter trait - must be implemented by all protocol adapters
#[async_trait::async_trait]
pub trait Adapter: Send + Sync {
    /// Get the protocol type this adapter handles
    fn protocol_type(&self) -> ProtocolType;

    /// Detect if this adapter can handle the given endpoint
    async fn can_handle(&self, url: &str) -> Result<bool>;

    /// Retrieve schema from the endpoint
    async fn fetch_schema(&self, url: &str) -> Result<Value>;

    /// List available operations
    async fn list_operations(&self, url: &str) -> Result<Vec<Operation>>;

    /// Get help for a specific operation
    async fn operation_help(&self, url: &str, operation: &str) -> Result<String>;

    /// Execute an operation
    async fn execute(
        &self,
        url: &str,
        operation: &str,
        args: HashMap<String, Value>,
    ) -> Result<ExecutionResult>;
}

/// Protocol detector - attempts to identify the protocol type with parallel detection
pub struct ProtocolDetector {
    /// Timeout for each protocol detection attempt
    timeout: Duration,
}

impl ProtocolDetector {
    pub fn new() -> Self {
        Self {
            // Default timeout per protocol check: 500ms
            // Total max detection time for 4 protocols = ~2 seconds
            timeout: Duration::from_millis(500),
        }
    }

    /// Create a detector with custom timeout
    pub fn with_timeout(timeout_ms: u64) -> Self {
        Self {
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    /// Get adapter for a URL (auto-detects protocol)
    pub async fn detect_adapter(&self, url: &str) -> Result<AdapterEnum> {
        // Try all protocols in parallel for fast detection
        // Use tokio::spawn for parallel execution with timeout

        let openapi_task = tokio::spawn({
            let url = url.to_string();
            async move {
                let adapter = openapi::OpenAPIAdapter::new();
                let can_handle = tokio::time::timeout(
                    Duration::from_millis(500),
                    adapter.can_handle(&url)
                ).await;
                match can_handle {
                    Ok(Ok(true)) => Some(AdapterEnum::OpenAPI(adapter)),
                    _ => None,
                }
            }
        });

        let grpc_task = tokio::spawn({
            let url = url.to_string();
            async move {
                let adapter = grpc::GrpcAdapter::new();
                let can_handle = tokio::time::timeout(
                    Duration::from_millis(500),
                    adapter.can_handle(&url)
                ).await;
                match can_handle {
                    Ok(Ok(true)) => Some(AdapterEnum::gRPC(adapter)),
                    _ => None,
                }
            }
        });

        let mcp_task = tokio::spawn({
            let url = url.to_string();
            async move {
                let adapter = mcp::McpAdapter::new();
                let can_handle = tokio::time::timeout(
                    Duration::from_millis(500),
                    adapter.can_handle(&url)
                ).await;
                match can_handle {
                    Ok(Ok(true)) => Some(AdapterEnum::MCP(adapter)),
                    _ => None,
                }
            }
        });

        let graphql_task = tokio::spawn({
            let url = url.to_string();
            async move {
                let adapter = graphql::GraphQLAdapter::new();
                let can_handle = tokio::time::timeout(
                    Duration::from_millis(500),
                    adapter.can_handle(&url)
                ).await;
                match can_handle {
                    Ok(Ok(true)) => Some(AdapterEnum::GraphQL(adapter)),
                    _ => None,
                }
            }
        });

        // Yield to let tasks start
        tokio::task::yield_now().await;

        // Check results in priority order
        // OpenAPI first (most common), then GraphQL, MCP, gRPC
        if let Ok(Some(adapter)) = openapi_task.await {
            return Ok(adapter);
        }

        if let Ok(Some(adapter)) = graphql_task.await {
            return Ok(adapter);
        }

        if let Ok(Some(adapter)) = mcp_task.await {
            return Ok(adapter);
        }

        if let Ok(Some(adapter)) = grpc_task.await {
            return Ok(adapter);
        }

        // If parallel detection didn't work, fall back to sequential with timeout
        // This is more reliable but slower
        let openapi_adapter = openapi::OpenAPIAdapter::new();
        if tokio::time::timeout(self.timeout, openapi_adapter.can_handle(&url)).await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or(false) {
            return Ok(AdapterEnum::OpenAPI(openapi_adapter));
        }

        let graphql_adapter = graphql::GraphQLAdapter::new();
        if tokio::time::timeout(self.timeout, graphql_adapter.can_handle(&url)).await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or(false) {
            return Ok(AdapterEnum::GraphQL(graphql_adapter));
        }

        let mcp_adapter = mcp::McpAdapter::new();
        if tokio::time::timeout(self.timeout, mcp_adapter.can_handle(&url)).await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or(false) {
            return Ok(AdapterEnum::MCP(mcp_adapter));
        }

        let grpc_adapter = grpc::GrpcAdapter::new();
        if tokio::time::timeout(self.timeout, grpc_adapter.can_handle(&url)).await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or(false) {
            return Ok(AdapterEnum::gRPC(grpc_adapter));
        }

        Err(anyhow::anyhow!(
            "Unable to detect protocol for URL: {}. \
            Tried: OpenAPI, GraphQL, MCP, gRPC. \
            Ensure the endpoint exposes one of these protocols with discovery enabled.",
            url
        ))
    }

    /// Detect protocol type without returning adapter
    pub async fn detect_protocol_type(&self, url: &str) -> Result<ProtocolType> {
        let adapter = self.detect_adapter(url).await?;
        Ok(adapter.protocol_type())
    }
}

impl Default for ProtocolDetector {
    fn default() -> Self {
        Self::new()
    }
}
