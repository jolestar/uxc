use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::info;

mod adapters;
mod auth;
mod cache;
mod error;
mod output;

use adapters::{Adapter, Operation, OperationDetail, ProtocolDetector};
use auth::{AuthType, Profile, Profiles};
use cache::CacheConfig;
use error::UxcError;
use output::OutputEnvelope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Json,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Json,
    Text,
}

#[derive(Parser)]
#[command(name = "uxc")]
#[command(about = "Universal X-Protocol Call", long_about = None)]
#[command(version = "0.1.0")]
#[command(disable_help_flag = true)]
#[command(disable_help_subcommand = true)]
struct Cli {
    /// Show help
    #[arg(short = 'h', long = "help", global = true)]
    help: bool,

    /// Authentication profile name (default: "default", overrides UXC_PROFILE env var)
    #[arg(long, global = true)]
    profile: Option<String>,

    /// Disable cache for this operation
    #[arg(long, global = true)]
    no_cache: bool,

    /// Cache TTL in seconds
    #[arg(long, global = true)]
    cache_ttl: Option<u64>,

    /// Output format (default: json)
    #[arg(long, value_enum, global = true)]
    format: Option<OutputFormat>,

    /// Use human-readable text output
    #[arg(long, global = true, conflicts_with = "format")]
    text: bool,

    /// Remote endpoint URL (not used with 'cache'/'auth' subcommands)
    #[arg(value_name = "URL", global = true)]
    url: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List available operations
    List {
        /// Show detailed information (text mode only)
        #[arg(short, long)]
        verbose: bool,
    },

    /// Describe one operation in detail
    Describe {
        /// Operation ID (e.g., "get:/users/{id}", "query/user", "ask_question")
        #[arg(value_name = "OPERATION_ID")]
        operation_id: String,
    },

