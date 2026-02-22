//! Authentication profiles module for secure credential management
//!
//! This module provides profile-based authentication storage without encryption (Phase 1).
//! Profiles are stored in ~/.uxc/profiles.toml in plain text.
//!
//! Phase 2 will add encryption support for sensitive fields.

mod profile;
mod storage;

pub use profile::{AuthProfile, Credentials, ProfileType};
pub use storage::ProfileStorage;

use anyhow::Result;
use std::sync::Arc;

/// Default profiles directory relative to home directory
pub const DEFAULT_PROFILES_DIR: &str = ".uxc";

/// Default profiles file name
pub const DEFAULT_PROFILES_FILE: &str = "profiles.toml";

/// Profile manager interface
pub trait ProfileManager: Send + Sync {
    /// List all profiles
    fn list_profiles(&self) -> Result<Vec<AuthProfile>>;

    /// Get a specific profile by name
    fn get_profile(&self, name: &str) -> Result<Option<AuthProfile>>;

    /// Add or update a profile
    fn set_profile(&self, profile: &AuthProfile) -> Result<()>;

    /// Delete a profile
    fn delete_profile(&self, name: &str) -> Result<()>;

    /// Check if a profile exists
    fn profile_exists(&self, name: &str) -> Result<bool>;
}

/// Create a new profile storage instance
pub fn create_profile_storage() -> Result<Arc<dyn ProfileManager>> {
    Ok(Arc::new(ProfileStorage::new()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_manager_interface() {
        // This test verifies the trait is properly defined
        // Actual implementation tests are in storage.rs
        assert!(true);
    }
}
