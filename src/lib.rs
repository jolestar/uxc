//! UXC - Universal X-Protocol Call
//!
//! Schema-driven, multi-protocol RPC execution runtime.

pub mod adapters;
pub mod error;
pub mod output;
pub mod protocol;

pub use adapters::{Adapter, ProtocolType};
pub use error::{Result, UxcError};
pub use output::OutputEnvelope;

/// UXC version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// UXC library initialization
pub fn init() {
    // Initialize logging, etc.
}
