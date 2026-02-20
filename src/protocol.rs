//! Protocol detection and routing

use crate::adapters::{Adapter, ProtocolDetector, ProtocolType, AdapterEnum};
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
        let adapter = self.detector.detect_adapter(url).await
            .map_err(|e| UxcError::GenericError(e))?;
        Ok(adapter.protocol_type())
    }

    /// Get adapter for a URL (auto-detects protocol)
    pub async fn get_adapter_for_url(&self, url: &str) -> Result<AdapterEnum> {
        self.detector.detect_adapter(url).await
            .map_err(|e| UxcError::GenericError(e))
    }
}

impl Default for ProtocolRouter {
    fn default() -> Self {
        Self::new()
    }
}
