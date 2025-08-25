//! Command-line interface for the wx-uploader application
//!
//! This module handles argument parsing, colored help display, and CLI-specific
//! functionality for the WeChat uploader tool.

use clap::Parser;
use colored::*;
use std::path::PathBuf;
use crate::models::{Config, ConfigFile, WeChatAccount, AiProviderConfig, GlobalSettings};

/// Command-line arguments for the wx-uploader application
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "A tool to upload articles to WeChat Official Account with multi-account support and AI-powered cover generation",
    long_about = "wx-uploader uploads Markdown files to WeChat public accounts with automatic AI cover generation.

CONFIGURATION:
  Environment Variables (single account):
    WECHAT_APP_ID, WECHAT_APP_SECRET (required)
    OPENAI_API_KEY or GEMINI_API_KEY (optional for AI covers)
    AI_PROVIDER=openai|gemini (optional, defaults to openai)

  Configuration Files (multi-account):
    Use --init-config to generate example YAML/JSON config file
    Use --config to specify config file path
    Use --account to select account from config file

EXAMPLES:
  Basic usage:
    wx-uploader ./posts                    # Upload directory using env vars
    wx-uploader ./article.md               # Upload single file

  Multi-account setup:
    wx-uploader --init-config config.yaml # Generate config template  
    wx-uploader -c config.yaml --list-accounts  # List available accounts
    wx-uploader -c config.yaml -a work ./posts  # Upload using 'work' account

  AI provider override:
    wx-uploader --provider gemini ./posts  # Use Gemini instead of OpenAI
    wx-uploader --ai-key custom_key ./posts # Override API key

SUPPORTED THEMES: default, lapis, maize, orangeheart, phycat, pie, purple, rainbow
SUPPORTED HIGHLIGHTERS: github, github-dark, vscode, atom-one-dark, atom-one-light, 
                        monokai, solarized-dark, solarized-light, vs, vs2015",
    override_help = "Run with --help to see colored help",
    color = clap::ColorChoice::Always,
    styles = clap::builder::Styles::styled()
        .header(clap::builder::styling::AnsiColor::Yellow.on_default())
        .usage(clap::builder::styling::AnsiColor::Green.on_default())
        .literal(clap::builder::styling::AnsiColor::Green.on_default())
        .placeholder(clap::builder::styling::AnsiColor::Green.on_default())
)]
pub struct Args {
    /// Path to markdown file or directory to upload
    #[arg(
        help = "Path to markdown file or directory to upload\n\
                • Files: uploaded regardless of published status\n\
                • Directories: skip files with published: true"
    )]
    pub path: Option<PathBuf>,

    /// Enable verbose logging with detailed tracing information
    #[arg(
        short,
        long,
        help = "Enable verbose logging with detailed tracing information"
    )]
    pub verbose: bool,

    /// Force refresh WeChat access token before operation
    #[arg(
        short = 'r',
        long = "refresh",
        help = "Force refresh WeChat access token before operation. This gets a new token from WeChat API."
    )]
    pub clear_cache: bool,

    /// AI provider to use for cover image generation
    #[arg(
        long = "provider",
        help = "AI provider for cover image generation\n\
                • openai: GPT-4o-mini + DALL-E 3 (default)\n\
                • gemini: Gemini 2.5 Flash + Imagen\n\
                Can override AI_PROVIDER env var or config file setting",
        value_name = "PROVIDER"
    )]
    pub ai_provider: Option<String>,

    /// AI API key (overrides environment variables and config file)
    #[arg(
        long = "ai-key",
        help = "AI API key for the selected provider\n\
                Overrides OPENAI_API_KEY/GEMINI_API_KEY env vars and config file",
        value_name = "KEY"
    )]
    pub ai_api_key: Option<String>,

    /// Configuration file path for multi-account setup
    #[arg(
        short = 'c',
        long = "config",
        help = "Path to configuration file for multi-account setup\n\
                • Supports YAML (.yaml, .yml) and JSON (.json) formats\n\
                • Contains multiple WeChat accounts, AI settings, and global options\n\
                • Use --init-config to generate example file",
        value_name = "FILE"
    )]
    pub config_file: Option<PathBuf>,

    /// WeChat account name to use from config file
    #[arg(
        short = 'a',
        long = "account",
        help = "WeChat account name to use from configuration file\n\
                • Must exist in the accounts section of config file\n\
                • If omitted, uses default_account from config\n\
                • Use --list-accounts to see available accounts",
        value_name = "NAME"
    )]
    pub account: Option<String>,

    /// List all available WeChat accounts from config file
    #[arg(
        long = "list-accounts",
        help = "List all available WeChat accounts from config file and exit\n\
                Shows account names, descriptions, and app IDs\n\
                Requires --config parameter",
        conflicts_with = "path"
    )]
    pub list_accounts: bool,

    /// Generate example configuration file template
    #[arg(
        long = "init-config",
        help = "Generate example configuration file template and exit\n\
                • Creates YAML or JSON file with placeholder values\n\
                • Includes examples for multiple accounts and AI providers\n\
                • Edit the generated file with your actual credentials",
        value_name = "FILE",
        conflicts_with = "path"
    )]
    pub init_config: Option<PathBuf>,
}

