use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;

mod adapters;
mod cache;
mod output;

use adapters::{Adapter, ProtocolDetector};
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
