use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;

mod adapters;
mod auth;
mod cache;
mod output;

use adapters::{Adapter, ProtocolDetector};
use auth::{create_profile_storage, ProfileManager};
use cache::{create_default_cache, CacheConfig};
use output::OutputEnvelope;

#[derive(Parser)]
#[command(name = "uxc")]
#[command(about = "Universal X-Protocol Call", long_about = None)]
#[command(version = "0.1.0")]
struct Cli {
    /// Remote endpoint URL (not used with 'cache' subcommand)
    #[arg(value_name = "URL", global = true)]
    url: Option<String>,

    /// Disable cache for this operation
    #[arg(long, global = true)]
    no_cache: bool,

    /// Cache TTL in seconds
    #[arg(long, global = true)]
    cache_ttl: Option<u64>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List available operations
    List {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },

    /// Execute an operation
    Call {
        /// Operation name (e.g., "user.get")
        #[arg(value_name = "OPERATION")]
        operation: String,

        /// Key-value arguments (e.g., "id=42")
        #[arg(short, long)]
        args: Vec<String>,

        /// JSON input payload
        #[arg(long)]
        json: Option<String>,

        /// Show help for this operation
        #[arg(long, short = 'h')]
        help: bool,
    },

    /// Inspect endpoint/schema
    Inspect {
        /// Show full schema
        #[arg(short, long)]
        full: bool,
    },

    /// Manage schema cache
    Cache {
        #[command(subcommand)]
        cache_command: CacheCommands,
    },

    /// Manage authentication profiles
    Auth {
        #[command(subcommand)]
        auth_command: AuthCommands,
    },
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Show cache statistics
    Stats,

