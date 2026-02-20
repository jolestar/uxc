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
use std::time::Duration;
use anyhow::Result;

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
pub struct ProtocolDetector {
    adapters: Vec<Box<dyn Adapter>>,
}

impl ProtocolDetector {
    pub fn new() -> Self {
        Self {
            adapters: vec![
                Box::new(openapi::OpenAPIAdapter::new()),
                Box::new(grpc::GrpcAdapter::new()),
                Box::new(mcp::McpAdapter::new()),
                Box::new(graphql::GraphQLAdapter::new()),
            ],
        }
    }

    /// Detect protocol type for a given URL using parallel detection
    /// Returns the first protocol that successfully handles the URL
    pub async fn detect(&self, url: &str) -> Result<Option<ProtocolType>> {
        // Try all adapters in parallel for fast detection
        // Use a 2-second timeout for the entire detection process
        let detect_futures: Vec<_> = self.adapters.iter().map(|adapter| {
            async move {
                let result = adapter.can_handle(url).await;
                match result {
                    Ok(true) => Some(adapter.protocol_type()),
                    _ => None,
                }
            }
        }).collect();

        // Run all detection checks in parallel with a timeout
        let results = tokio::time::timeout(
            Duration::from_secs(2),
            futures::future::join_all(detect_futures)
        ).await;

        match results {
            Ok(detected) => {
                // Return the first successfully detected protocol
                for protocol in detected {
                    if protocol.is_some() {
                        return Ok(protocol);
                    }
                }
                Ok(None)
            }
            Err(_) => {
                // Timeout - return error for unknown protocol
                Ok(None)
            }
        }
    }

    /// Get adapter for a specific protocol type
    pub fn get_adapter(&self, protocol: ProtocolType) -> Option<&dyn Adapter> {
        self.adapters
            .iter()
            .find(|a| a.protocol_type() == protocol)
            .map(|a| a.as_ref())
    }
}

impl Default for ProtocolDetector {
    fn default() -> Self {
        Self::new()
    }
}
