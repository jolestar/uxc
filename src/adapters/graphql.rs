//! GraphQL adapter with introspection support

use super::{Adapter, ProtocolType, Operation, Parameter, ExecutionResult, ExecutionMetadata};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;

pub struct GraphQLAdapter {
    client: reqwest::Client,
}

impl GraphQLAdapter {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for GraphQLAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Adapter for GraphQLAdapter {
    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::GraphQL
    }

    async fn can_handle(&self, url: &str) -> Result<bool> {
        // Try GraphQL introspection with timeout
        let query = r#"
            {
                __schema {
                    queryType {
                        name
                    }
                }
            }
        "#;

        let resp = match self
            .client
            .post(url)
            .timeout(std::time::Duration::from_secs(2))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(false),
        };

        if !resp.status().is_success() {
            return Ok(false);
        }

        // Check if response is valid GraphQL with __schema
        if let Ok(body) = resp.json::<Value>().await {
            // A valid GraphQL introspection response should have data.__schema
            // or errors (which still indicates GraphQL)
            if body.get("data").is_some()
                || body.get("errors").is_some()
                || body.get("__schema").is_some()
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn fetch_schema(&self, url: &str) -> Result<Value> {
        let introspection_query = r#"
            {
                __schema {
                    types {
                        name
                        kind
                        description
                        fields {
                            name
                            type {
                                name
                                kind
                            }
                        }
                    }
                }
            }
        "#;

        let resp = self
            .client
            .post(url)
            .json(&serde_json::json!({ "query": introspection_query }))
            .send()
            .await?;

        Ok(resp.json().await?)
    }

    async fn list_operations(&self, url: &str) -> Result<Vec<Operation>> {
        // For GraphQL, we expose the top-level query/mutation fields as operations
        let schema = self.fetch_schema(url).await?;
        let mut operations = Vec::new();

        // TODO: Parse introspection result and extract fields
        // For now, return placeholder

        Ok(operations)
    }

    async fn operation_help(&self, url: &str, operation: &str) -> Result<String> {
        // TODO: Implement field help via introspection
        Err(anyhow::anyhow!("GraphQL help not yet fully implemented"))
    }

    async fn execute(
        &self,
        url: &str,
        operation: &str,
        args: HashMap<String, Value>,
    ) -> Result<ExecutionResult> {
        let start = std::time::Instant::now();

        // Construct GraphQL query from operation name
        let query = format!("{{ {} }}", operation);

        let resp = self
            .client
            .post(url)
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await?;

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
