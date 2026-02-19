//! Output formatting - deterministic JSON envelope

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Standard UXC output envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputEnvelope {
    /// Indicates success or failure
    pub ok: bool,

    /// Protocol type (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,

    /// Endpoint URL (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,

    /// Operation name (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,

    /// Result data (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error information (present on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,

    /// Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Metadata>,
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
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

impl OutputEnvelope {
    /// Create a success response
    pub fn success(
        protocol: &str,
        endpoint: &str,
        operation: &str,
        result: Value,
        duration_ms: u64,
    ) -> Self {
        Self {
            ok: true,
            protocol: Some(protocol.to_string()),
            endpoint: Some(endpoint.to_string()),
            operation: Some(operation.to_string()),
            result: Some(result),
            error: None,
            meta: Some(Metadata { duration_ms }),
        }
    }

    /// Create an error response
    pub fn error(code: &str, message: &str) -> Self {
        Self {
            ok: false,
            protocol: None,
            endpoint: None,
            operation: None,
            result: None,
            error: Some(ErrorInfo {
                code: code.to_string(),
                message: message.to_string(),
            }),
            meta: None,
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
            "openapi",
            "https://api.example.com",
            "GET /users",
            serde_json::json!({"users": []}),
            128,
        );

        assert!(envelope.ok);
        assert_eq!(envelope.protocol, Some("openapi".to_string()));
        assert_eq!(envelope.operation, Some("GET /users".to_string()));
    }

    #[test]
    fn test_error_envelope() {
        let envelope = OutputEnvelope::error("INVALID_ARGUMENT", "Field 'id' must be an integer");

        assert!(!envelope.ok);
        assert_eq!(envelope.error.unwrap().code, "INVALID_ARGUMENT");
    }
}
