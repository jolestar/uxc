//! Protocol detection and routing

use crate::adapters::{Adapter, ProtocolDetector, ProtocolType};
use crate::error::{Result, UxcError};

/// Protocol detector and router
pub struct ProtocolRouter {
    detector: ProtocolDetector,
}

impl ProtocolRouter {
    pub fn new() -> Self {
        Self {
            detector: ProtocolDetector::new(),
        }
    }

    /// Detect protocol for a given URL
    pub async fn detect_protocol(&self, url: &str) -> Result<ProtocolType> {
        self.detector
            .detect(url)
            .await
            .map_err(|e| UxcError::ProtocolDetectionFailed(format!("{}: {}", url, e)))?
            .ok_or_else(|| UxcError::ProtocolDetectionFailed(url.to_string()))
    }

    /// Get adapter for a URL (auto-detects protocol)
    pub async fn get_adapter_for_url(&self, url: &str) -> Result<&dyn Adapter> {
        let protocol = self.detect_protocol(url).await?;
        self.detector
            .get_adapter(protocol)
            .ok_or_else(|| UxcError::UnsupportedProtocol(protocol.as_str().to_string()))
    }
}

impl Default for ProtocolRouter {
    fn default() -> Self {
        Self::new()
    }
}
