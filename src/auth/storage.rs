//! Profile storage implementation using TOML files

use super::profile::AuthProfile;
use super::{ProfileManager, DEFAULT_PROFILES_DIR, DEFAULT_PROFILES_FILE};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

// Import the external dirs crate for home directory resolution
use dirs;

/// TOML file structure for storing profiles
#[derive(Debug, Serialize, Deserialize, Default)]
struct ProfilesFile {
    #[serde(default)]
    profiles: HashMap<String, serde_json::Value>,
}

/// Filesystem-based profile storage
pub struct ProfileStorage {
    profiles_path: PathBuf,
}

impl ProfileStorage {
    /// Create a new profile storage instance
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

        let profiles_dir = home_dir.join(DEFAULT_PROFILES_DIR);
        let profiles_path = profiles_dir.join(DEFAULT_PROFILES_FILE);

        // Ensure profiles directory exists
        if !profiles_dir.exists() {
            fs::create_dir_all(&profiles_dir).with_context(|| {
                format!("Failed to create profiles directory: {:?}", profiles_dir)
            })?;
            info!("Created profiles directory: {:?}", profiles_dir);
        }

        // Create profiles file if it doesn't exist
        if !profiles_path.exists() {
            let default_file = ProfilesFile::default();
            Self::save_profiles(&profiles_path, &default_file)?;
            info!("Created profiles file: {:?}", profiles_path);
        }

        Ok(Self { profiles_path })
    }

    /// Load profiles from TOML file
    fn load_profiles(&self) -> Result<ProfilesFile> {
        let file = File::open(&self.profiles_path)
            .with_context(|| format!("Failed to open profiles file: {:?}", self.profiles_path))?;

        let mut buf_reader = BufReader::new(file);

        // Read as TOML
        let mut toml_content = String::new();
        std::io::Read::read_to_string(&mut buf_reader, &mut toml_content)?;

        let profiles_file: ProfilesFile = toml::from_str(&toml_content)
            .with_context(|| format!("Failed to parse profiles file: {:?}", self.profiles_path))?;

        Ok(profiles_file)
    }

    /// Save profiles to TOML file
    fn save_profiles(path: &Path, profiles: &ProfilesFile) -> Result<()> {
        let file = File::create(path)
            .with_context(|| format!("Failed to create profiles file: {:?}", path))?;

        let mut buf_writer = BufWriter::new(file);

        // Serialize to TOML
        let toml_string = toml::to_string_pretty(profiles)
            .with_context(|| "Failed to serialize profiles to TOML")?;

        std::io::Write::write_all(&mut buf_writer, toml_string.as_bytes())
            .with_context(|| format!("Failed to write profiles file: {:?}", path))?;

        // Set restrictive file permissions (0600 = read/write for owner only)
        // This is important since the file contains sensitive credentials in plain text (Phase 1)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(path, perms)
                .with_context(|| format!("Failed to set permissions for profiles file: {:?}", path))?;
        }

        debug!("Saved profiles to: {:?}", path);
        Ok(())
    }

    /// Deserialize a profile from JSON value
    fn deserialize_profile(name: &str, value: &serde_json::Value) -> Result<AuthProfile> {
        // Add the name field if not present
        let mut profile_data = value.clone();
        if let Some(obj) = profile_data.as_object_mut() {
            if !obj.contains_key("name") {
                obj.insert("name".to_string(), serde_json::json!(name));
            }
        }

        serde_json::from_value(profile_data)
            .with_context(|| format!("Failed to deserialize profile: {}", name))
    }

    /// Serialize a profile to JSON value
    fn serialize_profile(profile: &AuthProfile) -> Result<serde_json::Value> {
        serde_json::to_value(profile)
            .with_context(|| format!("Failed to serialize profile: {}", profile.name))
    }
}

impl ProfileManager for ProfileStorage {
    fn list_profiles(&self) -> Result<Vec<AuthProfile>> {
        let profiles_file = self.load_profiles()?;
        let mut profiles = Vec::new();

        for (name, value) in &profiles_file.profiles {
            match Self::deserialize_profile(name, value) {
                Ok(profile) => profiles.push(profile),
                Err(e) => {
                    warn!("Failed to deserialize profile '{}': {}", name, e);
                }
            }
        }

        profiles.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(profiles)
    }

    fn get_profile(&self, name: &str) -> Result<Option<AuthProfile>> {
        let profiles_file = self.load_profiles()?;

        match profiles_file.profiles.get(name) {
            Some(value) => {
                let profile = Self::deserialize_profile(name, value)?;
                Ok(Some(profile))
            }
            None => Ok(None),
        }
    }

    fn set_profile(&self, profile: &AuthProfile) -> Result<()> {
        // Validate the profile before saving
        profile
            .validate()
            .map_err(|e| anyhow::anyhow!("Invalid profile: {}", e))?;

        let mut profiles_file = self.load_profiles()?;

        let profile_value = Self::serialize_profile(profile)?;
        profiles_file
            .profiles
            .insert(profile.name.clone(), profile_value);

        Self::save_profiles(&self.profiles_path, &profiles_file)?;

        info!("Saved profile: {}", profile.name);
        Ok(())
    }

