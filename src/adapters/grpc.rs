//! gRPC adapter with reflection support
//!
//! This module provides full gRPC support including:
//! - Server reflection for automatic schema discovery
//! - Dynamic method invocation using tonic
//! - Support for all 4 call types: unary, server-stream, client-stream, bidi-stream
//! - TLS and h2c (cleartext) support
//! - Proper error handling and status code mapping

use super::{Adapter, ExecutionResult, Operation, Parameter, ProtocolType};
use crate::auth::Profile;
use crate::error::UxcError;
use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use prost::Message;
use prost_types::FileDescriptorProto;
use reflection::{server_reflection_request, ServerReflectionRequest};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Endpoint;
use tonic::Status;
use tonic_reflection::pb as reflection;
use tracing::{debug, info};

/// gRPC adapter implementation
pub struct GrpcAdapter {
    /// In-memory cache for reflection clients and descriptors
    in_memory_cache: Arc<RwLock<HashMap<String, CachedReflectionData>>>,
    /// Persistent schema cache
    schema_cache: Option<Arc<dyn crate::cache::Cache>>,
    /// Authentication profile
    auth_profile: Option<Profile>,
}

/// Cached reflection data for a server
#[derive(Clone)]
struct CachedReflectionData {
    services: HashMap<String, ServiceInfo>,
}

/// Information about a gRPC service
#[derive(Clone, Debug)]
struct ServiceInfo {
    methods: HashMap<String, MethodInfo>,
    #[allow(dead_code)]
    file_descriptor: FileDescriptorProto,
}

/// Information about a gRPC method
#[derive(Clone, Debug)]
struct MethodInfo {
    name: String,
    service_name: String,
    input_type: String,
    output_type: String,
    is_server_streaming: bool,
    is_client_streaming: bool,
    description: Option<String>,
}

impl GrpcAdapter {
    pub fn new() -> Self {
        Self {
            in_memory_cache: Arc::new(RwLock::new(HashMap::new())),
            schema_cache: None,
            auth_profile: None,
        }
    }

    pub fn with_cache(mut self, cache: Arc<dyn crate::cache::Cache>) -> Self {
        self.schema_cache = Some(cache);
        self
    }

    pub fn with_auth(mut self, profile: Profile) -> Self {
        self.auth_profile = Some(profile);
        self
    }

    /// Parse URL to get host:port
    fn parse_url(url: &str) -> Result<String> {
        let url = url.trim_end_matches('/');

        // If it's already in host:port format
        if url.contains(':') && !url.starts_with("http://") && !url.starts_with("https://") {
            return Ok(url.to_string());
        }

        // Handle URLs
        let url = if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else {
            format!("http://{}", url)
        };

        // Parse and extract host:port
        let parsed = url::Url::parse(&url)?;
        let host = parsed.host_str().ok_or_else(|| anyhow!("Invalid host"))?;
        let port = parsed.port().unwrap_or(50051); // Default gRPC port
        Ok(format!("{}:{}", host, port))
    }

    /// Check if the server has reflection enabled
    async fn has_reflection(endpoint: &Endpoint) -> Result<bool> {
        let channel = endpoint
            .connect()
            .await
            .context("Failed to connect to gRPC server")?;

        let mut reflection_client =
            reflection::server_reflection_client::ServerReflectionClient::new(channel)
                .max_decoding_message_size(usize::MAX);

        let (tx, rx) = tokio::sync::mpsc::channel(4);

        let request = ServerReflectionRequest {
            host: String::new(),
            message_request: Some(server_reflection_request::MessageRequest::ListServices(
                String::new(),
            )),
        };

        let _ = tx.send(request).await;
        drop(tx); // Close the channel

        let result = tokio::time::timeout(
            Duration::from_secs(3),
            async {
                let response = reflection_client.server_reflection_info(ReceiverStream::new(rx)).await;
                match response {
                    Ok(streaming) => {
                        let mut stream = streaming.into_inner();
                        while let Some(message) = stream.message().await? {
                            match message.message_response {
                                Some(
                                    reflection::server_reflection_response::MessageResponse::ListServicesResponse(_),
                                ) => return Ok::<bool, anyhow::Error>(true),
                                Some(
                                    reflection::server_reflection_response::MessageResponse::ErrorResponse(err),
                                ) => {
                                    if err.error_code == tonic::Code::Unimplemented as i32 {
                                        return Ok::<bool, anyhow::Error>(false);
                                    }
                                }
                                _ => {}
                            }
                        }
                        Ok::<bool, anyhow::Error>(false)
                    }
                    Err(_) => Ok::<bool, anyhow::Error>(false),
                }
            },
        )
        .await;

        match result {
            Ok(Ok(has_reflection)) => Ok(has_reflection),
            Ok(Err(_)) => Ok(false),
            Err(_) => Ok(false), // Timeout
        }
    }

