//! OpenAPI/Swagger adapter

use super::{Adapter, ProtocolType, Operation, Parameter, ExecutionResult, ExecutionMetadata};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use anyhow::Result;

const DETECTION_TIMEOUT: Duration = Duration::from_secs(2);

pub struct OpenAPIAdapter {
    client: reqwest::Client,
}

impl OpenAPIAdapter {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(DETECTION_TIMEOUT)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
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
        // Try common OpenAPI endpoints with timeout
        let endpoints = [
            "/openapi.json",
            "/swagger.json",
            "/api-docs",
            "/swagger/v1/swagger.json",
            "/api/docs",
            "/docs/swagger.json",
        ];

        let base_url = url.trim_end_matches('/');

        for endpoint in endpoints {
            let full_url = format!("{}{}", base_url, endpoint);

            // Use timeout for each endpoint probe
            let result = tokio::time::timeout(
                DETECTION_TIMEOUT,
                self.client.get(&full_url).send()
            ).await;

            if let Ok(Ok(resp)) = result {
                if resp.status().is_success() {
                    // Verify it's actually JSON and has OpenAPI fields
                    if let Ok(text) = resp.text().await {
                        if let Ok(value) = serde_json::from_str::<Value>(&text) {
                            // Check for OpenAPI/Swagger identifiers
                            if value.get("openapi").is_some()
                                || value.get("swagger").is_some()
                                || value.get("paths").is_some()
                            {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    async fn fetch_schema(&self, url: &str) -> Result<Value> {
        let schema_url = format!("{}/openapi.json", url.trim_end_matches('/'));
        let resp = self.client.get(&schema_url).send().await?;
        Ok(resp.json().await?)
    }

    async fn list_operations(&self, url: &str) -> Result<Vec<Operation>> {
        let schema = self.fetch_schema(url).await?;
        let mut operations = Vec::new();

        if let Some(paths) = schema.get("paths").and_then(|p| p.as_object()) {
            for (path, methods) in paths {
                if let Some(methods_obj) = methods.as_object() {
                    for (method, spec) in methods_obj {
                        let operation_name = format!("{} {}", method.to_uppercase(), path);

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
                            name: operation_name,
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

        // Parse operation (e.g., "GET /users/{id}")
        let parts: Vec<&str> = operation.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid operation format"));
        }

        let method = parts[0];
        let path = parts[1];

        let full_url = format!("{}{}", url.trim_end_matches('/'), path);

        let req = match method.to_uppercase().as_str() {
            "GET" => self.client.get(&full_url),
            "POST" => self.client.post(&full_url),
            "PUT" => self.client.put(&full_url),
            "DELETE" => self.client.delete(&full_url),
            "PATCH" => self.client.patch(&full_url),
            _ => return Err(anyhow::anyhow!("Unsupported HTTP method: {}", method)),
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
