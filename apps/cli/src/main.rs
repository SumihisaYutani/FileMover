mod commands;
mod config_manager;
mod progress;
mod error;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, error};
use anyhow::Result;

use crate::commands::*;
use crate::config_manager::ConfigManager;

#[derive(Parser)]
#[command(name = "filemover")]
#[command(about = "Safe folder organization tool for Windows")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Output format (json, pretty, minimal)
    #[arg(short, long, default_value = "pretty")]
    output: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan directories for matching folders
    Scan {
        /// Root directories to scan
        roots: Vec<PathBuf>,
        
        /// Output file for scan results
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Profile name to use
        #[arg(short, long)]
        profile: Option<String>,
    },
    
    /// Create move plan from scan results
    Plan {
        /// Input scan results file
        #[arg(short, long)]
        input: Option<PathBuf>,
        
        /// Output plan file
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Rules configuration file
        #[arg(short, long)]
        rules: Option<PathBuf>,
    },
    
    /// Dry-run simulation of move plan
    DryRun {
        /// Plan file to simulate
        #[arg(short, long)]
        plan: PathBuf,
    },
    
    /// Execute move plan
    Apply {
        /// Plan file to execute
        #[arg(short, long)]
        plan: PathBuf,
        
        /// Journal file for undo operations
        #[arg(short, long)]
        journal: Option<PathBuf>,
        
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    
    /// Undo previous operation
    Undo {
        /// Journal file from previous operation
        #[arg(short, long)]
        journal: PathBuf,
    },
    
    /// Manage configuration profiles
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// List available profiles
    List,
    
    /// Show profile configuration
    Show {
        /// Profile name
        profile: String,
    },
    
    /// Create new profile
    Create {
        /// Profile name
        profile: String,
        
        /// Copy from existing profile
        #[arg(long)]
        from: Option<String>,
    },
    
    /// Delete profile
    Delete {
        /// Profile name
        profile: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    init_logging(cli.verbose)?;
    
    info!("FileMover CLI v{} starting", env!("CARGO_PKG_VERSION"));
    
    // Load configuration
    let config_manager = ConfigManager::new(cli.config.clone())?;
    
    // Execute command
    let result = match cli.command {
        Commands::Scan { roots, output, profile } => {
            scan_command(roots, output, profile, &config_manager).await
        }
        Commands::Plan { input, output, rules } => {
            plan_command(input, output, rules, &config_manager).await
        }
        Commands::DryRun { plan } => {
            dry_run_command(plan, &config_manager).await
        }
        Commands::Apply { plan, journal, yes } => {
            apply_command(plan, journal, yes, &config_manager).await
        }
        Commands::Undo { journal } => {
            undo_command(journal, &config_manager).await
        }
        Commands::Config { action } => {
            config_command(action, &config_manager).await
        }
    };
    
    match result {
        Ok(_) => {
            info!("Command completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Command failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn init_logging(verbose: bool) -> Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    
    let log_level = if verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    tracing_subscriber::EnvFilter::new(
                        format!("filemover_cli={}", log_level)
                    )
                })
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    Ok(())
}