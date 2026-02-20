//! GraphQL adapter with introspection support

use super::{Adapter, ProtocolType, Operation, ExecutionResult, ExecutionMetadata};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use anyhow::Result;

const DETECTION_TIMEOUT: Duration = Duration::from_secs(2);

pub struct GraphQLAdapter {
    client: reqwest::Client,
}

impl GraphQLAdapter {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(DETECTION_TIMEOUT)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
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
        let introspection_query = r#"
            {
                __schema {
                    queryType {
                        name
                    }
                }
            }
        "#;

        let result = tokio::time::timeout(
            DETECTION_TIMEOUT,
            self.client
                .post(url)
                .json(&serde_json::json!({ "query": introspection_query }))
                .send()
        ).await;

        if let Ok(Ok(resp)) = result {
            if resp.status().is_success() {
                // Verify it's actually GraphQL by checking the response
                if let Ok(json) = resp.json::<Value>().await {
                    // Check for GraphQL-specific response structure
                    if json.get("data").is_some()
                        || json.get("errors").is_some()
                    {
                        return Ok(true);
                    }
                }
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
        let _schema = self.fetch_schema(url).await?;
        let operations = Vec::new();

        // TODO: Parse introspection result and extract fields
        // For now, return placeholder

        Ok(operations)
    }

    async fn operation_help(&self, url: &str, operation: &str) -> Result<String> {
        // TODO: Implement field help via introspection
        let _ = (url, operation);
        Err(anyhow::anyhow!("GraphQL help not yet fully implemented"))
    }

    async fn execute(
        &self,
        url: &str,
        operation: &str,
        _args: HashMap<String, Value>,
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
