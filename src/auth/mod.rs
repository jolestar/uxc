//! Authentication profile storage module
//!
//! Provides filesystem-based storage for authentication profiles.
//! Profiles are stored in ~/.uxc/profiles.toml in plain text (encryption in Phase 2).
//!
//! # Profile Structure
//!
//! ```toml
//! [profile.default]
//! api_key = "sk-..."
//! type = "bearer"
//!
//! [profile.production]
//! api_key = "sk-prod-..."
//! type = "bearer"
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Default profiles directory relative to home directory
pub const DEFAULT_PROFILES_DIR: &str = ".uxc";

/// Default profiles file name
pub const PROFILES_FILE: &str = "profiles.toml";

/// Authentication type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    /// Bearer token authentication
    Bearer,
    /// API key authentication
    ApiKey,
    /// Basic authentication
    Basic,
}

impl Default for AuthType {
    #[allow(clippy::derivable_impls)] // Manual impl needed for serde compatibility with #[serde(rename_all)]
    fn default() -> Self {
        Self::Bearer
    }
}

impl std::fmt::Display for AuthType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthType::Bearer => write!(f, "bearer"),
            AuthType::ApiKey => write!(f, "api_key"),
            AuthType::Basic => write!(f, "basic"),
        }
    }
}

impl std::str::FromStr for AuthType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bearer" => Ok(AuthType::Bearer),
            "api_key" => Ok(AuthType::ApiKey),
            "basic" => Ok(AuthType::Basic),
            _ => anyhow::bail!(
                "Invalid auth type: {}. Valid values: bearer, api_key, basic",
                s
            ),
        }
    }
}

/// Authentication profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// API key or token
    pub api_key: String,

    /// Authentication type
    #[serde(default)]
    pub auth_type: AuthType,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Profile {
    /// Create a new profile
    pub fn new(api_key: String, auth_type: AuthType) -> Self {
        Self {
            api_key,
            auth_type,
            description: None,
        }
    }

    /// Create a new profile with description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Mask the API key for display (show only first 8 and last 4 characters)
    pub fn mask_api_key(&self) -> String {
        let key = &self.api_key;
        if key.len() <= 12 {
            return "*".repeat(key.len());
        }
        format!("{}...{}", &key[..8], &key[key.len() - 4..])
    }
}

/// Profiles collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profiles {
    /// Map of profile name to profile
    #[serde(flatten)]
    pub profiles: HashMap<String, Profile>,
}

impl Default for Profiles {
    fn default() -> Self {
        Self::new()
    }
}

impl Profiles {
    /// Create a new empty profiles collection
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }

    /// Get the profiles file path
    fn profiles_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        Ok(home.join(DEFAULT_PROFILES_DIR).join(PROFILES_FILE))
    }

    /// Load profiles from ~/.uxc/profiles.toml
    ///
    /// If the file doesn't exist, returns an empty profiles collection.
    pub fn load_profiles() -> Result<Self> {
        let path = Self::profiles_path()?;

        if !path.exists() {
            return Ok(Self::new());
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read profiles file: {:?}", path))?;

        // Parse TOML - handle nested [profile.xxx] format
        let mut profiles = Self::new();

        // The format is:
        // [profile.default]
        // api_key = "..."
        // type = "bearer"

        let root = toml::from_str::<toml::Value>(&contents)
            .with_context(|| format!("Failed to parse TOML from file: {:?}", path))?;

        // Look for [profile] section
        let profile_section = root.get("profile")
            .and_then(|v| v.as_table())
            .ok_or_else(|| anyhow::anyhow!("Missing [profile] section in TOML file: {:?}", path))?;

        for (name, value) in profile_section {
            let table = value.as_table()
                .ok_or_else(|| anyhow::anyhow!("Profile '{}' is not a table", name))?;

            let api_key = table.get("api_key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Profile '{}' missing 'api_key' field", name))?;

            let auth_type_str = table.get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("bearer");

            let auth_type = auth_type_str.parse::<AuthType>()
                .with_context(|| format!("Failed to parse auth_type '{}' for profile '{}'", auth_type_str, name))?;

            let description = table.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let profile = Profile {
                api_key: api_key.to_string(),
                auth_type,
                description,
            };

            profiles.profiles.insert(name.clone(), profile);
        }

        Ok(profiles)
    }

    /// Save profiles to ~/.uxc/profiles.toml
    pub fn save_profiles(&self) -> Result<()> {
        let path = Self::profiles_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create profiles directory: {:?}", parent)
                })?;
            }
        }

        // Convert to TOML format with nested [profile.xxx] sections
        let mut root_table = toml::value::Table::new();
        let mut profile_table = toml::value::Table::new();

        for (name, profile) in &self.profiles {
            let mut table = toml::value::Table::new();
            table.insert(
                "api_key".to_string(),
                toml::Value::String(profile.api_key.clone()),
            );
            table.insert(
                "type".to_string(),
                toml::Value::String(profile.auth_type.to_string()),
            );

            if let Some(desc) = &profile.description {
                table.insert("description".to_string(), toml::Value::String(desc.clone()));
            }

            profile_table.insert(name.clone(), toml::Value::Table(table));
        }

        root_table.insert("profile".to_string(), toml::Value::Table(profile_table));

        let toml_string =
            toml::to_string_pretty(&root_table).context("Failed to serialize profiles to TOML")?;

        fs::write(&path, toml_string)
            .with_context(|| format!("Failed to write profiles file: {:?}", path))?;

        Ok(())
    }

    /// Get a profile by name
    pub fn get_profile(&self, name: &str) -> Result<&Profile> {
        self.profiles.get(name).context(format!(
            "Profile '{}' not found. Available profiles: {}",
            name,
            self.list_names()
        ))
    }

    /// Set a profile
    pub fn set_profile(&mut self, name: String, profile: Profile) -> Result<()> {
        self.profiles.insert(name, profile);
        Ok(())
    }

    /// Remove a profile
    pub fn remove_profile(&mut self, name: &str) -> Result<()> {
        self.profiles
            .remove(name)
            .context(format!("Profile '{}' not found", name))?;
        Ok(())
    }

    /// List all profile names
    pub fn list_names(&self) -> String {
        let mut names: Vec<_> = self.profiles.keys().cloned().collect();
        names.sort();
        names.join(", ")
    }

    /// Get all profile names
    pub fn profile_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.profiles.keys().cloned().collect();
        names.sort();
        names
    }

    /// Check if a profile exists
    pub fn has_profile(&self, name: &str) -> bool {
        self.profiles.contains_key(name)
    }

    /// Get the number of profiles
    pub fn count(&self) -> usize {
        self.profiles.len()
    }
}

