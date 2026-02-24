//! OpenAPI/Swagger adapter

use super::{
    Adapter, ExecutionMetadata, ExecutionResult, Operation, OperationDetail, Parameter,
    ProtocolType,
};
use crate::auth::Profile;
use crate::error::UxcError;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

pub struct OpenAPIAdapter {
    client: reqwest::Client,
    cache: Option<Arc<dyn crate::cache::Cache>>,
    auth_profile: Option<Profile>,
    discovered_schema_urls: Arc<RwLock<HashMap<String, String>>>,
}

impl OpenAPIAdapter {
    const SCHEMA_ENDPOINTS: [&'static str; 7] = [
        "/openapi.json",
        "/swagger.json",
        "/api-docs",
        "/swagger/v1/swagger.json",
        "/api/docs",
        "/docs/swagger.json",
        "/swagger-docs",
    ];
    const HTTP_METHODS: [&'static str; 8] = [
        "get", "post", "put", "patch", "delete", "head", "options", "trace",
    ];

    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            cache: None,
            auth_profile: None,
            discovered_schema_urls: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_cache(mut self, cache: Arc<dyn crate::cache::Cache>) -> Self {
        self.cache = Some(cache);
        self
    }

    pub fn with_auth(mut self, profile: Profile) -> Self {
        self.auth_profile = Some(profile);
        self
    }

    fn normalized_url(url: &str) -> String {
        url.trim_end_matches('/').to_string()
    }

    fn schema_candidates(url: &str) -> Vec<String> {
        let normalized = Self::normalized_url(url);
        if Self::SCHEMA_ENDPOINTS
            .iter()
            .any(|endpoint| normalized.ends_with(endpoint))
        {
            return vec![normalized];
        }

        let mut candidates = Vec::new();
        for endpoint in Self::SCHEMA_ENDPOINTS {
            candidates.push(format!("{}{}", normalized, endpoint));
        }

        candidates.sort();
        candidates.dedup();
        candidates
    }

    fn is_openapi_document(body: &Value) -> bool {
        body.get("openapi").is_some() || body.get("swagger").is_some()
    }

    fn is_http_method(method: &str) -> bool {
        Self::HTTP_METHODS.contains(&method)
    }

    fn operation_id(method: &str, path: &str) -> String {
        format!("{}:{}", method.to_lowercase(), path)
    }

    fn display_name(method: &str, path: &str) -> String {
        format!("{} {}", method.to_uppercase(), path)
    }

    fn parse_operation_id(operation_id: &str) -> Result<(String, String)> {
        let (method, path) = operation_id.split_once(':').ok_or_else(|| {
            UxcError::InvalidArguments(
                "Invalid operation ID format. Use 'method:/path'".to_string(),
            )
        })?;

        if method.is_empty() || path.is_empty() || !path.starts_with('/') {
            return Err(UxcError::InvalidArguments(
                "Invalid operation ID format. Use 'method:/path'".to_string(),
            )
            .into());
        }

        let method = method.to_lowercase();
        if !Self::is_http_method(&method) {
            return Err(UxcError::InvalidArguments(format!(
                "Unsupported HTTP method in operation ID: {}",
                method
            ))
            .into());
        }

        Ok((method, path.to_string()))
    }

    async fn discover_schema_url(&self, url: &str) -> Result<Option<String>> {
        let normalized = Self::normalized_url(url);
        {
            let cache = self.discovered_schema_urls.read().await;
            if let Some(discovered) = cache.get(&normalized) {
                return Ok(Some(discovered.clone()));
            }
        }

        for full_url in Self::schema_candidates(&normalized) {
            let resp = match self
                .client
                .get(&full_url)
                .timeout(std::time::Duration::from_secs(2))
                .header("Accept", "application/json")
                .send()
                .await
            {
                Ok(r) => r,
                Err(_) => continue,
            };

            if !resp.status().is_success() {
                continue;
            }

            if let Ok(body) = resp.json::<Value>().await {
                if Self::is_openapi_document(&body) {
                    let mut cache = self.discovered_schema_urls.write().await;
                    cache.insert(normalized, full_url.clone());
                    return Ok(Some(full_url));
                }
            }
        }

        Ok(None)
    }
}