/// Print colored help message with detailed information about usage and features
pub fn print_colored_help() {
    // Force colors to be enabled
    colored::control::set_override(true);

    println!(
        "{}",
        "A tool to upload articles to WeChat Official Account with multi-account support"
            .bright_white()
            .bold()
    );
    println!();
    println!(
        "{}: {} [OPTIONS] [PATH]",
        "Usage".bright_green().bold(),
        "wx-uploader".bright_cyan()
    );
    println!();

    println!("{}", "Arguments:".bright_yellow().bold());
    println!(
        "  {}    Path to markdown file or directory to upload (optional for some commands)",
        "[PATH]".bright_cyan()
    );
    println!(
        "            {} Files: uploaded regardless of published status",
        "•".bright_white()
    );
    println!(
        "            {} Directories: skip files with published: true",
        "•".bright_white()
    );
    println!();

    println!("{}", "Options:".bright_yellow().bold());
    println!();
    println!("  {}", "BASIC OPTIONS:".bright_white());
    println!(
        "    {}, {}       Enable verbose logging with detailed tracing",
        "-v".bright_cyan(),
        "--verbose".bright_cyan()
    );
    println!(
        "    {}, {}       Force refresh WeChat access token before operation",
        "-r".bright_cyan(),
        "--refresh".bright_cyan()
    );
    println!(
        "    {}, {}          Print help information",
        "-h".bright_cyan(),
        "--help".bright_cyan()
    );
    println!(
        "    {}, {}       Print version information",
        "-V".bright_cyan(),
        "--version".bright_cyan()
    );
    println!();

    println!("  {}", "MULTI-ACCOUNT OPTIONS:".bright_white());
    println!(
        "    {}, {} {}    Configuration file path (YAML or JSON)",
        "-c".bright_cyan(),
        "--config".bright_cyan(),
        "<FILE>".bright_green()
    );
    println!(
        "    {}, {} {}  WeChat account name from config file",
        "-a".bright_cyan(),
        "--account".bright_cyan(),
        "<NAME>".bright_green()
    );
    println!(
        "    {}       List available accounts from config file",
        "--list-accounts".bright_cyan()
    );
    println!(
        "    {} {} Generate example configuration file",
        "--init-config".bright_cyan(),
        "<FILE>".bright_green()
    );
    println!();

    println!("  {}", "AI PROVIDER OPTIONS:".bright_white());
    println!(
        "    {} {}  AI provider for cover generation (openai, gemini)",
        "--provider".bright_cyan(),
        "<PROVIDER>".bright_green()
    );
    println!(
        "    {} {}     AI API key (overrides env vars and config)",
        "--ai-key".bright_cyan(),
        "<KEY>".bright_green()
    );
    println!();

    println!("{}", "CONFIGURATION:".bright_magenta().bold());
    println!();
    println!("  {}", "Environment Variables (single account):".bright_white());
    println!("    {} {}      WeChat application ID", "WECHAT_APP_ID".bright_cyan(), "(required)".bright_red());
    println!("    {} {}   WeChat application secret", "WECHAT_APP_SECRET".bright_cyan(), "(required)".bright_red());
    println!("    {} {}    OpenAI API key for cover generation", "OPENAI_API_KEY".bright_cyan(), "(optional)".bright_blue());
    println!("    {} {}     Gemini API key for cover generation", "GEMINI_API_KEY".bright_cyan(), "(optional)".bright_blue());
    println!("    {} {}      AI provider preference (openai|gemini)", "AI_PROVIDER".bright_cyan(), "(optional)".bright_blue());
    println!();

    println!("  {}", "Configuration Files (multi-account):".bright_white());
    println!("    {} Supports YAML (.yaml, .yml) and JSON (.json) formats", "•".bright_white());
    println!("    {} Contains multiple WeChat accounts with individual credentials", "•".bright_white());
    println!("    {} Includes AI provider settings and global options", "•".bright_white());
    println!("    {} Use {} to generate example file", "•".bright_white(), "--init-config".bright_cyan());
    println!();

    println!("{}", "EXAMPLES:".bright_blue().bold());
    println!();
    println!("  {}", "Basic usage:".bright_white());
    println!(
        "    {}                    {} Upload directory using env vars",
        "wx-uploader ./posts".bright_white().bold(),
        "#".bright_black()
    );
    println!(
        "    {}               {} Upload single file",
        "wx-uploader ./article.md".bright_white().bold(),
        "#".bright_black()
    );
    println!(
        "    {}            {} Upload with verbose output",
        "wx-uploader --verbose ./posts".bright_white().bold(),
        "#".bright_black()
    );
    println!();

    println!("  {}", "Multi-account setup:".bright_white());
    println!(
        "    {} {} Generate config template",
        "wx-uploader --init-config config.yaml".bright_white().bold(),
        "#".bright_black()
    );
    println!(
        "    {}    {} List available accounts",
        "wx-uploader -c config.yaml --list-accounts".bright_white().bold(),
        "#".bright_black()
    );
    println!(
        "    {}  {} Upload using 'work' account",
        "wx-uploader -c config.yaml -a work ./posts".bright_white().bold(),
        "#".bright_black()
    );
    println!("    {} {} Upload using default account",
        "wx-uploader -c config.yaml ./posts".bright_white().bold(),
        "#".bright_black()
    );
    println!();

    println!("  {}", "AI provider override:".bright_white());
    println!(
        "    {}        {} Use Gemini instead of OpenAI",
        "wx-uploader --provider gemini ./posts".bright_white().bold(),
        "#".bright_black()
    );
    println!(
        "    {}    {} Override API key",
        "wx-uploader --ai-key custom_key ./posts".bright_white().bold(),
        "#".bright_black()
    );
    println!();

    println!("{}", "FRONTMATTER THEMING:".bright_green().bold());
    println!("  {}", "---".bright_black());
    println!("  {}: \"My Article\"", "title".bright_cyan());
    println!(
        "  {}: \"lapis\"        {} Themes: {}",
        "theme".bright_cyan(),
        "#".bright_black(),
        "default, lapis, maize, orangeheart, phycat, pie, purple, rainbow".white()
    );
    println!(
        "  {}: \"github\"        {} Highlighters: {}",
        "code".bright_cyan(),
        "#".bright_black(),
        "github, github-dark, vscode, atom-one-dark, atom-one-light,".white()
    );
    println!(
        "                        {}",
        "monokai, solarized-dark, solarized-light, vs, vs2015".white()
    );
    println!("  {}: \"draft\"", "published".bright_cyan());
    println!(
        "  {}: \"cover.png\"     {} Auto-generated if missing with AI",
        "cover".bright_cyan(),
        "#".bright_black()
    );
    println!("  {}", "---".bright_black());
    println!();

    println!("{}", "AI PROVIDERS:".bright_magenta().bold());
    println!("  {} OpenAI: GPT-4o-mini + DALL-E 3 (default)", "•".bright_white());
    println!("  {} Gemini: Gemini 2.5 Flash + Imagen", "•".bright_white());
    println!();

    println!("For more information, visit: {}", "https://github.com/tyrchen/wx-uploader".bright_blue());
}

