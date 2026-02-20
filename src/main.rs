use clap::{Parser, Subcommand};
use uxc::ProtocolRouter;
use anyhow::Result;
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "uxc")]
#[command(about = "Universal X-Protocol Call", long_about = None)]
#[command(version = "0.1.0")]
struct Cli {
    /// Remote endpoint URL
    #[arg(value_name = "URL")]
    url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Detect protocol type
    Detect {
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    let cli = Cli::parse();

    info!("UXC v0.1.0 - connecting to {}", cli.url);

    match cli.command {
        Commands::Detect { verbose } => {
            handle_detect(&cli.url, verbose).await?;
        }
        Commands::List { verbose } => {
            handle_list(&cli.url, verbose).await?;
        }
        Commands::Call { operation, args, json, help } => {
            if help {
                handle_help(&cli.url, &operation).await?;
            } else {
                handle_call(&cli.url, &operation, args, json).await?;
            }
        }
        Commands::Inspect { full } => {
            handle_inspect(&cli.url, full).await?;
        }
    }

    Ok(())
}

async fn handle_detect(url: &str, verbose: bool) -> Result<()> {
    println!("Detecting protocol for {}...", url);

    let router = ProtocolRouter::new();
    let start = std::time::Instant::now();

    match router.detect_protocol(url).await {
        Ok(protocol) => {
            let elapsed = start.elapsed();
            println!("✓ Detected: {}", protocol.as_str());
            if verbose {
                println!("  Detection time: {:.2}s", elapsed.as_secs_f64());
            }
            Ok(())
        }
        Err(e) => {
            let elapsed = start.elapsed();
            error!("✗ Detection failed: {}", e);
            if verbose {
                println!("  Time elapsed: {:.2}s", elapsed.as_secs_f64());
            }
            Err(e.into())
        }
    }
}

async fn handle_list(url: &str, verbose: bool) -> Result<()> {
    println!("Listing operations for {}", url);

    let router = ProtocolRouter::new();

    // First detect the protocol
    let protocol = router.detect_protocol(url).await?;

    if verbose {
        println!("Detected protocol: {}", protocol.as_str());
    }

    // Get the adapter and list operations
    let adapter = router
        .get_adapter_for_url(url)
        .await?;

    let operations = adapter.list_operations(url).await?;

    println!("\nAvailable operations ({}):", operations.len());
    for op in operations {
        println!("  - {}", op.name);
        if let Some(desc) = &op.description {
            println!("    {}", desc);
        }
    }

    Ok(())
}

async fn handle_call(url: &str, operation: &str, args: Vec<String>, json: Option<String>) -> Result<()> {
    println!("Calling {} on {}", operation, url);

    let router = ProtocolRouter::new();
    let adapter = router.get_adapter_for_url(url).await?;

    // Parse arguments
    use serde_json::Value;
    let mut parsed_args = std::collections::HashMap::new();

    if let Some(json_str) = json {
        let json_value: Value = serde_json::from_str(&json_str)?;
        if let Some(obj) = json_value.as_object() {
            for (k, v) in obj {
                parsed_args.insert(k.clone(), v.clone());
            }
        }
    } else {
        for arg in args {
            let parts: Vec<&str> = arg.splitn(2, '=').collect();
            if parts.len() == 2 {
                parsed_args.insert(parts[0].to_string(), Value::String(parts[1].to_string()));
            }
        }
    }

    let result = adapter.execute(url, operation, parsed_args).await?;

    println!("\nResult:");
    println!("{}", serde_json::to_string_pretty(&result.data)?);
    println!("\nMetadata:");
    println!("  Operation: {}", result.metadata.operation);
    println!("  Duration: {}ms", result.metadata.duration_ms);

    Ok(())
}

async fn handle_help(url: &str, operation: &str) -> Result<()> {
    println!("Showing help for {} on {}", operation, url);

    let router = ProtocolRouter::new();
    let adapter = router.get_adapter_for_url(url).await?;

    let help_text = adapter.operation_help(url, operation).await?;
    println!("\n{}", help_text);

    Ok(())
}

async fn handle_inspect(url: &str, full: bool) -> Result<()> {
    println!("Inspecting {}", url);

    let router = ProtocolRouter::new();
    let adapter = router.get_adapter_for_url(url).await?;

    let schema = adapter.fetch_schema(url).await?;

    if full {
        let schema_str: String = serde_json::to_string_pretty(&schema)?;
        println!("{}", schema_str);
    } else {
        println!("Schema retrieved successfully");
        println!("Use --full to see complete schema");
    }

    Ok(())
}