impl Default for OpenAPIAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Adapter for OpenAPIAdapter {
    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::OpenAPI
    }

    async fn can_handle(&self, url: &str) -> Result<bool> {
        Ok(self.discover_schema_url(url).await?.is_some())
    }

    async fn fetch_schema(&self, url: &str) -> Result<Value> {
        // Try cache first if available
        if let Some(cache) = &self.cache {
            match cache.get(url)? {
                crate::cache::CacheResult::Hit(schema) => {
                    debug!("OpenAPI cache hit for: {}", url);
                    return Ok(schema);
                }
                crate::cache::CacheResult::Bypassed => {
                    debug!("OpenAPI cache bypassed for: {}", url);
                }
                crate::cache::CacheResult::Miss => {
                    debug!("OpenAPI cache miss for: {}", url);
                }
            }
        }

        // Fetch from remote
        let schema_url = self
            .discover_schema_url(url)
            .await?
            .ok_or_else(|| anyhow::anyhow!("OpenAPI schema endpoint not found for {}", url))?;
        let resp = self.client.get(&schema_url).send().await?;
        let schema: Value = resp.json().await?;

        // Store in cache if available
        if let Some(cache) = &self.cache {
            if let Err(e) = cache.put(url, &schema) {
                debug!("Failed to cache OpenAPI schema: {}", e);
            } else {
                info!("Cached OpenAPI schema for: {}", url);
            }
        }

        Ok(schema)
    }

    async fn list_operations(&self, url: &str) -> Result<Vec<Operation>> {
        let schema = self.fetch_schema(url).await?;
        let mut operations = Vec::new();

        if let Some(paths) = schema.get("paths").and_then(|p| p.as_object()) {
            for (path, methods) in paths {
                if let Some(methods_obj) = methods.as_object() {
                    for (method, spec) in methods_obj {
                        let method = method.to_lowercase();
                        if !Self::is_http_method(&method) {
                            continue;
                        }

                        let operation_id = Self::operation_id(&method, path);
                        let display_name = Self::display_name(&method, path);

                        let mut parameters = Vec::new();
                        if let Some(params) = spec.get("parameters").and_then(|p| p.as_array()) {
                            for param in params {
                                if let Some(name) = param.get("name").and_then(|n| n.as_str()) {
                                    parameters.push(Parameter {
                                        name: name.to_string(),
                                        param_type: param
                                            .get("schema")
                                            .and_then(|s| s.get("type"))
                                            .and_then(|t| t.as_str())
                                            .unwrap_or("string")
                                            .to_string(),
                                        required: param
                                            .get("required")
                                            .and_then(|r| r.as_bool())
                                            .unwrap_or(false),
                                        description: param
                                            .get("description")
                                            .and_then(|d| d.as_str())
                                            .map(|s| s.to_string()),
                                    });
                                }
                            }
                        }

                        operations.push(Operation {
                            operation_id,
                            display_name,
                            description: spec
                                .get("description")
                                .or(spec.get("summary"))
                                .and_then(|d| d.as_str())
                                .map(|s| s.to_string()),
                            parameters,
                            return_type: None,
                        });
                    }
                }
            }
        }

        Ok(operations)
    }

    async fn describe_operation(&self, url: &str, operation: &str) -> Result<OperationDetail> {
        let operations = self.list_operations(url).await?;
        let op = operations
            .iter()
            .find(|o| o.operation_id == operation)
            .ok_or_else(|| UxcError::OperationNotFound(operation.to_string()))?;

        Ok(OperationDetail {
            operation_id: op.operation_id.clone(),
            display_name: op.display_name.clone(),
            description: op.description.clone(),
            parameters: op.parameters.clone(),
            return_type: op.return_type.clone(),
            input_schema: None,
        })
    }

    async fn execute(
        &self,
        url: &str,
        operation: &str,
        args: HashMap<String, Value>,
    ) -> Result<ExecutionResult> {
        let start = std::time::Instant::now();
        let (method, path) = Self::parse_operation_id(operation)?;

        let full_url = format!("{}{}", url.trim_end_matches('/'), path);

        let req = match method.as_str() {
            "get" => self.client.get(&full_url),
            "post" => self.client.post(&full_url),
            "put" => self.client.put(&full_url),
            "delete" => self.client.delete(&full_url),
            "patch" => self.client.patch(&full_url),
            "head" => self.client.head(&full_url),
            "options" => self.client.request(reqwest::Method::OPTIONS, &full_url),
            "trace" => self.client.request(reqwest::Method::TRACE, &full_url),
            _ => {
                return Err(UxcError::InvalidArguments(format!(
                    "Unsupported HTTP method: {}",
                    method
                ))
                .into())
            }
        };

        // Apply authentication if profile is set
        let req = if let Some(profile) = &self.auth_profile {
            crate::auth::apply_auth_to_request(req, &profile.auth_type, &profile.api_key)
        } else {
            req
        };

        let resp = req.json(&args).send().await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn swagger_doc() -> &'static str {
        r#"{
  "swagger": "2.0",
  "info": { "title": "Test", "version": "1.0.0" },
  "paths": {}
}"#
    }

    fn openapi_doc() -> &'static str {
        r#"{
  "openapi": "3.0.0",
  "info": { "title": "Test", "version": "1.0.0" },
  "paths": {}
}"#
    }

    #[tokio::test]
    async fn can_handle_discovers_swagger_json() {
        let mut server = mockito::Server::new_async().await;
        let _swagger = server
            .mock("GET", "/swagger.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(swagger_doc())
            .create_async()
            .await;

        let adapter = OpenAPIAdapter::new();
        assert!(adapter.can_handle(&server.url()).await.unwrap());
    }

    #[tokio::test]
    async fn fetch_schema_uses_discovered_swagger_endpoint() {
        let mut server = mockito::Server::new_async().await;
        let _swagger = server
            .mock("GET", "/swagger.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(swagger_doc())
            .expect(2)
            .create_async()
            .await;

        let adapter = OpenAPIAdapter::new();
        assert!(adapter.can_handle(&server.url()).await.unwrap());
        let schema = adapter.fetch_schema(&server.url()).await.unwrap();
        assert_eq!(schema["swagger"], "2.0");
    }

    #[tokio::test]
    async fn fetch_schema_supports_api_docs_endpoint() {
        let mut server = mockito::Server::new_async().await;
        let _api_docs = server
            .mock("GET", "/api-docs")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(openapi_doc())
            .expect(2)
            .create_async()
            .await;

        let adapter = OpenAPIAdapter::new();
        assert!(adapter.can_handle(&server.url()).await.unwrap());
        let schema = adapter.fetch_schema(&server.url()).await.unwrap();
        assert_eq!(schema["openapi"], "3.0.0");
    }

    #[test]
    fn schema_candidates_do_not_append_to_schema_url() {
        let candidates = OpenAPIAdapter::schema_candidates("https://example.com/openapi.json");
        assert_eq!(candidates, vec!["https://example.com/openapi.json"]);
    }

    #[test]
    fn parse_operation_id_accepts_method_path_format() {
        let (method, path) = OpenAPIAdapter::parse_operation_id("post:/pet").unwrap();
        assert_eq!(method, "post");
        assert_eq!(path, "/pet");
    }

    #[test]
    fn parse_operation_id_rejects_legacy_display_format() {
        let err = OpenAPIAdapter::parse_operation_id("POST /pet").unwrap_err();
        assert!(
            err.to_string().contains("method:/path"),
            "unexpected error: {}",
            err
        );
    }
}