/// Validates command-line arguments
pub fn validate_args(args: &Args) -> Result<(), String> {
    // Skip path validation for special commands
    if args.list_accounts || args.init_config.is_some() {
        return Ok(());
    }

    let path = args.path.as_ref().ok_or("Path is required for upload operations")?;

    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    if !path.is_file() && !path.is_dir() {
        return Err(format!(
            "Path must be a file or directory: {}",
            path.display()
        ));
    }

    // Validate config file if specified
    if let Some(config_file) = &args.config_file {
        if !config_file.exists() {
            return Err(format!(
                "Configuration file does not exist: {}",
                config_file.display()
            ));
        }
        if !config_file.is_file() {
            return Err(format!(
                "Configuration path must be a file: {}",
                config_file.display()
            ));
        }
    }

    // Validate account specification
    if args.account.is_some() && args.config_file.is_none() {
        return Err(
            "Account selection requires a configuration file (--config)".to_string(),
        );
    }

    Ok(())
}

/// Initializes logging based on the verbose flag
pub fn init_logging(verbose: bool) {
    if verbose {
        tracing_subscriber::fmt::init();
    }
}

/// Display startup banner with configuration information
pub fn display_banner(args: &Args) {
    if !args.verbose {
        return;
    }

    println!();
    println!("{}", "wx-uploader".bright_cyan().bold());
    println!("{}", "=".repeat(40).bright_black());
    
    // Only show path info for upload operations
    if !args.list_accounts && args.init_config.is_none() {
        if let Some(path) = &args.path {
            println!("Path: {}", path.display().to_string().bright_white());
            println!(
                "Mode: {}",
                if path.is_file() {
                    "Single file"
                } else {
                    "Directory"
                }
                .bright_green()
            );
        }
    }
    
    // Show configuration source
    if let Some(config_file) = &args.config_file {
        println!(
            "Config: {}",
            config_file.display().to_string().bright_magenta()
        );
    } else {
        println!("Config: {}", "Environment variables".bright_magenta());
    }
    
    println!("Verbose: {}", args.verbose.to_string().bright_blue());
    println!("{}", "=".repeat(40).bright_black());
    println!();
}