    fn delete_profile(&self, name: &str) -> Result<()> {
        let mut profiles_file = self.load_profiles()?;

        if profiles_file.profiles.remove(name).is_some() {
            Self::save_profiles(&self.profiles_path, &profiles_file)?;
            info!("Deleted profile: {}", name);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Profile not found: {}", name))
        }
    }

    fn profile_exists(&self, name: &str) -> Result<bool> {
        let profiles_file = self.load_profiles()?;
        Ok(profiles_file.profiles.contains_key(name))
    }
}

#[cfg(test)]
mod tests {
    use super::super::profile::{Credentials, ProfileType};
    use super::*;
    use tempfile::TempDir;

    fn create_test_storage() -> (ProfileStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let profiles_path = temp_dir.path().join("profiles.toml");

        // Create default profiles file
        let default_file = ProfilesFile::default();
        ProfileStorage::save_profiles(&profiles_path, &default_file).unwrap();

        let storage = ProfileStorage {
            profiles_path: profiles_path.clone(),
        };

        (storage, temp_dir)
    }

    #[test]
    fn test_create_profile_storage() {
        let (_storage, _temp) = create_test_storage();
        // Test passes if storage was created successfully
    }

    #[test]
    fn test_set_and_get_profile() {
        let (storage, _temp) = create_test_storage();

        let profile = AuthProfile::new(
            "test-profile".to_string(),
            ProfileType::Bearer,
            "https://api.example.com".to_string(),
            Credentials::Bearer {
                token: "test-token-123".to_string(),
            },
        );

        // Set profile
        storage.set_profile(&profile).unwrap();

        // Get profile
        let retrieved = storage.get_profile("test-profile").unwrap();
        assert!(retrieved.is_some());

        let retrieved_profile = retrieved.unwrap();
        assert_eq!(retrieved_profile.name, "test-profile");
        assert_eq!(retrieved_profile.endpoint, "https://api.example.com");
    }

    #[test]
    fn test_list_profiles() {
        let (storage, _temp) = create_test_storage();

        // Add multiple profiles
        let profile1 = AuthProfile::new(
            "profile-1".to_string(),
            ProfileType::Bearer,
            "https://api1.example.com".to_string(),
            Credentials::Bearer {
                token: "token1".to_string(),
            },
        );

        let profile2 = AuthProfile::new(
            "profile-2".to_string(),
            ProfileType::Basic,
            "https://api2.example.com".to_string(),
            Credentials::Basic {
                username: "user".to_string(),
                password: "pass".to_string(),
            },
        );

        storage.set_profile(&profile1).unwrap();
        storage.set_profile(&profile2).unwrap();

        // List profiles
        let profiles = storage.list_profiles().unwrap();
        assert_eq!(profiles.len(), 2);
    }

    #[test]
    fn test_delete_profile() {
        let (storage, _temp) = create_test_storage();

        let profile = AuthProfile::new(
            "test-profile".to_string(),
            ProfileType::Bearer,
            "https://api.example.com".to_string(),
            Credentials::Bearer {
                token: "test-token".to_string(),
            },
        );

        storage.set_profile(&profile).unwrap();
        assert!(storage.profile_exists("test-profile").unwrap());

        // Delete profile
        storage.delete_profile("test-profile").unwrap();
        assert!(!storage.profile_exists("test-profile").unwrap());

        // Deleting non-existent profile should fail
        assert!(storage.delete_profile("non-existent").is_err());
    }

    #[test]
    fn test_profile_validation_on_set() {
        let (storage, _temp) = create_test_storage();

        // Invalid profile (empty name)
        let profile = AuthProfile::new(
            "".to_string(),
            ProfileType::Bearer,
            "https://api.example.com".to_string(),
            Credentials::Bearer {
                token: "test-token".to_string(),
            },
        );

        // Should fail validation
        assert!(storage.set_profile(&profile).is_err());
    }

    #[test]
    fn test_profile_with_description_and_metadata() {
        let (storage, _temp) = create_test_storage();

        let profile = AuthProfile::new(
            "test-profile".to_string(),
            ProfileType::Bearer,
            "https://api.example.com".to_string(),
            Credentials::Bearer {
                token: "test-token".to_string(),
            },
        )
        .with_description("Test profile description".to_string())
        .with_metadata("environment".to_string(), "production".to_string());

        storage.set_profile(&profile).unwrap();

        let retrieved = storage.get_profile("test-profile").unwrap().unwrap();
        assert_eq!(
            retrieved.description,
            Some("Test profile description".to_string())
        );
        assert_eq!(
            retrieved.metadata.get("environment"),
            Some(&"production".to_string())
        );
    }
}
