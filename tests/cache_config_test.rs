//! Cache configuration tests

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use uxc::cache::CacheConfig;

#[test]
fn test_load_from_file_not_exists() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Set HOME to temp directory
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", temp_dir.path());

    // Try to load from non-existent config
    let result = CacheConfig::load_from_file();

    // Should succeed with defaults
    assert!(result.is_ok(), "Should return default config when file doesn't exist");
    let config = result.unwrap();
    assert!(config.enabled(), "Default config should be enabled");

    // Restore HOME
    match prev_home {
        Some(val) => std::env::set_var("HOME", val),
        None => std::env::remove_var("HOME"),
    }
}

#[test]
fn test_load_from_file_valid_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Set HOME to temp directory
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", temp_dir.path());

    // Create config file
    let config_dir = temp_dir.path().join(".uxc");
    fs::create_dir_all(&config_dir).expect("Failed to create config dir");
    let config_file = config_dir.join("config.toml");

    fs::write(
        &config_file,
        r#"
[cache]
enabled = true
ttl = 3600
max_size = 1048576
"#
    ).expect("Failed to write config");

    // Load config
    let result = CacheConfig::load_from_file();

    assert!(result.is_ok(), "Should successfully load valid config");
    let config = result.unwrap();
    assert!(config.enabled(), "Config should be enabled");
    assert_eq!(config.ttl(), 3600, "TTL should be 3600");
    assert_eq!(config.max_size(), 1048576, "Max size should be 1048576");

    // Restore HOME
    match prev_home {
        Some(val) => std::env::set_var("HOME", val),
        None => std::env::remove_var("HOME"),
    }
}

#[test]
fn test_load_from_file_invalid_values_fallback() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Set HOME to temp directory
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", temp_dir.path());

    // Create config file with invalid values
    let config_dir = temp_dir.path().join(".uxc");
    fs::create_dir_all(&config_dir).expect("Failed to create config dir");
    let config_file = config_dir.join("config.toml");

    fs::write(
        &config_file,
        r#"
[cache]
enabled = "not_a_bool"
ttl = "not_a_number"
max_size = -100
"#
    ).expect("Failed to write config");

    // Load config - should use defaults for invalid values
    let result = CacheConfig::load_from_file();

    assert!(result.is_ok(), "Should succeed with fallback defaults");
    let config = result.unwrap();
    // Values should fall back to defaults when invalid

    // Restore HOME
    match prev_home {
        Some(val) => std::env::set_var("HOME", val),
        None => std::env::remove_var("HOME"),
    }
}

#[test]
fn test_load_from_file_with_comments_and_empty_lines() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Set HOME to temp directory
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", temp_dir.path());

    // Create config file with comments and empty lines
    let config_dir = temp_dir.path().join(".uxc");
    fs::create_dir_all(&config_dir).expect("Failed to create config dir");
    let config_file = config_dir.join("config.toml");

    fs::write(
        &config_file,
        r#"
# UXC Configuration File
# This file controls cache behavior

[cache]
enabled = true
ttl = 7200

# Max cache size in bytes
max_size = 2097152

# Additional settings can go here
"#
    ).expect("Failed to write config");

    // Load config
    let result = CacheConfig::load_from_file();

    assert!(result.is_ok(), "Should successfully load config with comments");
    let config = result.unwrap();
    assert!(config.enabled(), "Config should be enabled");
    assert_eq!(config.ttl(), 7200, "TTL should be 7200");

    // Restore HOME
    match prev_home {
        Some(val) => std::env::set_var("HOME", val),
        None => std::env::remove_var("HOME"),
    }
}

#[test]
fn test_ensure_cache_dir_creates_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let config = CacheConfig::new(true, 3600, 1024, temp_dir.path().to_path_buf());

    // ensure_cache_dir should create the directory if it doesn't exist
    let result = config.ensure_cache_dir();

    assert!(result.is_ok(), "Should successfully create cache directory");
    assert!(temp_dir.path().join("cache").exists(), "Cache directory should be created");
}

#[test]
fn test_ensure_cache_dir_handles_existing_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Pre-create the cache directory
    let cache_dir = temp_dir.path().join("cache");
    fs::create_dir(&cache_dir).expect("Failed to create cache dir");

    let config = CacheConfig::new(true, 3600, 1024, temp_dir.path().to_path_buf());

    // ensure_cache_dir should succeed even if directory already exists
    let result = config.ensure_cache_dir();

    assert!(result.is_ok(), "Should succeed when directory already exists");
}