/// Generates and saves an example configuration file
pub async fn generate_example_config(path: &PathBuf) -> Result<(), String> {
    let mut example_config = ConfigFile::default();
    
    // Add example accounts
    example_config.accounts.insert(
        "personal".to_string(),
        WeChatAccount {
            name: "personal".to_string(),
            app_id: "your_personal_app_id_here".to_string(),
            app_secret: "your_personal_app_secret_here".to_string(),
            description: Some("Personal WeChat public account".to_string()),
        },
    );
    
    example_config.accounts.insert(
        "work".to_string(),
        WeChatAccount {
            name: "work".to_string(),
            app_id: "your_work_app_id_here".to_string(),
            app_secret: "your_work_app_secret_here".to_string(),
            description: Some("Work WeChat public account".to_string()),
        },
    );
    
    // Set default account
    example_config.default_account = Some("personal".to_string());
    
    // Add AI provider configuration example
    example_config.ai_provider = Some(AiProviderConfig {
        provider: "openai".to_string(),
        api_key: "your_openai_api_key_here".to_string(),
        base_url: None,
    });
    
    // Add global settings
    example_config.settings = Some(GlobalSettings {
        verbose: Some(false),
        default_theme: Some("lapis".to_string()),
        default_code_highlighter: Some("github".to_string()),
    });
    
    // Determine output format based on file extension
    let content = if path.extension().and_then(|s| s.to_str()) == Some("json") {
        serde_json::to_string_pretty(&example_config)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?
    } else {
        // Default to YAML
        serde_yaml::to_string(&example_config)
            .map_err(|e| format!("Failed to serialize YAML: {}", e))?
    };
    
    tokio::fs::write(path, content)
        .await
        .map_err(|e| format!("Failed to write config file: {}", e))?;
    
    println!(
        "{} Generated example configuration file: {}",
        "✓".bright_green(),
        path.display().to_string().bright_cyan()
    );
    println!();
    println!("{}", "Next steps:".bright_yellow().bold());
    println!("  1. Edit the configuration file and replace placeholder values");
    println!("  2. Run: {} to list available accounts", 
        format!("wx-uploader -c {} --list-accounts", path.display()).bright_white());
    println!("  3. Upload files: {}", 
        format!("wx-uploader -c {} -a personal ./articles/", path.display()).bright_white());
    
    Ok(())
}

/// Lists all available accounts from configuration file
pub async fn list_accounts_from_config(config_path: &PathBuf) -> Result<(), String> {
    let config = Config::from_file(config_path, None)
        .await
        .map_err(|e| format!("Failed to load config: {}", e))?;
    
    println!(
        "{} Available WeChat accounts in {}:",
        "📋".bright_blue(),
        config_path.display().to_string().bright_cyan()
    );
    println!();
    
    let accounts = config.list_accounts();
    if accounts.is_empty() {
        println!("{}", "No accounts configured.".bright_red());
        return Ok(());
    }
    
    for account in accounts {
        let is_current = account.name == config.wechat_account.name;
        let marker = if is_current { "●" } else { "○" };
        let color = if is_current { "bright_green" } else { "bright_white" };
        
        match color {
            "bright_green" => {
                println!(
                    "  {} {} {} {}",
                    marker.bright_green(),
                    account.name.bright_green().bold(),
                    "-".bright_black(),
                    account.description.as_deref().unwrap_or("No description").bright_green()
                );
                println!(
                    "    {} App ID: {} (current)",
                    "│".bright_green(),
                    account.app_id.bright_green()
                );
            }
            _ => {
                println!(
                    "  {} {} {} {}",
                    marker.bright_white(),
                    account.name.bright_white().bold(),
                    "-".bright_black(),
                    account.description.as_deref().unwrap_or("No description")
                );
                println!(
                    "    {} App ID: {}",
                    "│".bright_black(),
                    account.app_id.bright_black()
                );
            }
        }
        println!();
    }
    
    println!(
        "{}: Use {} to select an account",
        "Usage".bright_blue().bold(),
        "-a/--account <name>".bright_cyan()
    );
    
    Ok(())
}

