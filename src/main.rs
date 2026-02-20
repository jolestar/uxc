use clap::{Parser, Subcommand};
use anyhow::Result;
use tracing::{info, error};

mod adapters;
mod output;

use adapters::{Adapter, ProtocolDetector, AdapterEnum};
use output::OutputEnvelope;

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

async fn handle_list(url: &str, verbose: bool) -> Result<()> {
    let detector = ProtocolDetector::new();
    let adapter = detector.detect_adapter(url).await?;

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
                    println!("    - {} ({}){}", param.name, param.param_type,
                        if param.required { " required" } else { "" });
                }
            }
        }
    }

    Ok(())
}

async fn handle_call(url: &str, operation: &str, args: Vec<String>, json: Option<String>) -> Result<()> {
    let detector = ProtocolDetector::new();
    let adapter = detector.detect_adapter(url).await?;

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

async fn handle_help(url: &str, operation: &str) -> Result<()> {
    let detector = ProtocolDetector::new();
    let adapter = detector.detect_adapter(url).await?;

    let help_text = adapter.operation_help(url, operation).await?;
    println!("{}", help_text);
    Ok(())
}

async fn handle_inspect(url: &str, full: bool) -> Result<()> {
    let detector = ProtocolDetector::new();
    let adapter = detector.detect_adapter(url).await?;

    println!("Protocol: {:?}", adapter.protocol_type());
    println!("Endpoint: {}", url);

    if full {
        let schema = adapter.fetch_schema(url).await?;
        println!("\nSchema:\n{}", serde_json::to_string_pretty(&schema)?);
    }

    Ok(())
}
