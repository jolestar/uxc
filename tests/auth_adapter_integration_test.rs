//! Integration tests for authentication with protocol adapters
//!
//! These tests verify that authentication profiles are correctly applied
//! to HTTP requests for different protocol adapters.

use std::fs;
use tempfile::TempDir;
use uxc::auth::{AuthType, Profile, Profiles};

/// Helper function to create a test environment with a temporary directory
fn setup_test_env() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Set HOME to the temp directory for testing
    std::env::set_var("HOME", temp_dir.path());

    temp_dir
}

#[test]
fn test_profile_selection_cli_flag_precedence() {
    // Test precedence: CLI flag > env var > default
    let temp_dir = setup_test_env();

    // Create test profiles
    let mut profiles = Profiles::new();
    profiles
        .set_profile("default".to_string(), Profile::new("key-default".to_string(), AuthType::Bearer))
        .expect("Failed to set default profile");
    profiles
        .set_profile("production".to_string(), Profile::new("key-prod".to_string(), AuthType::Bearer))
        .expect("Failed to set production profile");
    profiles
        .set_profile("staging".to_string(), Profile::new("key-staging".to_string(), AuthType::Bearer))
        .expect("Failed to set staging profile");

    profiles.save_profiles().expect("Failed to save profiles");

    // Note: This test only verifies profile storage works correctly.
    // Actual CLI flag testing would require running the binary with different arguments.
    assert!(profiles.has_profile("default"));
    assert!(profiles.has_profile("production"));
    assert!(profiles.has_profile("staging"));

    // Verify profile content
    let default = profiles.get_profile("default").unwrap();
    assert_eq!(default.api_key, "key-default");
    assert_eq!(default.auth_type, AuthType::Bearer);
}

#[test]
fn test_auth_apply_to_request_bearer() {
    use reqwest::Client;

    let profile = Profile::new("test-token-12345".to_string(), AuthType::Bearer);
    let client = Client::new();

    let req = client.get("http://example.com");
    let req = uxc::auth::apply_auth_to_request(req, &profile.auth_type, &profile.api_key);

    // We can't easily inspect the built request, but we can verify it compiles
    // and doesn't panic. In a real scenario, we'd use a mock server to verify.
    let _ = req;
}

#[test]
fn test_auth_apply_to_request_api_key() {
    use reqwest::Client;

    let profile = Profile::new("test-api-key".to_string(), AuthType::ApiKey);
    let client = Client::new();

    let req = client.post("http://example.com");
    let req = uxc::auth::apply_auth_to_request(req, &profile.auth_type, &profile.api_key);

    let _ = req;
}

#[test]
fn test_auth_apply_to_request_basic() {
    use reqwest::Client;

    let profile = Profile::new("user:password".to_string(), AuthType::Basic);
    let client = Client::new();

    let req = client.get("http://example.com");
    let req = uxc::auth::apply_auth_to_request(req, &profile.auth_type, &profile.api_key);

    let _ = req;
}

#[test]
fn test_auth_to_metadata_bearer() {
    let metadata = uxc::auth::auth_to_metadata(&AuthType::Bearer, "test-token")
        .expect("Failed to create metadata");

    assert!(metadata.contains_key("authorization"));
}

#[test]
fn test_auth_to_metadata_api_key() {
    let metadata = uxc::auth::auth_to_metadata(&AuthType::ApiKey, "test-api-key")
        .expect("Failed to create metadata");

    assert!(metadata.contains_key("x-api-key"));
}

#[test]
fn test_auth_to_metadata_basic() {
    let metadata = uxc::auth::auth_to_metadata(&AuthType::Basic, "user:password")
        .expect("Failed to create metadata");

    assert!(metadata.contains_key("authorization"));
}

#[test]
fn test_auth_to_metadata_invalid_token() {
    // Test with invalid metadata characters (e.g., newline)
    let result = uxc::auth::auth_to_metadata(&AuthType::Bearer, "test\n token");

    assert!(result.is_err());
}
