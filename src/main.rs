use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;

mod adapters;
mod auth;
mod cache;
mod output;

use adapters::{Adapter, ProtocolDetector};
use auth::{AuthType, Profile, Profiles};
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
    List,

    /// Show information about a specific profile
    Info {
        /// Profile name
        #[arg(value_name = "PROFILE")]
        profile: String,
    },

    /// Set or update an authentication profile
    Set {
        /// Profile name
        #[arg(value_name = "PROFILE")]
        profile: String,

        /// API key or token
        #[arg(long)]
        api_key: String,

        /// Authentication type (bearer, api_key, basic)
        #[arg(short = 't', long, default_value = "bearer")]
        auth_type: String,

        /// Profile description
        #[arg(long)]
        description: Option<String>,
    },

    /// Remove an authentication profile
    Remove {
        /// Profile name
        #[arg(value_name = "PROFILE")]
        profile: String,
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

    // Create cache configuration for all commands
    let cache_config = if cli.no_cache {
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

    // Handle cache commands first (they don't require a URL)
    if let Commands::Cache { cache_command } = &cli.command {
        return handle_cache_command(cache_command, cache_config).await;
    }

    // Handle auth commands (they don't require a URL)
    if let Commands::Auth { auth_command } = &cli.command {
        return handle_auth_command(auth_command).await;
    }

    // All other commands require a URL
    let url = cli.url.ok_or_else(|| anyhow::anyhow!("URL is required"))?;

    info!("UXC v0.1.0 - connecting to {}", url);

    // Create cache instance with configuration
    let cache = cache::create_cache(cache_config)?;

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

async fn handle_cache_command(command: &CacheCommands, cache_config: CacheConfig) -> Result<()> {
    let cache = cache::create_cache(cache_config)?;

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

async fn handle_auth_command(command: &AuthCommands) -> Result<()> {
    match command {
        AuthCommands::List => {
            let profiles = Profiles::load_profiles()?;

            if profiles.count() == 0 {
                println!("No profiles found.");
                println!("\nCreate a profile with: uxc auth set <profile> --api-key <key>");
                return Ok(());
            }

            println!("Authentication Profiles:");
            println!();

            for name in profiles.profile_names() {
                let profile = profiles.get_profile(&name)?;
                println!("  {}", name);
                println!("    Type: {}", profile.auth_type);
                println!("    API Key: {}", profile.mask_api_key());
                if let Some(desc) = &profile.description {
                    println!("    Description: {}", desc);
                }
                println!();
            }
        }
        AuthCommands::Info { profile } => {
            let profiles = Profiles::load_profiles()?;
            let profile_data = profiles.get_profile(profile)?;

            println!("Profile: {}", profile);
            println!("  Type: {}", profile_data.auth_type);
            println!("  API Key: {}", profile_data.mask_api_key());
            if let Some(desc) = &profile_data.description {
                println!("  Description: {}", desc);
            }
        }
        AuthCommands::Set {
            profile,
            api_key,
            auth_type,
            description,
        } => {
            let auth_type = auth_type
                .parse::<AuthType>()
                .map_err(|e| anyhow::anyhow!("Invalid auth type: {}", e))?;

            // Clone api_key for display before moving it
            let api_key_display = api_key.clone();
            let auth_type_display = auth_type.clone();

            let mut profile_obj = Profile::new(api_key.clone(), auth_type);
            if let Some(desc) = description {
                profile_obj = profile_obj.with_description(desc.clone());
            }

            let mut profiles = Profiles::load_profiles()?;
            profiles.set_profile(profile.clone(), profile_obj)?;
            profiles.save_profiles()?;

            println!("Profile '{}' saved successfully.", profile);
            println!(
                "  API Key: {}",
                Profile::new(api_key_display, auth_type_display).mask_api_key()
            );
        }
        AuthCommands::Remove { profile } => {
            let mut profiles = Profiles::load_profiles()?;

            if !profiles.has_profile(profile) {
                println!("Profile '{}' not found.", profile);
                println!("\nAvailable profiles: {}", profiles.list_names());
                return Ok(());
            }

            profiles.remove_profile(profile)?;
            profiles.save_profiles()?;

            println!("Profile '{}' removed successfully.", profile);
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
