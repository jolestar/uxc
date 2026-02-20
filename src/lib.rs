//! UXC - Universal X-Protocol Call
//!
//! Schema-driven, multi-protocol RPC execution runtime.

pub mod adapters;
pub mod protocol;
pub mod error;
pub mod output;

pub use adapters::{Adapter, ProtocolType};
pub use error::{UxcError, Result};
pub use output::OutputEnvelope;
pub use protocol::ProtocolRouter;

/// UXC version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// UXC library initialization
pub fn init() {
    // Initialize logging, etc.
}
