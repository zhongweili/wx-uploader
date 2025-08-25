//! WeChat Public Account Markdown Uploader
//!
//! A command-line tool for uploading markdown files to WeChat public accounts.

use anyhow::{Context, Result};
use clap::Parser;
use wx_uploader::{WxUploader, cli};

#[tokio::main]
async fn main() -> Result<()> {
    // Check if help is requested before clap processes args
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        cli::print_colored_help();
        std::process::exit(0);
    }

    let args = cli::Args::parse();

    // Handle special commands first
    if let Some(config_path) = &args.init_config {
        if let Err(error_msg) = cli::generate_example_config(config_path).await {
            eprintln!("Error: {}", error_msg);
            std::process::exit(1);
        }
        return Ok(());
    }

    if args.list_accounts {
        if let Some(config_path) = &args.config_file {
            if let Err(error_msg) = cli::list_accounts_from_config(config_path).await {
                eprintln!("Error: {}", error_msg);
                std::process::exit(1);
            }
        } else {
            eprintln!("Error: --list-accounts requires a configuration file (--config)");
            std::process::exit(1);
        }
        return Ok(());
    }

    // Validate arguments
    if let Err(error_msg) = cli::validate_args(&args) {
        eprintln!("Error: {}", error_msg);
        std::process::exit(1);
    }

    // Initialize logging
    cli::init_logging(args.verbose);

    // Create configuration from CLI arguments (handles both env vars and config files)
    let config = cli::create_config_from_args(&args)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create configuration: {}", e))?;
    
    // Display banner if verbose
    cli::display_banner(&args);

    // Create the uploader
    let uploader = WxUploader::new(config)
        .await
        .context("Failed to initialize WeChat uploader")?;
    
    if args.verbose {
        println!("Using account: {} ({})", 
            uploader.current_account().name,
            uploader.current_account().description.as_deref().unwrap_or("No description")
        );
    }

    // Clear cache and refresh token if requested
    if args.clear_cache {
        if args.verbose {
            println!("Refreshing WeChat access token...");
        }
        uploader
            .refresh_token()
            .await
            .context("Failed to refresh WeChat token")?;
        if !args.verbose {
            println!("WeChat access token refreshed");
        }
    }

    // Process the input path
    if let Some(path) = &args.path {
        if path.is_file() {
            // Force upload single file
            uploader
                .upload_file(path, true)
                .await
                .with_context(|| format!("Failed to upload file: {}", path.display()))?;
        } else if path.is_dir() {
            // Process directory
            uploader
                .process_directory(path)
                .await
                .with_context(|| format!("Failed to process directory: {}", path.display()))?;
        } else {
            anyhow::bail!("Path must be a file or directory: {}", path.display());
        }
    } else {
        anyhow::bail!("No path specified for upload operation");
    }

    Ok(())
}