    /// Show endpoint help, or operation help when OPERATION_ID is provided
    Help {
        /// Optional operation ID
        #[arg(value_name = "OPERATION_ID")]
        operation_id: Option<String>,
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

    /// Execute an operation explicitly
    Call {
        /// Operation ID
        #[arg(value_name = "OPERATION_ID")]
        operation_id: String,

        /// Key-value arguments (e.g., "id=42")
        #[arg(short, long)]
        args: Vec<String>,

        /// JSON input payload
        #[arg(long)]
        json: Option<String>,
    },

    /// Dynamic operation execution: `uxc <url> <operation_id> [--json ...] [--args k=v]`
    #[command(external_subcommand)]
    External(Vec<String>),
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

enum EndpointCommand {
    HostHelp,
    List {
        verbose: bool,
    },
    Describe {
        operation_id: String,
    },
    Inspect {
        full: bool,
    },
    Execute {
        operation_id: String,
        args: Vec<String>,
        json: Option<String>,
    },
}

#[derive(Debug, Serialize)]
struct OperationSummary {
    operation_id: String,
    display_name: String,
    summary: Option<String>,
    required: Vec<String>,
    input_shape_hint: String,
    protocol_kind: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    if let Err(err) = run().await {
        if prefers_text_output() {
            eprintln!("{}", err);
        } else {
            let code = error_code(&err);
            let envelope = OutputEnvelope::error(code, &err.to_string());
            match envelope.to_json() {
                Ok(json) => println!("{}", json),
                Err(ser_err) => {
                    eprintln!("failed to serialize error output: {}", ser_err);
                    eprintln!("{}", err);
                }
            }
        }
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let output_mode = if cli.text || cli.format == Some(OutputFormat::Text) {
        OutputMode::Text
    } else {
        OutputMode::Json
    };

    if should_show_global_help(&cli) {
        print_global_help()?;
        return Ok(());
    }

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
        CacheConfig::load_from_file().unwrap_or_default()
    };

    if let Some(Commands::Cache { cache_command }) = &cli.command {
        return handle_cache_command(cache_command, cache_config).await;
    }

    if let Some(Commands::Auth { auth_command }) = &cli.command {
        return handle_auth_command(auth_command).await;
    }

    let url = cli
        .url
        .clone()
        .ok_or_else(|| UxcError::InvalidArguments("URL is required".to_string()))?;

    info!("UXC v0.1.0 - connecting to {}", url);

    let endpoint_command = resolve_endpoint_command(&cli)?;
    let auth_profile = load_auth_profile(cli.profile)?;
    let cache = cache::create_cache(cache_config)?;

    let detector = ProtocolDetector::new();
    let mut adapter = detector.detect_adapter(&url).await?;
    adapter = inject_cache_if_supported(adapter, cache);
    adapter = inject_auth_if_supported(adapter, auth_profile);

    match endpoint_command {
        EndpointCommand::HostHelp => {
            let start = std::time::Instant::now();
            let operations = adapter.list_operations(&url).await?;
            let protocol = adapter.protocol_type().as_str();
            let duration_ms = start.elapsed().as_millis() as u64;

            if output_mode == OutputMode::Json {
                let summaries = operations
                    .iter()
                    .map(|op| to_operation_summary(protocol, op))
                    .collect::<Vec<_>>();
                let data = json!({
                    "operations": summaries,
                    "count": summaries.len(),
                    "next": [
                        "uxc <host> list",
                        "uxc <host> describe <operation_id>",
                        "uxc <host> call <operation_id> --json '{...}'"
                    ]
                });
                print_json(OutputEnvelope::success(
                    "host_help",
                    protocol,
                    &url,
                    None,
                    data,
                    Some(duration_ms),
                ))?;
            } else {
                print_host_help_text(protocol, &url, &operations);
            }
        }
        EndpointCommand::List { verbose } => {
            let start = std::time::Instant::now();
            let operations = adapter.list_operations(&url).await?;
            let protocol = adapter.protocol_type().as_str();
            let duration_ms = start.elapsed().as_millis() as u64;

            if output_mode == OutputMode::Json {
                let summaries = operations
                    .iter()
                    .map(|op| to_operation_summary(protocol, op))
                    .collect::<Vec<_>>();
                let data = json!({"operations": summaries, "count": summaries.len()});
                print_json(OutputEnvelope::success(
                    "operation_list",
                    protocol,
                    &url,
                    None,
                    data,
                    Some(duration_ms),
                ))?;
            } else {
                print_list_text(protocol, &operations, verbose);
            }
        }
        EndpointCommand::Describe { operation_id } => {
            let start = std::time::Instant::now();
            let detail = adapter.describe_operation(&url, &operation_id).await?;
            let protocol = adapter.protocol_type().as_str();
            let duration_ms = start.elapsed().as_millis() as u64;

            if output_mode == OutputMode::Json {
                let data = serde_json::to_value(&detail)?;
                print_json(OutputEnvelope::success(
                    "operation_detail",
                    protocol,
                    &url,
                    Some(&detail.operation_id),
                    data,
                    Some(duration_ms),
                ))?;
            } else {
                print_detail_text(protocol, &url, &detail);
            }
        }
        EndpointCommand::Inspect { full } => {
            let start = std::time::Instant::now();
            let protocol = adapter.protocol_type().as_str();
            let schema = if full {
                Some(adapter.fetch_schema(&url).await?)
            } else {
                None
            };
            let duration_ms = start.elapsed().as_millis() as u64;

            if output_mode == OutputMode::Json {
                let data = json!({
                    "protocol": protocol,
                    "endpoint": url,
                    "schema": schema,
                });
                print_json(OutputEnvelope::success(
                    "inspect_result",
                    protocol,
                    &url,
                    None,
                    data,
                    Some(duration_ms),
                ))?;
            } else {
                println!("Protocol: {}", protocol);
                println!("Endpoint: {}", url);
                if let Some(schema) = schema {
                    println!("\nSchema:\n{}", serde_json::to_string_pretty(&schema)?);
                }
            }
        }
        EndpointCommand::Execute {
            operation_id,
            args,
            json,
        } => {
            let args_map = parse_arguments(args, json)?;
            let result = adapter.execute(&url, &operation_id, args_map).await?;
            let protocol = adapter.protocol_type().as_str();

            if output_mode == OutputMode::Json {
                print_json(OutputEnvelope::success(
                    "call_result",
                    protocol,
                    &url,
                    Some(&operation_id),
                    result.data,
                    Some(result.metadata.duration_ms),
                ))?;
            } else {
                println!("{}", serde_json::to_string_pretty(&result.data)?);
            }
        }
    }

    Ok(())
}

fn should_show_global_help(cli: &Cli) -> bool {
    if cli.help && cli.url.is_none() && cli.command.is_none() {
        return true;
    }

    matches!(cli.command, Some(Commands::Help { operation_id: None })) && cli.url.is_none()
}

fn print_global_help() -> Result<()> {
    let mut cmd = Cli::command();
    cmd.print_help()?;
    println!();
    Ok(())
}

fn resolve_endpoint_command(cli: &Cli) -> Result<EndpointCommand> {
    match &cli.command {
        None => Ok(EndpointCommand::HostHelp),
        Some(Commands::List { verbose }) => Ok(EndpointCommand::List { verbose: *verbose }),
        Some(Commands::Describe { operation_id }) => Ok(EndpointCommand::Describe {
            operation_id: operation_id.clone(),
        }),
        Some(Commands::Help {
            operation_id: Some(operation_id),
        }) => Ok(EndpointCommand::Describe {
            operation_id: operation_id.clone(),
        }),
        Some(Commands::Help { operation_id: None }) => Ok(EndpointCommand::HostHelp),
        Some(Commands::Inspect { full }) => Ok(EndpointCommand::Inspect { full: *full }),
        Some(Commands::Call {
            operation_id,
            args,
            json,
        }) => Ok(EndpointCommand::Execute {
            operation_id: operation_id.clone(),
            args: args.clone(),
            json: json.clone(),
        }),
        Some(Commands::External(tokens)) => parse_external_command(tokens, cli.help),
        Some(Commands::Cache { .. }) | Some(Commands::Auth { .. }) => Err(
            UxcError::InvalidArguments("Internal routing error for cache/auth command".to_string())
                .into(),
        ),
    }
}

fn parse_external_command(tokens: &[String], global_help: bool) -> Result<EndpointCommand> {
    if tokens.is_empty() {
        return Err(UxcError::InvalidArguments("Operation ID is required".to_string()).into());
    }

    let operation_id = tokens[0].clone();

    if global_help {
        return Ok(EndpointCommand::Describe { operation_id });
    }

    if tokens.len() >= 2 && tokens[1] == "help" {
        if tokens.len() > 2 {
            return Err(UxcError::InvalidArguments(
                "Unexpected arguments after '<operation_id> help'".to_string(),
            )
            .into());
        }
        return Ok(EndpointCommand::Describe { operation_id });
    }

    let mut args = Vec::new();
    let mut json_payload = None;
    let mut idx = 1;

    while idx < tokens.len() {
        match tokens[idx].as_str() {
            "-h" | "--help" => {
                return Ok(EndpointCommand::Describe { operation_id });
            }
            "-a" | "--args" => {
                idx += 1;
                let arg = tokens.get(idx).ok_or_else(|| {
                    UxcError::InvalidArguments("Missing value for --args".to_string())
                })?;
                args.push(arg.clone());
            }
            "--json" => {
                idx += 1;
                let payload = tokens.get(idx).ok_or_else(|| {
                    UxcError::InvalidArguments("Missing value for --json".to_string())
                })?;
                json_payload = Some(payload.clone());
            }
            token if token.contains('=') && !token.starts_with('-') => {
                args.push(token.to_string());
            }
            unknown => {
                return Err(UxcError::InvalidArguments(format!(
                    "Unknown argument '{}' for operation '{}'. Use --json or --args",
                    unknown, operation_id
                ))
                .into());
            }
        }

        idx += 1;
    }

    Ok(EndpointCommand::Execute {
        operation_id,
        args,
        json: json_payload,
    })
}

fn parse_arguments(
    args: Vec<String>,
    json_payload: Option<String>,
) -> Result<HashMap<String, Value>> {
    let mut args_map = HashMap::new();

    if let Some(json_str) = json_payload {
        let value: Value = serde_json::from_str(&json_str)
            .map_err(|e| UxcError::InvalidArguments(format!("Invalid JSON payload: {}", e)))?;
        if let Some(obj) = value.as_object() {
            for (k, v) in obj {
                args_map.insert(k.clone(), v.clone());
            }
        } else {
            return Err(
                UxcError::InvalidArguments("JSON payload must be an object".to_string()).into(),
            );
        }
    } else {
        for arg in args {
            let parts: Vec<&str> = arg.splitn(2, '=').collect();
            if parts.len() == 2 {
                args_map.insert(parts[0].to_string(), json!(parts[1]));
            }
        }
    }

    Ok(args_map)
}

fn load_auth_profile(cli_profile: Option<String>) -> Result<Option<Profile>> {
    let (profile_name, profile_explicitly_selected) = if let Some(profile) = cli_profile {
        (profile, true)
    } else if let Ok(profile) = std::env::var("UXC_PROFILE") {
        (profile, true)
    } else {
        ("default".to_string(), false)
    };

    match Profiles::load_profiles() {
        Ok(profiles) => match profiles.get_profile(&profile_name) {
            Ok(profile) => Ok(Some(profile.clone())),
            Err(e) => {
                if !profile_explicitly_selected && profile_name == "default" {
                    info!("No 'default' profile found, continuing without authentication");
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        },
        Err(e) => {
            if !profile_explicitly_selected && profile_name == "default" {
                info!(
                    "Could not load profiles: {}, continuing without authentication",
                    e
                );
                Ok(None)
            } else {
                Err(anyhow::anyhow!(
                    "Failed to load profile '{}': {}. Please run 'uxc auth set {} --api-key <key>' to create it.",
                    profile_name,
                    e,
                    profile_name
                ))
            }
        }
    }
}

fn print_json(envelope: OutputEnvelope) -> Result<()> {
    println!("{}", envelope.to_json()?);
    Ok(())
}

fn print_host_help_text(protocol: &str, endpoint: &str, operations: &[Operation]) {
    println!("Protocol: {}", protocol);
    println!("Endpoint: {}", endpoint);
    println!();
    println!("Available operations:");
    for op in operations {
        if let Some(desc) = &op.description {
            println!("- {} ({}) : {}", op.display_name, op.operation_id, desc);
        } else {
            println!("- {} ({})", op.display_name, op.operation_id);
        }
    }
    println!();
    println!("Next steps:");
    println!("  uxc {} list", endpoint);
    println!("  uxc {} describe <operation_id>", endpoint);
    println!("  uxc {} call <operation_id> --json '{{...}}'", endpoint);
}

fn print_list_text(protocol: &str, operations: &[Operation], verbose: bool) {
    if verbose {
        println!("Detected protocol: {}", protocol);
        println!();
    }

    for op in operations {
        println!("{} ({})", op.display_name, op.operation_id);
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
}

fn print_detail_text(protocol: &str, endpoint: &str, detail: &OperationDetail) {
    println!("Protocol: {}", protocol);
    println!("Endpoint: {}", endpoint);
    println!("Operation ID: {}", detail.operation_id);
    println!("Display Name: {}", detail.display_name);

    if let Some(description) = &detail.description {
        println!("Description: {}", description);
    }

    if let Some(return_type) = &detail.return_type {
        println!("Return Type: {}", return_type);
    }

    if !detail.parameters.is_empty() {
        println!("\nParameters:");
        for param in &detail.parameters {
            println!(
                "- {} ({}){}",
                param.name,
                param.param_type,
                if param.required { " required" } else { "" }
            );
            if let Some(desc) = &param.description {
                println!("  {}", desc);
            }
        }
    }

    if let Some(input_schema) = &detail.input_schema {
        println!(
            "\nInput Schema:\n{}",
            serde_json::to_string_pretty(input_schema).unwrap_or_else(|_| "{}".to_string())
        );
    }
}

fn to_operation_summary(protocol: &str, op: &Operation) -> OperationSummary {
    let required = op
        .parameters
        .iter()
        .filter(|p| p.required)
        .map(|p| p.name.clone())
        .collect::<Vec<_>>();

    let protocol_kind = match protocol {
        "mcp" => "tool",
        "graphql" => {
            if op.operation_id.starts_with("query/") {
                "query"
            } else if op.operation_id.starts_with("mutation/") {
                "mutation"
            } else if op.operation_id.starts_with("subscription/") {
                "subscription"
            } else {
                "field"
            }
        }
        "grpc" => "rpc",
        "openapi" => "http_operation",
        _ => "operation",
    }
    .to_string();

    let input_shape_hint = if op.parameters.is_empty() {
        "none".to_string()
    } else {
        "object".to_string()
    };

    OperationSummary {
        operation_id: op.operation_id.clone(),
        display_name: op.display_name.clone(),
        summary: op.description.clone(),
        required,
        input_shape_hint,
        protocol_kind,
    }
}

fn error_code(err: &anyhow::Error) -> &'static str {
    for cause in err.chain() {
        if let Some(uxc_error) = cause.downcast_ref::<UxcError>() {
            return match uxc_error {
                UxcError::ProtocolDetectionFailed(_) | UxcError::UnsupportedProtocol(_) => {
                    "PROTOCOL_DETECTION_FAILED"
                }
                UxcError::OperationNotFound(_) => "OPERATION_NOT_FOUND",
                UxcError::InvalidArguments(_) => "INVALID_ARGUMENT",
                UxcError::ExecutionFailed(_)
                | UxcError::SchemaRetrievalFailed(_)
                | UxcError::NetworkError(_)
                | UxcError::JsonError(_)
                | UxcError::IoError(_)
                | UxcError::GenericError(_) => "EXECUTION_FAILED",
            };
        }

        if cause.downcast_ref::<serde_json::Error>().is_some() {
            return "INVALID_ARGUMENT";
        }
    }

    "EXECUTION_FAILED"
}

fn prefers_text_output() -> bool {
    let args = std::env::args().collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--text") {
        return true;
    }

    for (idx, arg) in args.iter().enumerate() {
        if arg == "--format" {
            if let Some(value) = args.get(idx + 1) {
                if value == "text" {
                    return true;
                }
            }
        } else if arg == "--format=text" {
            return true;
        }
    }

    false
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

fn inject_auth_if_supported(
    adapter: adapters::AdapterEnum,
    profile: Option<Profile>,
) -> adapters::AdapterEnum {
    match profile {
        Some(profile) => match adapter {
            adapters::AdapterEnum::OpenAPI(a) => {
                adapters::AdapterEnum::OpenAPI(a.with_auth(profile))
            }
            adapters::AdapterEnum::GraphQL(a) => {
                adapters::AdapterEnum::GraphQL(a.with_auth(profile))
            }
            adapters::AdapterEnum::GRpc(a) => adapters::AdapterEnum::GRpc(a.with_auth(profile)),
            adapters::AdapterEnum::Mcp(a) => adapters::AdapterEnum::Mcp(a.with_auth(profile)),
        },
        None => adapter,
    }
}
