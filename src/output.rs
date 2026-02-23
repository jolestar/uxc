//! Output formatting - deterministic JSON envelope

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Standard UXC output envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputEnvelope {
    /// Indicates success or failure
    pub ok: bool,

    /// Output kind (operation_list, operation_detail, host_help, call_result, ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,

    /// Protocol type (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,

    /// Endpoint URL (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,

    /// Operation name (present when applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,

    /// Payload data (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,

    /// Error information (present on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,

    /// Metadata
    pub meta: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// Machine-readable error code
    pub code: String,

    /// Human-readable error message
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Envelope schema version
    pub version: String,

    /// Execution duration in milliseconds when applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl OutputEnvelope {
    /// Create a success response
    pub fn success(
        kind: &str,
        protocol: &str,
        endpoint: &str,
        operation: Option<&str>,
        data: Value,
        duration_ms: Option<u64>,
    ) -> Self {
        Self {
            ok: true,
            kind: Some(kind.to_string()),
            protocol: Some(protocol.to_string()),
            endpoint: Some(endpoint.to_string()),
            operation: operation.map(ToString::to_string),
            data: Some(data),
            error: None,
            meta: Metadata {
                version: "v1".to_string(),
                duration_ms,
            },
        }
    }

    /// Create an error response
    pub fn error(code: &str, message: &str) -> Self {
        Self {
            ok: false,
            kind: None,
            protocol: None,
            endpoint: None,
            operation: None,
            data: None,
            error: Some(ErrorInfo {
                code: code.to_string(),
                message: message.to_string(),
            }),
            meta: Metadata {
                version: "v1".to_string(),
                duration_ms: None,
            },
        }
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_envelope() {
        let envelope = OutputEnvelope::success(
            "call_result",
            "openapi",
            "https://api.example.com",
            Some("GET /users"),
            serde_json::json!({"users": []}),
            Some(128),
        );

        assert!(envelope.ok);
        assert_eq!(envelope.kind, Some("call_result".to_string()));
        assert_eq!(envelope.protocol, Some("openapi".to_string()));
        assert_eq!(envelope.operation, Some("GET /users".to_string()));
    }

    #[test]
    fn test_error_envelope() {
        let envelope = OutputEnvelope::error("INVALID_ARGUMENT", "Field 'id' must be an integer");

        assert!(!envelope.ok);
        assert_eq!(
            envelope.error.as_ref().map(|e| e.code.clone()),
            Some("INVALID_ARGUMENT".to_string())
        );
        assert_eq!(envelope.meta.version, "v1");
    }
}