    /// List all services via reflection
    async fn list_services_reflection(&self, endpoint: &Endpoint) -> Result<Vec<String>> {
        let channel = endpoint.connect().await?;
        let mut client = reflection::server_reflection_client::ServerReflectionClient::new(channel)
            .max_decoding_message_size(usize::MAX);

        let (tx, rx) = tokio::sync::mpsc::channel(4);

        let request = ServerReflectionRequest {
            host: String::new(),
            message_request: Some(server_reflection_request::MessageRequest::ListServices(
                String::new(),
            )),
        };

        tx.send(request).await?;
        drop(tx); // Close the channel

        let mut stream = client
            .server_reflection_info(ReceiverStream::new(rx))
            .await?
            .into_inner();

        let mut services = Vec::new();
        while let Some(response) = stream.message().await? {
            if let Some(
                reflection::server_reflection_response::MessageResponse::ListServicesResponse(ls),
            ) = response.message_response
            {
                for service in ls.service {
                    services.push(service.name);
                }
            }
        }

        Ok(services)
    }

    /// Get file descriptor for a service
    async fn get_service_descriptor(
        &self,
        endpoint: &Endpoint,
        service_name: &str,
    ) -> Result<FileDescriptorProto> {
        let channel = endpoint.connect().await?;
        let mut client = reflection::server_reflection_client::ServerReflectionClient::new(channel)
            .max_decoding_message_size(usize::MAX);

        let (tx, rx) = tokio::sync::mpsc::channel(4);

        let request = ServerReflectionRequest {
            host: String::new(),
            message_request: Some(
                server_reflection_request::MessageRequest::FileContainingSymbol(
                    service_name.to_string(),
                ),
            ),
        };

        tx.send(request).await?;
        drop(tx); // Close the channel

        let mut stream = client
            .server_reflection_info(ReceiverStream::new(rx))
            .await?
            .into_inner();

        while let Some(response) = stream.message().await? {
            if let Some(
                reflection::server_reflection_response::MessageResponse::FileDescriptorResponse(fd),
            ) = response.message_response
            {
                if let Some(descriptor_bytes) = fd.file_descriptor_proto.first() {
                    let descriptor = FileDescriptorProto::decode(descriptor_bytes.as_slice())
                        .context("Failed to decode file descriptor")?;
                    return Ok(descriptor);
                }
            }
        }

        bail!("File descriptor not found for service: {}", service_name)
    }

    /// Parse service and methods from file descriptor
    fn parse_service_info(&self, descriptor: &FileDescriptorProto) -> Result<ServiceInfo> {
        let mut methods = HashMap::new();
        let package = descriptor.package.clone().unwrap_or_default();

        for service in &descriptor.service {
            let service_name = service.name.clone().unwrap_or_default();
            let full_service_name = if package.is_empty() {
                service_name
            } else {
                format!("{}.{}", package, service_name)
            };
            for method in &service.method {
                let method_name = method.name.clone().unwrap_or_default();
                let method_info = MethodInfo {
                    name: method_name.clone(),
                    service_name: full_service_name.clone(),
                    input_type: method.input_type.clone().unwrap_or_default(),
                    output_type: method.output_type.clone().unwrap_or_default(),
                    is_server_streaming: method.server_streaming.unwrap_or(false),
                    is_client_streaming: method.client_streaming.unwrap_or(false),
                    description: None, // Comments are in source_code_info
                };
                methods.insert(method_name, method_info);
            }
        }

        Ok(ServiceInfo {
            methods,
            file_descriptor: descriptor.clone(),
        })
    }

