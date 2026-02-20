//! gRPC adapter with reflection support

use super::{Adapter, ProtocolType, Operation, ExecutionResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;

pub struct GrpcAdapter {
    _client: Option<reqwest::Client>,
}

impl GrpcAdapter {
    pub fn new() -> Self {
        Self {
            _client: Some(reqwest::Client::new()),
        }
    }
}

impl Default for GrpcAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Adapter for GrpcAdapter {
    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::gRPC
    }

    async fn can_handle(&self, url: &str) -> Result<bool> {
        // gRPC detection strategy:
        // 1. Check if URL uses gRPC scheme (grpc://)
        // 2. Try to connect to common gRPC port (50051) with HTTP/2
        // 3. Attempt gRPC server reflection if available

        // Check for gRPC scheme
        if url.starts_with("grpc://") || url.starts_with("grpcs://") {
            return Ok(true);
        }

        // For https:// URLs, try to detect if it might be a gRPC-Web endpoint
        // by checking for common gRPC indicators
        if url.starts_with("http://") || url.starts_with("https://") {
            // Try to detect gRPC-Web by checking if endpoint accepts gRPC content
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(500))
                .build()?;

            // Try to make a simple request to see if it responds like gRPC
            // gRPC servers typically reject HTTP/1.1 or require HTTP/2
            match client.get(url).send().await {
                Ok(resp) => {
                    // Check HTTP version - gRPC uses HTTP/2
                    if resp.version() == reqwest::Version::HTTP_2 {
                        return Ok(true);
                    }
                    // Check for gRPC-specific headers
                    if resp.headers().get("grpc-status").is_some() {
                        return Ok(true);
                    }
                }
                Err(e) => {
                    // If we get a protocol error, it might be gRPC
                    if e.to_string().contains("HTTP/2") || e.to_string().contains("protocol") {
                        return Ok(true);
                    }
                }
            }
        }

        // Check for common gRPC port
        if let Ok(parsed) = url::Url::parse(url) {
            if parsed.port() == Some(50051) || parsed.port() == Some(50052) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn fetch_schema(&self, _url: &str) -> Result<Value> {
        // TODO: Implement gRPC reflection
        // This would involve:
        // 1. Connect to the gRPC server
        // 2. Call ServerReflectionInfo
        // 3. Retrieve service descriptors
        // 4. Convert to JSON schema
        Err(anyhow::anyhow!("gRPC reflection not yet implemented - requires tonic reflection support"))
    }

    async fn list_operations(&self, _url: &str) -> Result<Vec<Operation>> {
        // TODO: Implement service discovery via reflection
        Err(anyhow::anyhow!("gRPC reflection not yet implemented"))
    }

    async fn operation_help(&self, _url: &str, _operation: &str) -> Result<String> {
        // TODO: Implement operation help
        Err(anyhow::anyhow!("gRPC reflection not yet implemented"))
    }

    async fn execute(
        &self,
        _url: &str,
        _operation: &str,
        _args: HashMap<String, Value>,
    ) -> Result<ExecutionResult> {
        // TODO: Implement gRPC execution
        Err(anyhow::anyhow!("gRPC execution not yet implemented"))
    }
}
