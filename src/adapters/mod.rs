//! Protocol adapters for different schema types
//!
//! Each adapter implements a common interface for:
//! - Protocol detection
//! - Schema retrieval
//! - Operation discovery
//! - Execution

pub mod graphql;
pub mod grpc;
pub mod mcp;
pub mod openapi;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Enum of all available adapters
#[allow(non_camel_case_types)]
pub enum AdapterEnum {
    OpenAPI(openapi::OpenAPIAdapter),
    GRpc(grpc::GrpcAdapter),
    Mcp(mcp::McpAdapter),
    GraphQL(graphql::GraphQLAdapter),
}

#[async_trait]
impl Adapter for AdapterEnum {
    fn protocol_type(&self) -> ProtocolType {
        match self {
            AdapterEnum::OpenAPI(_) => ProtocolType::OpenAPI,
            AdapterEnum::GRpc(_) => ProtocolType::GRpc,
            AdapterEnum::Mcp(_) => ProtocolType::Mcp,
            AdapterEnum::GraphQL(_) => ProtocolType::GraphQL,
        }
    }

    async fn can_handle(&self, url: &str) -> Result<bool> {
        match self {
            AdapterEnum::OpenAPI(a) => a.can_handle(url).await,
            AdapterEnum::GRpc(a) => a.can_handle(url).await,
            AdapterEnum::Mcp(a) => a.can_handle(url).await,
            AdapterEnum::GraphQL(a) => a.can_handle(url).await,
        }
    }

    async fn fetch_schema(&self, url: &str) -> Result<Value> {
        match self {
            AdapterEnum::OpenAPI(a) => a.fetch_schema(url).await,
            AdapterEnum::GRpc(a) => a.fetch_schema(url).await,
            AdapterEnum::Mcp(a) => a.fetch_schema(url).await,
            AdapterEnum::GraphQL(a) => a.fetch_schema(url).await,
        }
    }

    async fn list_operations(&self, url: &str) -> Result<Vec<Operation>> {
        match self {
            AdapterEnum::OpenAPI(a) => a.list_operations(url).await,
            AdapterEnum::GRpc(a) => a.list_operations(url).await,
            AdapterEnum::Mcp(a) => a.list_operations(url).await,
            AdapterEnum::GraphQL(a) => a.list_operations(url).await,
        }
    }

    async fn operation_help(&self, url: &str, operation: &str) -> Result<String> {
        match self {
            AdapterEnum::OpenAPI(a) => a.operation_help(url, operation).await,
            AdapterEnum::GRpc(a) => a.operation_help(url, operation).await,
            AdapterEnum::Mcp(a) => a.operation_help(url, operation).await,
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
            AdapterEnum::GRpc(a) => a.execute(url, operation, args).await,
            AdapterEnum::Mcp(a) => a.execute(url, operation, args).await,
            AdapterEnum::GraphQL(a) => a.execute(url, operation, args).await,
        }
    }
}

/// Supported protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum ProtocolType {
    OpenAPI,
    GRpc,
    Mcp,
    GraphQL,
}

impl ProtocolType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtocolType::OpenAPI => "openapi",
            ProtocolType::GRpc => "grpc",
            ProtocolType::Mcp => "mcp",
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
        // Try MCP first (stdio commands are distinct)
        let mcp_adapter = mcp::McpAdapter::new();
        if mcp_adapter.can_handle(url).await? {
            return Ok(AdapterEnum::Mcp(mcp_adapter));
        }

        // Try GraphQL (introspection is reliable)
        let graphql_adapter = graphql::GraphQLAdapter::new();
        if graphql_adapter.can_handle(url).await? {
            return Ok(AdapterEnum::GraphQL(graphql_adapter));
        }

        // Try OpenAPI
        let openapi_adapter = openapi::OpenAPIAdapter::new();
        if openapi_adapter.can_handle(url).await? {
            return Ok(AdapterEnum::OpenAPI(openapi_adapter));
        }

        // Try gRPC (less reliable detection, try last)
        let grpc_adapter = grpc::GrpcAdapter::new();
        if grpc_adapter.can_handle(url).await? {
            return Ok(AdapterEnum::GRpc(grpc_adapter));
        }

        Err(anyhow::anyhow!("No adapter found for URL: {}", url))
    }
}

impl Default for ProtocolDetector {
    fn default() -> Self {
        Self::new()
    }
}