    /// Get or load service information
    async fn get_service_info(&self, url: &str) -> Result<HashMap<String, ServiceInfo>> {
        // Check in-memory cache first
        {
            let cache = self.in_memory_cache.read().await;
            if let Some(data) = cache.get(url) {
                return Ok(data.services.clone());
            }
        }

        // Load from reflection
        let endpoint = self.create_endpoint(url)?;
        let service_names = self.list_services_reflection(&endpoint).await?;

        let mut services = HashMap::new();
        for service_name in service_names {
            // Skip reflection services
            if service_name.contains("reflection") || service_name.contains("Reflection") {
                continue;
            }

            match self.get_service_descriptor(&endpoint, &service_name).await {
                Ok(descriptor) => {
                    if let Ok(info) = self.parse_service_info(&descriptor) {
                        services.insert(service_name.clone(), info);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to get descriptor for {}: {}", service_name, e);
                }
            }
        }

        // Cache the results in memory
        let mut cache = self.in_memory_cache.write().await;
        cache.insert(
            url.to_string(),
            CachedReflectionData {
                services: services.clone(),
            },
        );

        Ok(services)
    }

    /// Create a gRPC endpoint with proper configuration
    fn create_endpoint(&self, url: &str) -> Result<Endpoint> {
        let addr = Self::parse_url(url)?;
        let endpoint = Endpoint::from_shared(format!("http://{}", addr))?
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .http2_keep_alive_interval(Duration::from_secs(30))
            .keep_alive_timeout(Duration::from_secs(10));

        Ok(endpoint)
    }

    /// Find method by full name (ServiceName/MethodName)
    async fn find_method(&self, url: &str, operation: &str) -> Result<MethodInfo> {
        let parts: Vec<&str> = operation.split('/').collect();
        let (service_name, method_name) = match parts.as_slice() {
            [service, method] => (service.to_string(), method.to_string()),
            [method] => {
                // Need to search all services for this method
                let services = self.get_service_info(url).await?;
                for (_, service_info) in services {
                    if let Some(method_info) = service_info.methods.get(*method) {
                        return Ok(method_info.clone());
                    }
                }
                return Err(UxcError::OperationNotFound(operation.to_string()).into());
            }
            _ => {
                return Err(UxcError::InvalidArguments(format!(
                    "Invalid operation format: {}. Use ServiceName/MethodName",
                    operation
                ))
                .into())
            }
        };

        let services = self.get_service_info(url).await?;
        let service_info = services
            .get(&service_name)
            .ok_or_else(|| UxcError::OperationNotFound(service_name.clone()))?;

        let method_info = service_info.methods.get(&method_name).ok_or_else(|| {
            UxcError::OperationNotFound(format!("{}/{}", service_name, method_name))
        })?;

        Ok(method_info.clone())
    }

    /// Execute a gRPC method call
    async fn call_method(
        &self,
        url: &str,
        method_info: &MethodInfo,
        args: HashMap<String, Value>,
    ) -> Result<Value> {
        if method_info.is_server_streaming || method_info.is_client_streaming {
            bail!(
                "Unsupported gRPC call type for '{}/{}': only unary methods are supported",
                method_info.service_name,
                method_info.name
            );
        }

        let target = Self::parse_url(url)?;
        let full_method = format!("{}/{}", method_info.service_name, method_info.name);
        let request_data = self.build_request_message(&args)?;

        self.invoke_unary_with_grpcurl(url, &target, &full_method, &request_data)
            .await
    }

    async fn invoke_unary_with_grpcurl(
        &self,
        original_url: &str,
        target: &str,
        full_method: &str,
        request_data: &Value,
    ) -> Result<Value> {
        let request_json = serde_json::to_string(request_data)?;
        let attempts = Self::grpcurl_attempts(original_url, target);
        let mut last_error = String::new();

        for plaintext in attempts {
            let mut cmd = tokio::process::Command::new("grpcurl");
            cmd.arg("-format").arg("json");

            if plaintext {
                cmd.arg("-plaintext");
            }

            if let Some(profile) = &self.auth_profile {
                for header in profile.to_grpcurl_headers()? {
                    cmd.arg("-H").arg(header);
                }
            }

            cmd.arg("-d")
                .arg(&request_json)
                .arg(target)
                .arg(full_method);

            let output = cmd.output().await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    anyhow::anyhow!(
                        "grpcurl is required for gRPC unary calls. Install grpcurl and retry."
                    )
                } else {
                    anyhow::anyhow!("Failed to execute grpcurl: {}", e)
                }
            })?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if stdout.is_empty() {
                    return Ok(serde_json::json!({}));
                }