/// Creates configuration based on command-line arguments
pub async fn create_config_from_args(args: &Args) -> Result<Config, String> {
    let config = if let Some(config_file) = &args.config_file {
        // Load from configuration file
        Config::from_file(config_file, args.account.as_deref())
            .await
            .map_err(|e| format!("Failed to load configuration: {}", e))?
    } else {
        // Load from environment variables (legacy mode)
        Config::from_env()
            .map_err(|e| format!("Failed to load environment configuration: {}", e))?
    };
    
    // Override AI provider if specified via CLI
    let mut final_config = config;
    if args.ai_provider.is_some() || args.ai_api_key.is_some() {
        let provider = args.ai_provider.as_deref().unwrap_or("openai");
        let api_key = if let Some(key) = &args.ai_api_key {
            key.clone()
        } else {
            // Try to get from environment based on provider
            let env_var = match provider {
                "gemini" => "GEMINI_API_KEY",
                _ => "OPENAI_API_KEY", // Default to OpenAI
            };
            std::env::var(env_var)
                .map_err(|_| format!("AI provider '{}' specified but {} not set", provider, env_var))?
        };
        
        use crate::models::AiProvider;
        final_config.ai_provider = Some(match provider {
            "gemini" => AiProvider::Gemini {
                api_key,
                base_url: None,
            },
            _ => AiProvider::OpenAI {
                api_key,
                base_url: None,
            },
        });
    }
    
    // Override verbose setting
    if args.verbose {
        final_config.verbose = true;
    }
    
    // Validate the final configuration
    final_config.validate()
        .map_err(|e| format!("Configuration validation failed: {}", e))?;
    
    Ok(final_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_validate_args_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        fs::write(&file_path, "test content").unwrap();

        let args = Args {
            path: Some(file_path),
            verbose: false,
            clear_cache: false,
            ai_provider: None,
            ai_api_key: None,
            config_file: None,
            account: None,
            list_accounts: false,
            init_config: None,
        };

        assert!(validate_args(&args).is_ok());
    }

    #[test]
    fn test_validate_args_dir_exists() {
        let temp_dir = TempDir::new().unwrap();
        let args = Args {
            path: Some(temp_dir.path().to_path_buf()),
            verbose: false,
            clear_cache: false,
            ai_provider: None,
            ai_api_key: None,
            config_file: None,
            account: None,
            list_accounts: false,
            init_config: None,
        };

        assert!(validate_args(&args).is_ok());
    }

    #[test]
    fn test_validate_args_path_not_exists() {
        let args = Args {
            path: Some(PathBuf::from("nonexistent/path")),
            verbose: false,
            clear_cache: false,
            ai_provider: None,
            ai_api_key: None,
            config_file: None,
            account: None,
            list_accounts: false,
            init_config: None,
        };

        assert!(validate_args(&args).is_err());
        assert!(validate_args(&args).unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_init_logging_verbose() {
        // This test mainly ensures the function doesn't panic
        // In practice, logging setup would be tested differently
        init_logging(true);
        init_logging(false);
    }

    #[test]
    fn test_display_banner() {
        let temp_dir = TempDir::new().unwrap();
        let args = Args {
            path: Some(temp_dir.path().to_path_buf()),
            verbose: true,
            clear_cache: false,
            ai_provider: None,
            ai_api_key: None,
            config_file: None,
            account: None,
            list_accounts: false,
            init_config: None,
        };

        // This test mainly ensures the function doesn't panic
        display_banner(&args);

        let args = Args {
            path: Some(temp_dir.path().to_path_buf()),
            verbose: false,
            clear_cache: false,
            ai_provider: None,
            ai_api_key: None,
            config_file: None,
            account: None,
            list_accounts: false,
            init_config: None,
        };

        display_banner(&args);
    }

    #[test]
    fn test_args_parsing() {
        // This test verifies the Args structure can be created
        let args = Args {
            path: Some(PathBuf::from("test.md")),
            verbose: true,
            clear_cache: false,
            ai_provider: None,
            ai_api_key: None,
            config_file: None,
            account: None,
            list_accounts: false,
            init_config: None,
        };

        assert_eq!(args.path, Some(PathBuf::from("test.md")));
        assert!(args.verbose);
    }
}