    /// Clear cache entries
    Clear {
        /// Optional URL to clear specific cache entry
        url: Option<String>,

        /// Clear all cached entries
        #[arg(long)]
        all: bool,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// List all authentication profiles
    List {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show information about a specific profile
    Info {
        /// Profile name
        name: String,
    },

    /// Set or update an authentication profile
    ///
    /// WARNING: Passing sensitive credentials (passwords, tokens, API keys) as command-line
    /// arguments is a security risk. They appear in shell history and process listings.
    /// Phase 2 will add interactive mode with secure input prompting.
    Set {
        /// Profile name
        #[arg(short, long)]
        name: String,

        /// Profile type (none, bearer, basic, apikey, oauth2, custom)
        #[arg(short, long)]
        profile_type: String,

        /// Endpoint URL
        #[arg(short, long)]
        endpoint: String,

        /// Bearer token (for bearer type)
        ///
        /// WARNING: This will be visible in shell history and process listings
        #[arg(long)]
        token: Option<String>,

        /// Username (for basic type)
        #[arg(long)]
        username: Option<String>,

        /// Password (for basic type)
        ///
        /// WARNING: This will be visible in shell history and process listings
        #[arg(long)]
        password: Option<String>,

        /// API key name (for apikey type)
        #[arg(long)]
        key_name: Option<String>,

        /// API key value (for apikey type)
        ///
        /// WARNING: This will be visible in shell history and process listings
        #[arg(long)]
        key_value: Option<String>,

        /// API key location (header or query, for apikey type)
        #[arg(long)]
        key_location: Option<String>,

        /// Description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Delete an authentication profile
    Delete {
        /// Profile name
        name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    // Handle cache commands first (they don't require a URL)
    if let Commands::Cache { cache_command } = &cli.command {
        return handle_cache_command(cache_command).await;
    }

    // Handle auth commands (they don't require a URL)
    if let Commands::Auth { auth_command } = &cli.command {
        return handle_auth_command(auth_command).await;
    }

    // All other commands require a URL
    let url = cli.url.ok_or_else(|| anyhow::anyhow!("URL is required"))?;

    info!("UXC v0.1.0 - connecting to {}", url);

    // Create cache configuration
    let _cache_config = if cli.no_cache {
        CacheConfig {
            enabled: false,
            ..Default::default()
        }
    } else if let Some(ttl) = cli.cache_ttl {
        CacheConfig {
            ttl,
            ..Default::default()
        }
    } else {
        // Try to load from file, fall back to defaults
        CacheConfig::load_from_file().unwrap_or_default()
    };

    // Create cache instance
    let cache = create_default_cache()?;

    match cli.command {
        Commands::List { verbose } => {
            handle_list(&url, verbose, cache).await?;
        }
        Commands::Call {
            operation,
            args,
            json,
            help,
        } => {
            if help {
                handle_help(&url, &operation, cache).await?;
            } else {
                handle_call(&url, &operation, args, json, cache).await?;
            }
        }
        Commands::Inspect { full } => {
            handle_inspect(&url, full, cache).await?;
        }
        Commands::Cache { .. } => {
            // Already handled above
            unreachable!();
        }
        Commands::Auth { .. } => {
            // Already handled above
            unreachable!();
        }
    }

    Ok(())
}

async fn handle_cache_command(command: &CacheCommands) -> Result<()> {
    let cache = create_default_cache()?;

    match command {
        CacheCommands::Stats => {
            let stats = cache.stats()?;
            println!("{}", stats.display());
        }
        CacheCommands::Clear { url, all } => {
            if *all {
                cache.clear()?;
                println!("Cache cleared successfully.");
            } else if let Some(url) = url {
                cache.invalidate(url)?;
                println!("Cache entry cleared for: {}", url);
            } else {
                // If no URL specified and --all not set, show usage
                println!("Usage: uxc cache clear <url> OR uxc cache clear --all");
            }
        }
    }

    Ok(())
}

async fn handle_list(
    url: &str,
    verbose: bool,
    cache: std::sync::Arc<dyn cache::Cache>,
) -> Result<()> {
    let detector = ProtocolDetector::new();
    let mut adapter = detector.detect_adapter(url).await?;

    // Inject cache if adapter supports it
    adapter = inject_cache_if_supported(adapter, cache);

    if verbose {
        println!("Detected protocol: {:?}\n", adapter.protocol_type());
    }

    let operations = adapter.list_operations(url).await?;

    for op in operations {
        println!("{}", op.name);
        if verbose {
            if let Some(desc) = &op.description {
                println!("  {}", desc);
            }
            if !op.parameters.is_empty() {
                println!("  Parameters:");
                for param in &op.parameters {
                    println!(
                        "    - {} ({}){}",
                        param.name,
                        param.param_type,
                        if param.required { " required" } else { "" }
                    );
                }
            }
        }
    }

    Ok(())
}

async fn handle_call(
    url: &str,
    operation: &str,
    args: Vec<String>,
    json: Option<String>,
    cache: std::sync::Arc<dyn cache::Cache>,
) -> Result<()> {
    let detector = ProtocolDetector::new();
    let mut adapter = detector.detect_adapter(url).await?;

    // Inject cache if adapter supports it
    adapter = inject_cache_if_supported(adapter, cache);

    // Parse arguments
    let mut args_map = std::collections::HashMap::new();
    if let Some(json_str) = json {
        let value: serde_json::Value = serde_json::from_str(&json_str)?;
        if let Some(obj) = value.as_object() {
            for (k, v) in obj {
                args_map.insert(k.clone(), v.clone());
            }
        }
    } else {
        for arg in args {
            let parts: Vec<&str> = arg.splitn(2, '=').collect();
            if parts.len() == 2 {
                args_map.insert(parts[0].to_string(), serde_json::json!(parts[1]));
            }
        }
    }

    let result = adapter.execute(url, operation, args_map).await?;

    // Output deterministic JSON envelope
    let envelope = OutputEnvelope::success(
        adapter.protocol_type().as_str(),
        url,
        operation,
        result.data,
        result.metadata.duration_ms,
    );

    println!("{}", envelope.to_json()?);
    Ok(())
}

async fn handle_help(
    url: &str,
    operation: &str,
    cache: std::sync::Arc<dyn cache::Cache>,
) -> Result<()> {
    let detector = ProtocolDetector::new();
    let mut adapter = detector.detect_adapter(url).await?;

    // Inject cache if adapter supports it
    adapter = inject_cache_if_supported(adapter, cache);

    let help_text = adapter.operation_help(url, operation).await?;
    println!("{}", help_text);
    Ok(())
}

async fn handle_inspect(
    url: &str,
    full: bool,
    cache: std::sync::Arc<dyn cache::Cache>,
) -> Result<()> {
    let detector = ProtocolDetector::new();
    let mut adapter = detector.detect_adapter(url).await?;

    // Inject cache if adapter supports it
    adapter = inject_cache_if_supported(adapter, cache);

    println!("Protocol: {:?}", adapter.protocol_type());
    println!("Endpoint: {}", url);

    if full {
        let schema = adapter.fetch_schema(url).await?;
        println!("\nSchema:\n{}", serde_json::to_string_pretty(&schema)?);
    }

    Ok(())
}

/// Inject cache into adapter if it supports caching
fn inject_cache_if_supported(
    adapter: adapters::AdapterEnum,
    cache: std::sync::Arc<dyn cache::Cache>,
) -> adapters::AdapterEnum {
    match adapter {
        adapters::AdapterEnum::OpenAPI(a) => adapters::AdapterEnum::OpenAPI(a.with_cache(cache)),
        adapters::AdapterEnum::GraphQL(a) => adapters::AdapterEnum::GraphQL(a.with_cache(cache)),
        adapters::AdapterEnum::GRpc(a) => adapters::AdapterEnum::GRpc(a.with_cache(cache)),
        adapters::AdapterEnum::Mcp(a) => adapters::AdapterEnum::Mcp(a.with_cache(cache)),
    }
}
async fn handle_auth_command(command: &AuthCommands) -> Result<()> {
    use auth::{AuthProfile, Credentials, ProfileType};

    let profile_manager = create_profile_storage()?;

    match command {
        AuthCommands::List { verbose } => {
            let profiles = profile_manager.list_profiles()?;

            if profiles.is_empty() {
                println!("No authentication profiles found.");
                println!("Use 'uxc auth set' to create a new profile.");
            } else {
                println!("Authentication profiles ({}):", profiles.len());
                println!();

                for profile in profiles {
                    if *verbose {
                        println!("Name: {}", profile.name);
                        println!("  Type: {:?}", profile.profile_type);
                        println!("  Endpoint: {}", profile.endpoint);

                        if let Some(desc) = &profile.description {
                            println!("  Description: {}", desc);
                        }

                        if !profile.metadata.is_empty() {
                            println!("  Metadata:");
                            for (key, value) in &profile.metadata {
                                println!("    {}: {}", key, value);
                            }
                        }

                        println!();
                    } else {
                        println!("  {}", profile.name);
                        if let Some(desc) = &profile.description {
                            println!("    - {}", desc);
                        }
                    }
                }
            }
        }

        AuthCommands::Info { name } => {
            match profile_manager.get_profile(name)? {
                Some(profile) => {
                    println!("Profile: {}", profile.name);
                    println!("Type: {:?}", profile.profile_type);
                    println!("Endpoint: {}", profile.endpoint);

                    if let Some(desc) = &profile.description {
                        println!("Description: {}", desc);
                    }

                    if !profile.metadata.is_empty() {
                        println!("\nMetadata:");
                        for (key, value) in &profile.metadata {
                            println!("  {}: {}", key, value);
                        }
                    }

                    // Note: Phase 1 does not encrypt, so we show credentials
                    // Phase 2 will hide sensitive information
                    println!("\nCredentials:");
                    match &profile.credentials {
                        Credentials::None => println!("  None"),
                        Credentials::Bearer { token } => {
                            println!("  Type: Bearer");
                            println!("  Token: {}", token);
                        }
                        Credentials::Basic { username, password } => {
                            println!("  Type: Basic");
                            println!("  Username: {}", username);
                            println!("  Password: {}", password);
                        }
                        Credentials::ApiKey {
                            key_name,
                            key_value,
                            location,
                        } => {
                            println!("  Type: API Key");
                            println!("  Key Name: {}", key_name);
                            println!("  Key Value: {}", key_value);
                            println!("  Location: {}", location);
                        }
                        Credentials::OAuth2 {
                            client_id,
                            token_url,
                            scope,
                            ..
                        } => {
                            println!("  Type: OAuth2");
                            println!("  Client ID: {}", client_id);
                            println!("  Token URL: {}", token_url);
                            if let Some(scope) = scope {
                                println!("  Scope: {}", scope);
                            }
                        }
                        Credentials::Custom(map) => {
                            println!("  Type: Custom");
                            for (key, value) in map {
                                println!("  {}: {}", key, value);
                            }
                        }
                    }
                }
                None => {
                    println!("Profile '{}' not found.", name);
                    println!("Use 'uxc auth set' to create a new profile.");
                }
            }
        }

        AuthCommands::Set {
            name,
            profile_type,
            endpoint,
            token,
            username,
            password,
            key_name,
            key_value,
            key_location,
            description,
        } => {
            // Parse profile type
            let profile_type = match profile_type.to_lowercase().as_str() {
                "none" => ProfileType::None,
                "bearer" => ProfileType::Bearer,
                "basic" => ProfileType::Basic,
                "apikey" | "api-key" => ProfileType::ApiKey,
                "oauth2" | "oauth" => ProfileType::OAuth2,
                "custom" => ProfileType::Custom,
                _ => return Err(anyhow::anyhow!("Invalid profile type: {}", profile_type)),
            };

            // Build credentials based on profile type
            let credentials = match profile_type {
                ProfileType::None => Credentials::None,
                ProfileType::Bearer => {
                    let token = token.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("--token is required for bearer authentication")
                    })?;
                    Credentials::Bearer {
                        token: token.clone(),
                    }
                }
                ProfileType::Basic => {
                    let username = username.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("--username is required for basic authentication")
                    })?;
                    let password = password.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("--password is required for basic authentication")
                    })?;
                    Credentials::Basic {
                        username: username.clone(),
                        password: password.clone(),
                    }
                }
                ProfileType::ApiKey => {
                    let key_name = key_name.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("--key-name is required for apikey authentication")
                    })?;
                    let key_value = key_value.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("--key-value is required for apikey authentication")
                    })?;
                    Credentials::ApiKey {
                        key_name: key_name.clone(),
                        key_value: key_value.clone(),
                        location: key_location.clone().unwrap_or_else(|| "header".to_string()),
                    }
                }
                ProfileType::OAuth2 => {
                    return Err(anyhow::anyhow!(
                        "OAuth2 authentication is not yet supported in Phase 1"
                    ))
                }
                ProfileType::Custom => {
                    return Err(anyhow::anyhow!(
                        "Custom authentication is not yet supported in Phase 1"
                    ))
                }
            };

            // Create profile
            let mut profile =
                AuthProfile::new(name.clone(), profile_type, endpoint.clone(), credentials);

            // Add description if provided
            if let Some(desc) = description {
                profile = profile.with_description(desc.clone());
            }

            // Save profile
            profile_manager.set_profile(&profile)?;

            println!("Profile '{}' saved successfully.", name);

            // Warning about security (Phase 1)
            println!();
            println!("⚠️  WARNING: Credentials are stored in plain text (Phase 1).");
            println!("   Phase 2 will add encryption support for secure credential storage.");
        }

        AuthCommands::Delete { name } => {
            match profile_manager.delete_profile(name) {
                Ok(_) => {
                    println!("Profile '{}' deleted successfully.", name);
                }
                Err(e) => {
                    println!("Error deleting profile: {}", e);
                    println!("Use 'uxc auth list' to see available profiles.");
                }
            }
        }
    }

    Ok(())
}
