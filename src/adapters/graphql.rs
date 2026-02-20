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
        // Try GraphQL introspection with short timeout for fast detection
        let timeout_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .build()?;

        // Minimal introspection query for fast detection
        let query = r#"
            {
                __schema {
                    queryType {
                        name
                    }
                }
            }
        "#;

        match timeout_client
            .post(url)
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await
        {
            Ok(resp) => {
                if !resp.status().is_success() {
                    return Ok(false);
                }

                // Verify response structure is valid GraphQL
                if let Ok(json) = resp.json::<Value>().await {
                    // Check for GraphQL response structure (data field present)
                    if json.get("data").is_some() {
                        // Also check that there's no errors field, or if there is,
                        // it doesn't indicate that introspection is disabled
                        if let Some(errors) = json.get("errors") {
                            if let Some(err_arr) = errors.as_array() {
                                for err in err_arr {
                                    if let Some(msg) = err.get("message").and_then(|m| m.as_str()) {
                                        if msg.contains("introspection") || msg.contains("disabled") {
                                            return Ok(false);
                                        }
                                    }
                                }
                            }
                        }
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Err(_) => Ok(false),
        }
    }

    async fn fetch_schema(&self, url: &str) -> Result<Value> {
        // More complete introspection query
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
                                ofType {
                                    name
                                    kind
                                }
                            }
                        }
                        inputFields {
                            name
                            type {
                                name
                                kind
                            }
                        }
                    }
                    queryType {
                        fields {
                            name
                            description
                            args {
                                name
                                type {
                                    name
                                    kind
                                }
                            }
                            type {
                                name
                                    kind
                            }
                        }
                    }
                    mutationType {
                        fields {
                            name
                            description
                            args {
                                name
                                type {
                                    name
                                    kind
                                }
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
        let schema = self.fetch_schema(url).await?;
        let mut operations = Vec::new();

        // Extract query fields
        if let Some(data) = schema.get("data") {
            if let Some(_schema) = data.get("__schema") {
                // Get query operations
                if let Some(query_type) = _schema.get("queryType") {
                    if let Some(fields) = query_type.get("fields").and_then(|f| f.as_array()) {
                        for field in fields {
                            let name = field.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");

                            let mut parameters = Vec::new();
                            if let Some(args) = field.get("args").and_then(|a| a.as_array()) {
                                for arg in args {
                                    if let Some(arg_name) = arg.get("name").and_then(|n| n.as_str()) {
                                        let param_type = arg
                                            .get("type")
                                            .and_then(|t| t.get("name"))
                                            .and_then(|n| n.as_str())
                                            .unwrap_or("unknown");

                                        parameters.push(Parameter {
                                            name: arg_name.to_string(),
                                            param_type: param_type.to_string(),
                                            required: false, // GraphQL args are optional by default
                                            description: None,
                                        });
                                    }
                                }
                            }

                            operations.push(Operation {
                                name: format!("query:{}", name),
                                description: field.get("description")
                                    .and_then(|d| d.as_str())
                                    .map(|s| s.to_string()),
                                parameters,
                                return_type: None,
                            });
                        }
                    }
                }

                // Get mutation operations
                if let Some(mutation_type) = _schema.get("mutationType") {
                    if let Some(fields) = mutation_type.get("fields").and_then(|f| f.as_array()) {
                        for field in fields {
                            let name = field.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");

                            let mut parameters = Vec::new();
                            if let Some(args) = field.get("args").and_then(|a| a.as_array()) {
                                for arg in args {
                                    if let Some(arg_name) = arg.get("name").and_then(|n| n.as_str()) {
                                        let param_type = arg
                                            .get("type")
                                            .and_then(|t| t.get("name"))
                                            .and_then(|n| n.as_str())
                                            .unwrap_or("unknown");

                                        parameters.push(Parameter {
                                            name: arg_name.to_string(),
                                            param_type: param_type.to_string(),
                                            required: false,
                                            description: None,
                                        });
                                    }
                                }
                            }

                            operations.push(Operation {
                                name: format!("mutation:{}", name),
                                description: field.get("description")
                                    .and_then(|d| d.as_str())
                                    .map(|s| s.to_string()),
                                parameters,
                                return_type: None,
                            });
                        }
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

        // Parse operation name (e.g., "query:user" or "mutation:createUser")
        let parts: Vec<&str> = operation.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid operation format. Expected 'operation_name:field_name'"));
        }

        let op_type = parts[0]; // "query" or "mutation"
        let field_name = parts[1];

        // Build arguments string
        let args_str = if args.is_empty() {
            String::new()
        } else {
            let args_vec: Vec<String> = args
                .iter()
                .map(|(k, v)| {
                    let value_str = if v.is_string() {
                        format!("\"{}\"", v.as_str().unwrap())
                    } else {
                        v.to_string()
                    };
                    format!("{}: {}", k, value_str)
                })
                .collect();
            format!("({})", args_vec.join(", "))
        };

        // Construct GraphQL query
        let query = format!("{} {}{} {{ {{ }} }}",
            if op_type == "mutation" { "mutation" } else { "query" },
            field_name,
            args_str
        );

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