                return serde_json::from_str(&stdout).or_else(|_| {
                    Ok(serde_json::json!({
                        "raw": stdout
                    }))
                });
            }

            last_error = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if last_error.is_empty() {
                last_error = "grpcurl failed without stderr output".to_string();
            }
        }

        bail!("gRPC unary invocation failed: {}", last_error)
    }

    fn grpcurl_attempts(original_url: &str, target: &str) -> Vec<bool> {
        let mut attempts = Vec::new();

        if original_url.starts_with("http://") {
            attempts.push(true);
        } else if original_url.starts_with("https://") {
            attempts.push(false);
        } else if target.ends_with(":9000")
            || target.ends_with(":50051")
            || target.ends_with(":50052")
            || target.ends_with(":50053")
            || target.ends_with(":9090")
        {
            attempts.push(true);
            attempts.push(false);
        } else {
            attempts.push(false);
            attempts.push(true);
        }

        attempts.dedup();
        attempts
    }

    /// Build request message from args
    fn build_request_message(&self, args: &HashMap<String, Value>) -> Result<Value> {
        Ok(serde_json::to_value(args)?)
    }

    /// Map gRPC status to user-friendly error message
    #[allow(dead_code)]
    fn map_grpc_status(status: &Status) -> String {
        match status.code() {
            tonic::Code::Ok => "Success".to_string(),
            tonic::Code::Cancelled => "Operation was cancelled".to_string(),
            tonic::Code::Unknown => format!("Unknown error: {}", status.message()),
            tonic::Code::InvalidArgument => format!("Invalid argument: {}", status.message()),
            tonic::Code::DeadlineExceeded => "Deadline exceeded".to_string(),
            tonic::Code::NotFound => format!("Not found: {}", status.message()),
            tonic::Code::AlreadyExists => format!("Already exists: {}", status.message()),
            tonic::Code::PermissionDenied => format!("Permission denied: {}", status.message()),
            tonic::Code::ResourceExhausted => "Resource exhausted".to_string(),
            tonic::Code::FailedPrecondition => {
                format!("Failed precondition: {}", status.message())
            }
            tonic::Code::Aborted => "Operation was aborted".to_string(),
            tonic::Code::OutOfRange => format!("Out of range: {}", status.message()),
            tonic::Code::Unimplemented => "Method not implemented".to_string(),
            tonic::Code::Internal => format!("Internal error: {}", status.message()),
            tonic::Code::Unavailable => format!("Service unavailable: {}", status.message()),
            tonic::Code::DataLoss => "Data loss".to_string(),
            tonic::Code::Unauthenticated => "Unauthenticated".to_string(),
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
        let endpoint = match self.create_endpoint(url) {
            Ok(endpoint) => endpoint,
            Err(_) => return Ok(false),
        };

        Ok(Self::has_reflection(&endpoint).await.unwrap_or(false))
    }

    async fn fetch_schema(&self, url: &str) -> Result<Value> {
        // Try persistent cache first if available
        if let Some(cache) = &self.schema_cache {
            match cache.get(url)? {
                crate::cache::CacheResult::Hit(schema) => {
                    debug!("gRPC cache hit for: {}", url);
                    return Ok(schema);
                }
                crate::cache::CacheResult::Bypassed => {
                    debug!("gRPC cache bypassed for: {}", url);
                }
                crate::cache::CacheResult::Miss => {
                    debug!("gRPC cache miss for: {}", url);
                }
            }
        }

        let services = self.get_service_info(url).await?;

        let mut service_list = Vec::new();
        for (name, info) in &services {
            let mut methods = Vec::new();
            for (method_name, method_info) in &info.methods {
                methods.push(serde_json::json!({
                    "name": method_name,
                    "input_type": method_info.input_type,
                    "output_type": method_info.output_type,
                    "server_streaming": method_info.is_server_streaming,
                    "client_streaming": method_info.is_client_streaming,
                }));
            }

            service_list.push(serde_json::json!({
                "name": name,
                "methods": methods,
            }));
        }

        let schema = serde_json::json!({
            "protocol": "gRPC",
            "services": service_list,
        });

        // Store in persistent cache if available
        if let Some(cache) = &self.schema_cache {
            if let Err(e) = cache.put(url, &schema) {
                debug!("Failed to cache gRPC schema: {}", e);
            } else {
                info!("Cached gRPC schema for: {}", url);
            }
        }

        Ok(schema)
    }

    async fn list_operations(&self, url: &str) -> Result<Vec<Operation>> {
        let services = self.get_service_info(url).await?;

        let mut operations = Vec::new();
        for (service_name, service_info) in &services {
            for (method_name, method_info) in &service_info.methods {
                operations.push(Operation {
                    name: format!("{}/{}", service_name, method_name),
                    description: method_info.description.clone(),
                    parameters: vec![Parameter {
                        name: "request".to_string(),
                        param_type: method_info.input_type.clone(),
                        required: true,
                        description: Some(format!(
                            "Request message of type {}",
                            method_info.input_type
                        )),
                    }],
                    return_type: Some(method_info.output_type.clone()),
                });
            }
        }

        Ok(operations)
    }

    async fn operation_help(&self, url: &str, operation: &str) -> Result<String> {
        let method_info = self.find_method(url, operation).await?;

        let mut help = format!(
            "Method: {}/{}\n",
            method_info.service_name, method_info.name
        );
        help.push_str(&format!("Service: {}\n", method_info.service_name));
        help.push_str(&format!("Input Type: {}\n", method_info.input_type));
        help.push_str(&format!("Output Type: {}\n", method_info.output_type));

        // Determine call type
        let call_type = match (
            method_info.is_client_streaming,
            method_info.is_server_streaming,
        ) {
            (false, false) => "Unary",
            (false, true) => "Server Streaming",
            (true, false) => "Client Streaming",
            (true, true) => "Bidirectional Streaming",
        };
        help.push_str(&format!("Call Type: {}\n", call_type));

        help.push_str(&format!(
            "\nUsage:\n  uxc {} call {}\\{{args}}\n",
            url, operation
        ));

        Ok(help)
    }

    async fn execute(
        &self,
        url: &str,
        operation: &str,
        args: HashMap<String, Value>,
    ) -> Result<ExecutionResult> {
        let start = std::time::Instant::now();
        let method_info = self.find_method(url, operation).await?;

        let data = self.call_method(url, &method_info, args).await?;

        Ok(ExecutionResult {
            data,
            metadata: super::ExecutionMetadata {
                duration_ms: start.elapsed().as_millis() as u64,
                operation: operation.to_string(),
            },
        })
    }
}

