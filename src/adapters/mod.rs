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

/// Protocol detector - attempts to identify the protocol type
pub struct ProtocolDetector;

impl ProtocolDetector {
    pub fn new() -> Self {
        Self
    }

    /// Get adapter for a URL (auto-detects protocol)
    pub async fn detect_adapter(&self, url: &str) -> Result<AdapterEnum> {
        // Try OpenAPI first
        let openapi_adapter = openapi::OpenAPIAdapter::new();
        if openapi_adapter.can_handle(url).await? {
            return Ok(AdapterEnum::OpenAPI(openapi_adapter));
        }

        // Try gRPC
        let grpc_adapter = grpc::GrpcAdapter::new();
        if grpc_adapter.can_handle(url).await? {
            return Ok(AdapterEnum::gRPC(grpc_adapter));
        }

        // Try MCP
        let mcp_adapter = mcp::McpAdapter::new();
        if mcp_adapter.can_handle(url).await? {
            return Ok(AdapterEnum::MCP(mcp_adapter));
        }

        // Try GraphQL
        let graphql_adapter = graphql::GraphQLAdapter::new();
        if graphql_adapter.can_handle(url).await? {
            return Ok(AdapterEnum::GraphQL(graphql_adapter));
        }

        Err(anyhow::anyhow!("No adapter found for URL: {}", url))
    }
}

impl Default for ProtocolDetector {
    fn default() -> Self {
        Self::new()
    }
}
