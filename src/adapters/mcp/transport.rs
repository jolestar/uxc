//! MCP stdio transport for communicating with MCP server processes

use super::types::*;
use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value as JsonValue;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};

/// MCP stdio transport client
pub struct McpStdioTransport {
    /// Child process handle
    _child: tokio::process::Child,
    /// Request ID counter
    next_id: Arc<Mutex<i64>>,
    /// Request sender
    request_tx: mpsc::UnboundedSender<OutboundMessage>,
    /// Pending response channels keyed by request id
    response_channels:
        Arc<Mutex<std::collections::HashMap<RequestId, tokio::sync::oneshot::Sender<JsonRpcResponse>>>>,
}

/// Message queued for the writer task
struct OutboundMessage {
    request_id: Option<RequestId>,
    message: String,
}

impl McpStdioTransport {
    /// Spawn a new MCP server process and create a transport
    pub async fn connect(command: &str, args: &[String]) -> Result<Self> {
        // Parse the command (handle quoted strings, etc.)
        let parts = parse_command(command);
        let (cmd, cmd_args) = parts.split_first().context("Empty command")?;

        // Build the full argument list
        let full_args: Vec<&str> = cmd_args
            .iter()
            .map(|s| s.as_str())
            .chain(args.iter().map(|s| s.as_str()))
            .collect();

        tracing::info!("Spawning MCP server: {} {:?}", cmd, full_args);

        // Spawn the process
        let mut child = Command::new(cmd)
            .args(&full_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to spawn MCP server process")?;

        // Get stdin and stdout handles
        let stdin = child.stdin.take().context("Failed to get stdin handle")?;
        let stdout = child.stdout.take().context("Failed to get stdout handle")?;

        // Create channels for sending requests
        let (request_tx, mut request_rx) = mpsc::unbounded_channel::<OutboundMessage>();

        let next_id = Arc::new(Mutex::new(1i64));
        let response_channels = Arc::new(Mutex::new(std::collections::HashMap::<
            RequestId,
            tokio::sync::oneshot::Sender<JsonRpcResponse>,
        >::new()));

        // Spawn a task to handle writing to stdin
        let mut stdin_writer = stdin;
        let response_channels_for_writer = response_channels.clone();
        tokio::spawn(async move {
            while let Some(req) = request_rx.recv().await {
                if let Err(e) = stdin_writer.write_all(req.message.as_bytes()).await {
                    tracing::error!("Failed to write to stdin: {}", e);
                    if let Some(request_id) = req.request_id {
                        let mut channels = response_channels_for_writer.lock().await;
                        if let Some(tx) = channels.remove(&request_id) {
                            let _ = tx.send(JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: request_id,
                                result: None,
                                error: Some(JsonRpcError {
                                    code: -32603,
                                    message: format!("Write error: {}", e),
                                    data: None,
                                }),
                            });
                        }
                    }
                    break;
                }
                if let Err(e) = stdin_writer.write_all(b"\n").await {
                    tracing::error!("Failed to write newline to stdin: {}", e);
                    break;
                }
                if let Err(e) = stdin_writer.flush().await {
                    tracing::error!("Failed to flush stdin: {}", e);
                    break;
                }
            }
        });

        // Spawn a task to read responses from stdout
        let response_channels_clone = response_channels.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut buffer = String::new();

            while let Ok(Some(line)) = lines.next_line().await {
                buffer.push_str(&line);
                buffer.push('\n');

                // Try to parse a JSON-RPC message from the buffer
                while let Some(pos) = find_complete_json(&buffer) {
                    let json_str = buffer[..pos].to_string();
                    buffer = buffer[pos..].to_string();

                    // Parse the JSON-RPC message
                    match parse_jsonrpc_message(&json_str) {
                        Ok(Some(response)) => {
                            let id = response.id.clone();
                            let mut channels = response_channels_clone.lock().await;
                            if let Some(tx) = channels.remove(&id) {
                                let _ = tx.send(response);
                            }
                        }
                        Ok(None) => {
                            // Notification (no response expected), ignore
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse JSON-RPC message: {}", e);
                        }
                    }
                }
            }
        });

        Ok(Self {
            _child: child,
            next_id,
            request_tx,
            response_channels,
        })
    }

    /// Send a request and wait for the response
    pub async fn send_request(
        &mut self,
        method: &str,
        params: Option<JsonValue>,
    ) -> Result<JsonValue> {
        // Get the next ID
        let id = {
            let mut id_guard = self.next_id.lock().await;
            let id = *id_guard;
            *id_guard += 1;
            RequestId::Number(id)
        };

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: id.clone(),
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)?;
        tracing::debug!("Sending request: {}", request_json);

        // Create a response channel
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        // Register request id -> response channel before sending the request
        {
            let mut channels = self.response_channels.lock().await;
            channels.insert(id.clone(), response_tx);
        }

        // Send the request
        if self
            .request_tx
            .send(OutboundMessage {
                request_id: Some(id.clone()),
                message: request_json,
            })
            .is_err()
        {
            let mut channels = self.response_channels.lock().await;
            channels.remove(&id);
            return Err(anyhow!("Request channel closed"));
        }

        // Wait for the response
        let response = response_rx.await.context("Response channel closed")?;

        if let Some(error) = response.error {
            bail!("JSON-RPC error: {} - {}", error.code, error.message);
        }

        response.result.context("No result in response")
    }

    /// Send a notification (no response expected)
    pub async fn send_notification(
        &mut self,
        method: &str,
        params: Option<JsonValue>,
    ) -> Result<()> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        };

        let notification_json = serde_json::to_string(&notification)?;
        tracing::debug!("Sending notification: {}", notification_json);

        self.request_tx
            .send(OutboundMessage {
                request_id: None,
                message: notification_json,
            })
            .map_err(|_| anyhow!("Request channel closed"))?;

        Ok(())
    }

    /// Initialize the MCP session
    pub async fn initialize(&mut self, client_info: ClientInfo) -> Result<InitializeResult> {
        let params = InitializeParams {
            protocolVersion: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            clientInfo: client_info,
        };

        let result = self
            .send_request("initialize", Some(serde_json::to_value(params)?))
            .await?;

        let init_result: InitializeResult = serde_json::from_value(result)?;
        Ok(init_result)
    }

    /// Send initialized notification
    pub async fn initialized(&mut self) -> Result<()> {
        self.send_notification("notifications/initialized", None)
            .await
    }
}

