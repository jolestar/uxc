//! Cache configuration

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Cache configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether caching is enabled
    pub enabled: bool,

    /// Time-to-live for cache entries in seconds
    pub ttl: u64,

    /// Maximum cache size in bytes (0 = unlimited)
    pub max_size: u64,

    /// Cache directory path
    pub location: PathBuf,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl: super::DEFAULT_CACHE_TTL,
            max_size: 0, // Unlimited
            location: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(super::DEFAULT_CACHE_DIR),
        }
    }
}

impl CacheConfig {
    /// Create a new cache configuration with custom settings
    pub fn new(enabled: bool, ttl: u64, max_size: u64, location: PathBuf) -> Self {
        Self {
            enabled,
            ttl,
            max_size,
            location,
        }
    }

    /// Create configuration from cache options
    pub fn from_options(options: CacheOptions) -> Self {
        let mut config = Self::default();

        if let Some(enabled) = options.enabled {
            config.enabled = enabled;
        }

        if let Some(ttl) = options.ttl {
            config.ttl = ttl;
        }

        if let Some(max_size) = options.max_size {
            config.max_size = max_size;
        }

        if let Some(location) = options.location {
            config.location = location;
        }

        config
    }

    /// Load configuration from a config file
    ///
    /// Looks for ~/.uxc/config.toml and reads the [cache] section if present.
    /// If the file doesn't exist or doesn't have a [cache] section, returns defaults.
    pub fn load_from_file() -> Result<Self> {
        let config_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".uxc/config.toml");

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

        // Parse the [cache] section from TOML
        // For now, we'll do simple parsing since we don't want to add a TOML dependency
        // just for this one feature. If the config becomes more complex, we should
        // add a proper TOML parser.
        let mut config = Self::default();

        for line in contents.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Look for [cache] section
            if line == "[cache]" {
                continue;
            }

            // Parse key-value pairs within [cache] section
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "enabled" => {
                        config.enabled = value.parse::<bool>().unwrap_or(config.enabled);
                    }
                    "ttl" => {
                        config.ttl = value.parse::<u64>().unwrap_or(config.ttl);
                    }
                    "max_size" => {
                        config.max_size = value.parse::<u64>().unwrap_or(config.max_size);
                    }
                    "location" => {
                        config.location = PathBuf::from(value);
                    }
                    _ => {}
                }
            }
        }

        Ok(config)
    }

    /// Ensure the cache directory exists
    pub fn ensure_cache_dir(&self) -> Result<()> {
        if !self.location.exists() {
            fs::create_dir_all(&self.location).with_context(|| {
                format!("Failed to create cache directory: {:?}", self.location)
            })?;
        }
        Ok(())
    }
}

/// Runtime cache options that can override configuration
///
/// These are typically set via CLI flags like --no-cache or --cache-ttl.
#[derive(Debug, Clone, Default)]
pub struct CacheOptions {
    /// Override the enabled setting
    pub enabled: Option<bool>,

    /// Override the TTL setting
    pub ttl: Option<u64>,

    /// Override the max_size setting
    pub max_size: Option<u64>,

    /// Override the location setting
    pub location: Option<PathBuf>,
}

impl CacheOptions {
    /// Create new cache options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the enabled flag
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    /// Set the TTL
    pub fn with_ttl(mut self, ttl: u64) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Set the max size
    pub fn with_max_size(mut self, max_size: u64) -> Self {
        self.max_size = Some(max_size);
        self
    }

    /// Set the location
    pub fn with_location(mut self, location: PathBuf) -> Self {
        self.location = Some(location);
        self
    }
}

/// Helper functions for directory resolution
mod dirs {
    use std::path::PathBuf;

    /// Get the home directory
    pub fn home_dir() -> Option<PathBuf> {
        if let Some(home) = std::env::var_os("HOME") {
            return Some(PathBuf::from(home));
        }

        #[cfg(windows)]
        {
            if let Some(user_profile) = std::env::var_os("USERPROFILE") {
                return Some(PathBuf::from(user_profile));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CacheConfig::default();
        assert!(config.enabled);
        assert_eq!(config.ttl, 86400);
        assert_eq!(config.max_size, 0);
    }

    #[test]
    fn test_config_from_options() {
        let options = CacheOptions::new().with_enabled(false).with_ttl(3600);

        let config = CacheConfig::from_options(options);
        assert!(!config.enabled);
        assert_eq!(config.ttl, 3600);
    }

    #[test]
    fn test_cache_options_builder() {
        let options = CacheOptions::new()
            .with_enabled(true)
            .with_ttl(7200)
            .with_max_size(1024);

        assert_eq!(options.enabled, Some(true));
        assert_eq!(options.ttl, Some(7200));
        assert_eq!(options.max_size, Some(1024));
    }
}
