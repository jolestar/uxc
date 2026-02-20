//! Protocol detection and routing

use crate::adapters::{ProtocolDetector, ProtocolType, AdapterEnum};
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
    /// Returns ProtocolType if detected, or error if no supported protocol found
    pub async fn detect_protocol(&self, url: &str) -> Result<ProtocolType> {
        self.detector.detect_protocol_type(url).await
            .map_err(|e| UxcError::ProtocolDetectionFailed(e.to_string()))
    }

    /// Get adapter for a URL (auto-detects protocol)
    pub async fn get_adapter_for_url(&self, url: &str) -> Result<AdapterEnum> {
        self.detector.detect_adapter(url).await
            .map_err(|e| UxcError::ProtocolDetectionFailed(e.to_string()))
    }
}

impl Default for ProtocolRouter {
    fn default() -> Self {
        Self::new()
    }
}