/// Parse a command string into parts (handles quoted strings)
pub fn parse_command(cmd: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escape_next = false;

    for ch in cmd.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
        } else if ch == '\\' {
            escape_next = true;
        } else if ch == '"' {
            in_quotes = !in_quotes;
        } else if ch.is_whitespace() && !in_quotes {
            if !current.is_empty() {
                parts.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

/// Find a complete JSON object in the string
/// Returns the length of the JSON object if found
fn find_complete_json(s: &str) -> Option<usize> {
    let mut brace_count = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, ch) in s.chars().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if ch == '\\' && in_string {
            escape_next = true;
            continue;
        }

        if ch == '"' {
            in_string = !in_string;
            continue;
        }

        if !in_string {
            if ch == '{' {
                brace_count += 1;
            } else if ch == '}' {
                brace_count -= 1;
                if brace_count == 0 {
                    return Some(i + 1);
                }
            }
        }
    }

    None
}

/// Parse a JSON-RPC message
fn parse_jsonrpc_message(s: &str) -> Result<Option<JsonRpcResponse>> {
    let value: JsonValue = serde_json::from_str(s)?;

    // Check if it's a response (has "id" field)
    if value.get("id").is_some() {
        let response: JsonRpcResponse = serde_json::from_value(value)?;
        Ok(Some(response))
    } else {
        // It's a notification, no response expected
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn send_request_routes_response_by_id() {
        let script = "read line; echo '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"ok\":true}}'; sleep 1";
        let mut transport = McpStdioTransport::connect(
            "sh",
            &["-c".to_string(), script.to_string()],
        )
        .await
        .unwrap();

        let response = transport.send_request("ping", None).await.unwrap();
        assert_eq!(response["ok"], true);
    }
}