/// Get the home directory
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        // Try HOME environment variable first (Unix-like systems)
        if let Some(home) = std::env::var_os("HOME") {
            return Some(PathBuf::from(home));
        }

        // Try USERPROFILE on Windows
        #[cfg(windows)]
        {
            if let Some(user_profile) = std::env::var_os("USERPROFILE") {
                return Some(PathBuf::from(user_profile));
            }

            // Fallback to HOMEDRIVE + HOMEPATH on Windows
            if let Some(home_drive) = std::env::var_os("HOMEDRIVE") {
                if let Some(home_path) = std::env::var_os("HOMEPATH") {
                    let mut path = PathBuf::from(&home_drive);
                    path.push(&home_path);
                    return Some(path);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_profile_default() {
        let profile = Profile::new("test-key".to_string(), AuthType::Bearer);
        assert_eq!(profile.api_key, "test-key");
        assert_eq!(profile.auth_type, AuthType::Bearer);
        assert!(profile.description.is_none());
    }

    #[test]
    fn test_profile_with_description() {
        let profile = Profile::new("test-key".to_string(), AuthType::ApiKey)
            .with_description("Test profile".to_string());
        assert_eq!(profile.description, Some("Test profile".to_string()));
    }

    #[test]
    fn test_mask_api_key() {
        let profile = Profile::new("sk-12345678abcdefgh".to_string(), AuthType::Bearer);
        assert_eq!(profile.mask_api_key(), "sk-12345...efgh");
    }

    #[test]
    fn test_mask_short_api_key() {
        let profile = Profile::new("short".to_string(), AuthType::Bearer);
        assert_eq!(profile.mask_api_key(), "*****");
    }

    #[test]
    fn test_auth_type_from_str() {
        assert_eq!(AuthType::from_str("bearer").unwrap(), AuthType::Bearer);
        assert_eq!(AuthType::from_str("BEARER").unwrap(), AuthType::Bearer);
        assert_eq!(AuthType::from_str("api_key").unwrap(), AuthType::ApiKey);
        assert_eq!(AuthType::from_str("basic").unwrap(), AuthType::Basic);
        assert!(AuthType::from_str("invalid").is_err());
    }

    #[test]
    fn test_profiles_new() {
        let profiles = Profiles::new();
        assert_eq!(profiles.count(), 0);
        assert!(!profiles.has_profile("default"));
    }

    #[test]
    fn test_profiles_set_get() {
        let mut profiles = Profiles::new();
        let profile = Profile::new("test-key".to_string(), AuthType::Bearer);

        profiles
            .set_profile("default".to_string(), profile)
            .unwrap();
        assert!(profiles.has_profile("default"));
        assert_eq!(profiles.count(), 1);

        let retrieved = profiles.get_profile("default").unwrap();
        assert_eq!(retrieved.api_key, "test-key");
    }

    #[test]
    fn test_profiles_remove() {
        let mut profiles = Profiles::new();
        let profile = Profile::new("test-key".to_string(), AuthType::Bearer);

        profiles
            .set_profile("default".to_string(), profile)
            .unwrap();
        assert!(profiles.has_profile("default"));

        profiles.remove_profile("default").unwrap();
        assert!(!profiles.has_profile("default"));
        assert_eq!(profiles.count(), 0);
    }

    #[test]
    fn test_profiles_list_names() {
        let mut profiles = Profiles::new();
        profiles
            .set_profile(
                "dev".to_string(),
                Profile::new("key1".to_string(), AuthType::Bearer),
            )
            .unwrap();
        profiles
            .set_profile(
                "prod".to_string(),
                Profile::new("key2".to_string(), AuthType::Bearer),
            )
            .unwrap();

        let names = profiles.profile_names();
        assert_eq!(names, vec!["dev", "prod"]);
        assert_eq!(profiles.list_names(), "dev, prod");
    }
}