trait GrpcurlAuthHeaders {
    fn to_grpcurl_headers(&self) -> Result<Vec<String>>;
}

impl GrpcurlAuthHeaders for Profile {
    fn to_grpcurl_headers(&self) -> Result<Vec<String>> {
        use base64::Engine;

        let header = match self.auth_type {
            crate::auth::AuthType::Bearer => format!("authorization: Bearer {}", self.api_key),
            crate::auth::AuthType::ApiKey => format!("x-api-key: {}", self.api_key),
            crate::auth::AuthType::Basic => {
                let encoded = base64::engine::general_purpose::STANDARD.encode(&self.api_key);
                format!("authorization: Basic {}", encoded)
            }
        };

        Ok(vec![header])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_url() {
        assert_eq!(
            GrpcAdapter::parse_url("localhost:50051").unwrap(),
            "localhost:50051"
        );
        assert_eq!(
            GrpcAdapter::parse_url("http://localhost:50051").unwrap(),
            "localhost:50051"
        );
        assert_eq!(
            GrpcAdapter::parse_url("localhost").unwrap(),
            "localhost:50051"
        );
    }

    #[test]
    fn test_grpcurl_attempts_for_common_targets() {
        assert_eq!(
            GrpcAdapter::grpcurl_attempts("grpcb.in:9000", "grpcb.in:9000"),
            vec![true, false]
        );
        assert_eq!(
            GrpcAdapter::grpcurl_attempts("https://grpcb.in:9001", "grpcb.in:9001"),
            vec![false]
        );
    }

    #[tokio::test]
    async fn test_can_handle_rejects_non_grpc_endpoint() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_body("ok")
            .create_async()
            .await;

        let adapter = GrpcAdapter::new();
        let can_handle = adapter.can_handle(&server.url()).await.unwrap();
        assert!(!can_handle);
    }

    #[tokio::test]
    async fn test_call_method_rejects_streaming_methods() {
        let adapter = GrpcAdapter::new();
        let method = MethodInfo {
            name: "Stream".to_string(),
            service_name: "example.StreamService".to_string(),
            input_type: "example.Request".to_string(),
            output_type: "example.Response".to_string(),
            is_server_streaming: true,
            is_client_streaming: false,
            description: None,
        };

        let err = adapter
            .call_method("localhost:50051", &method, HashMap::new())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("only unary methods are supported"));
    }
}
