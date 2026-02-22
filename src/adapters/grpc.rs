//! gRPC adapter with reflection support

use super::{Adapter, ExecutionResult, Operation, ProtocolType};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

pub struct GrpcAdapter {
    client: reqwest::Client,
}

impl GrpcAdapter {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
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
        ProtocolType::GRpc
    }

    async fn can_handle(&self, url: &str) -> Result<bool> {
        // Attempt gRPC reflection detection
        // gRPC reflection runs on the same endpoint as the gRPC service
        // We try to connect to the reflection service

        // Parse URL to get host and port
        let base_url = url.trim_end_matches('/');

        // Try standard gRPC web detection (for gRPC-Web endpoints)
        // First check if there's a gRPC-Web HTTP endpoint
        // Many gRPC services expose a gRPC-Web HTTP endpoint
        let grpc_web_paths = [
            "/grpc.reflect.v1alpha.ServerReflection/ServerReflectionInfo",
            "/grpc.reflection.v1.ServerReflection/ServerReflectionInfo",
        ];

        for path in &grpc_web_paths {
            let full_url = format!("{}{}", base_url, path);
            if let Ok(resp) = self
                .client
                .post(&full_url)
                .timeout(std::time::Duration::from_secs(2))
                .send()
                .await
            {
                // If we get any response (even an error) that's not a connection error,
                // it might be a gRPC endpoint
                if resp.status().is_success() || resp.status().as_u16() < 500 {
                    return Ok(true);
                }
            }
        }

        // Try direct gRPC port detection
        // If the URL specifies a port that's commonly used for gRPC
        if let Some(port_str) = base_url.split(':').next_back() {
            if let Ok(port) = port_str.parse::<u16>() {
                // Common gRPC ports
                if port == 50051 || port == 50052 || port == 50053 || port == 9090 {
                    // Assume it's gRPC if using standard gRPC ports
                    return Ok(true);
                }
            }
        }

        // Try to detect gRPC-Web by checking common endpoints
        let grpc_endpoints = ["/grpc.health.v1.Health/Check", "/grpc.reflection.v1alpha"];

        for path in &grpc_endpoints {
            let full_url = format!("{}{}", base_url, path);
            if let Ok(resp) = self
                .client
                .post(&full_url)
                .timeout(std::time::Duration::from_secs(2))
                .send()
                .await
            {
                if resp.status().is_success() || resp.status().as_u16() < 500 {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    async fn fetch_schema(&self, _url: &str) -> Result<Value> {
        // TODO: Implement gRPC reflection
        Err(anyhow::anyhow!("gRPC reflection not yet implemented"))
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
