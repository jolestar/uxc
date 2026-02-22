//! Authentication profile data structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Authentication profile containing credentials for an endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthProfile {
    /// Profile name (unique identifier)
    pub name: String,

    /// Profile description (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Profile type
    #[serde(rename = "type")]
    pub profile_type: ProfileType,

    /// Endpoint URL this profile is for
    pub endpoint: String,

    /// Authentication credentials
    #[serde(flatten)]
    pub credentials: Credentials,

    /// Additional metadata
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub metadata: HashMap<String, String>,
}

impl AuthProfile {
    /// Create a new authentication profile
    pub fn new(
        name: String,
        profile_type: ProfileType,
        endpoint: String,
        credentials: Credentials,
    ) -> Self {
        Self {
            name,
            description: None,
            profile_type,
            endpoint,
            credentials,
            metadata: HashMap::default(),
        }
    }

    /// Create a profile with a description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Add metadata to the profile
    #[allow(dead_code)]
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Validate the profile
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Profile name cannot be empty".to_string());
        }

        if self.endpoint.is_empty() {
            return Err("Endpoint cannot be empty".to_string());
        }

        // Validate endpoint URL format
        if !self.endpoint.starts_with("http://")
            && !self.endpoint.starts_with("https://")
            && !self.endpoint.starts_with("grpc://")
            && !self.endpoint.starts_with("ws://")
            && !self.endpoint.starts_with("wss://")
        {
            return Err(format!("Invalid endpoint URL: {}", self.endpoint));
        }

        // Validate credentials based on profile type
        self.credentials.validate(&self.profile_type)?;

        Ok(())
    }
}

/// Profile type determines the authentication method
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProfileType {
    /// No authentication (anonymous access)
    None,

    /// Bearer token authentication
    Bearer,

    /// Basic authentication (username/password)
    Basic,

    /// API key authentication
    ApiKey,

    /// OAuth2 client credentials
    OAuth2,

    /// Custom authentication
    Custom,
}

/// Authentication credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Credentials {
    /// No credentials
    #[serde(rename = "none")]
    None,

    /// Bearer token
    Bearer {
        /// Access token
        token: String,
    },

    /// Basic authentication
    Basic {
        /// Username
        username: String,
        /// Password
        password: String,
    },

    /// API key authentication
    ApiKey {
        /// Key name (e.g., "X-API-Key")
        key_name: String,
        /// Key value
        key_value: String,
        /// Location (header or query)
        #[serde(default = "default_key_location")]
        location: String,
    },

    /// OAuth2 client credentials
    OAuth2 {
        /// Client ID
        client_id: String,
        /// Client secret
        client_secret: String,
        /// Token URL
        token_url: String,
        /// Scope (optional)
        #[serde(skip_serializing_if = "Option::is_none")]
        scope: Option<String>,
    },

    /// Custom authentication (free-form key-value pairs)
    #[serde(rename = "custom")]
    Custom(HashMap<String, String>),
}

fn default_key_location() -> String {
    "header".to_string()
}

impl Credentials {
    /// Validate credentials based on profile type
    #[allow(dead_code)]
    pub fn validate(&self, profile_type: &ProfileType) -> Result<(), String> {
        match (profile_type, self) {
            (ProfileType::None, Credentials::None) => Ok(()),
            (ProfileType::Bearer, Credentials::Bearer { token }) => {
                if token.is_empty() {
                    return Err("Bearer token cannot be empty".to_string());
                }
                Ok(())
            }
            (ProfileType::Basic, Credentials::Basic { username, password }) => {
                if username.is_empty() {
                    return Err("Username cannot be empty".to_string());
                }
                if password.is_empty() {
                    return Err("Password cannot be empty".to_string());
                }
                Ok(())
            }
            (
                ProfileType::ApiKey,
                Credentials::ApiKey {
                    key_name,
                    key_value,
                    ..
                },
            ) => {
                if key_name.is_empty() {
                    return Err("API key name cannot be empty".to_string());
                }
                if key_value.is_empty() {
                    return Err("API key value cannot be empty".to_string());
                }
                Ok(())
            }
            (
                ProfileType::OAuth2,
                Credentials::OAuth2 {
                    client_id,
                    client_secret,
                    token_url,
                    ..
                },
            ) => {
                if client_id.is_empty() {
                    return Err("Client ID cannot be empty".to_string());
                }
                if client_secret.is_empty() {
                    return Err("Client secret cannot be empty".to_string());
                }
                if token_url.is_empty() {
                    return Err("Token URL cannot be empty".to_string());
                }
                Ok(())
            }
            (ProfileType::Custom, Credentials::Custom(map)) => {
                if map.is_empty() {
                    return Err("Custom credentials cannot be empty".to_string());
                }
                Ok(())
            }
            _ => Err(format!(
                "Credentials {:?} do not match profile type {:?}",
                self, profile_type
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_profile_bearer() {
        let profile = AuthProfile::new(
            "test-profile".to_string(),
            ProfileType::Bearer,
            "https://api.example.com".to_string(),
            Credentials::Bearer {
                token: "test-token-123".to_string(),
            },
        );

        assert_eq!(profile.name, "test-profile");
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn test_auth_profile_basic() {
        let profile = AuthProfile::new(
            "basic-profile".to_string(),
            ProfileType::Basic,
            "https://api.example.com".to_string(),
            Credentials::Basic {
                username: "user@example.com".to_string(),
                password: "secret123".to_string(),
            },
        );

        assert_eq!(profile.name, "basic-profile");
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn test_auth_profile_api_key() {
        let profile = AuthProfile::new(
            "api-key-profile".to_string(),
            ProfileType::ApiKey,
            "https://api.example.com".to_string(),
            Credentials::ApiKey {
                key_name: "X-API-Key".to_string(),
                key_value: "my-api-key".to_string(),
                location: "header".to_string(),
            },
        );

        assert_eq!(profile.name, "api-key-profile");
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn test_auth_profile_validation() {
        // Test empty name
        let profile = AuthProfile::new(
            "".to_string(),
            ProfileType::Bearer,
            "https://api.example.com".to_string(),
            Credentials::Bearer {
                token: "test-token".to_string(),
            },
        );
        assert!(profile.validate().is_err());

        // Test invalid endpoint
        let profile = AuthProfile::new(
            "test".to_string(),
            ProfileType::Bearer,
            "not-a-url".to_string(),
            Credentials::Bearer {
                token: "test-token".to_string(),
            },
        );
        assert!(profile.validate().is_err());

        // Test empty token
        let profile = AuthProfile::new(
            "test".to_string(),
            ProfileType::Bearer,
            "https://api.example.com".to_string(),
            Credentials::Bearer {
                token: "".to_string(),
            },
        );
        assert!(profile.validate().is_err());
    }

    #[test]
    fn test_profile_with_description() {
        let profile = AuthProfile::new(
            "test".to_string(),
            ProfileType::Bearer,
            "https://api.example.com".to_string(),
            Credentials::Bearer {
                token: "test-token".to_string(),
            },
        )
        .with_description("Test profile".to_string());

        assert_eq!(profile.description, Some("Test profile".to_string()));
    }

    #[test]
    fn test_profile_with_metadata() {
        let profile = AuthProfile::new(
            "test".to_string(),
            ProfileType::Bearer,
            "https://api.example.com".to_string(),
            Credentials::Bearer {
                token: "test-token".to_string(),
            },
        )
        .with_metadata("environment".to_string(), "production".to_string());

        assert_eq!(
            profile.metadata.get("environment"),
            Some(&"production".to_string())
        );
    }
}
