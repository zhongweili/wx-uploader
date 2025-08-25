//! Data models and configuration for the wx-uploader library
//!
//! This module contains core data structures used throughout the application,
//! including configuration, frontmatter parsing, and validation logic.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{env, path::Path, collections::HashMap};

/// AI provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiProvider {
    /// Use OpenAI directly
    OpenAI {
        api_key: String,
        base_url: Option<String>,
    },
    /// Use Google Gemini
    Gemini {
        api_key: String,
        base_url: Option<String>,
    },
}

/// WeChat account configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WeChatAccount {
    /// Account name/identifier
    pub name: String,
    /// WeChat application ID
    pub app_id: String,
    /// WeChat application secret
    pub app_secret: String,
    /// Optional description for this account
    pub description: Option<String>,
}

/// Configuration file structure for multiple accounts and settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigFile {
    /// WeChat accounts configuration
    pub accounts: HashMap<String, WeChatAccount>,
    /// Default account name to use
    pub default_account: Option<String>,
    /// AI provider configuration
    pub ai_provider: Option<AiProviderConfig>,
    /// Global settings
    pub settings: Option<GlobalSettings>,
}

/// AI provider configuration in config file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProviderConfig {
    /// Provider type
    pub provider: String, // "openai" or "gemini"
    /// API key
    pub api_key: String,
    /// Optional base URL
    pub base_url: Option<String>,
}

/// Global settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalSettings {
    /// Enable verbose logging by default
    pub verbose: Option<bool>,
    /// Default theme for articles
    pub default_theme: Option<String>,
    /// Default code highlighter
    pub default_code_highlighter: Option<String>,
}

impl AiProvider {
    /// Create OpenAI provider from API key
    pub fn openai(api_key: String) -> Self {
        Self::OpenAI {
            api_key,
            base_url: None,
        }
    }

    /// Create Gemini provider from API key
    pub fn gemini(api_key: String) -> Self {
        Self::Gemini {
            api_key,
            base_url: None,
        }
    }

    /// Get the API key
    pub fn api_key(&self) -> &str {
        match self {
            AiProvider::OpenAI { api_key, .. } => api_key,
            AiProvider::Gemini { api_key, .. } => api_key,
        }
    }

    /// Get provider name for display
    pub fn name(&self) -> &'static str {
        match self {
            AiProvider::OpenAI { .. } => "OpenAI",
            AiProvider::Gemini { .. } => "Gemini",
        }
    }
}

/// Configuration for the wx-uploader application
///
/// Contains all necessary API keys and settings for WeChat and AI provider integration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Currently selected WeChat account
    pub wechat_account: WeChatAccount,
    /// All available WeChat accounts
    pub available_accounts: HashMap<String, WeChatAccount>,
    /// Optional AI provider configuration for cover image generation
    pub ai_provider: Option<AiProvider>,
    /// Enable verbose logging
    pub verbose: bool,
    /// Configuration file path (if loaded from file)
    pub config_file_path: Option<String>,
}

impl Config {
    /// Creates a new configuration from environment variables (legacy support)
    ///
    /// # Required Environment Variables
    ///
    /// - `WECHAT_APP_ID`: WeChat application ID
    /// - `WECHAT_APP_SECRET`: WeChat application secret
    ///
    /// # Optional Environment Variables
    ///
    /// - `OPENAI_API_KEY`: OpenAI API key for cover image generation (legacy)
    /// - `GEMINI_API_KEY`: Google Gemini API key for cover image generation
    /// - `AI_PROVIDER`: Which provider to use ("openai" or "gemini", defaults to "openai")
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables are not set
    pub fn from_env() -> Result<Self> {
        let wechat_app_id =
            env::var("WECHAT_APP_ID").map_err(|_| Error::missing_env_var("WECHAT_APP_ID"))?;

        let wechat_app_secret = env::var("WECHAT_APP_SECRET")
            .map_err(|_| Error::missing_env_var("WECHAT_APP_SECRET"))?;

        // Create default account from environment variables
        let default_account = WeChatAccount {
            name: "default".to_string(),
            app_id: wechat_app_id,
            app_secret: wechat_app_secret,
            description: Some("Default account from environment variables".to_string()),
        };

        let mut available_accounts = HashMap::new();
        available_accounts.insert("default".to_string(), default_account.clone());

        // Determine AI provider based on environment variables
        let ai_provider = Self::determine_ai_provider_from_env();

        Ok(Self {
            wechat_account: default_account,
            available_accounts,
            ai_provider,
            verbose: false, // Default to false, can be overridden by CLI
            config_file_path: None,
        })
    }

    /// Creates configuration from a configuration file
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the configuration file
    /// * `account_name` - Optional account name to select, uses default if None
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration file cannot be read
    /// - Configuration file has invalid format
    /// - Specified account is not found
    /// - No accounts are configured
    pub async fn from_file<P: AsRef<Path>>(
        config_path: P,
        account_name: Option<&str>,
    ) -> Result<Self> {
        let config_path = config_path.as_ref();
        let config_content = tokio::fs::read_to_string(config_path)
            .await
            .map_err(|e| Error::config(format!("Failed to read config file: {}", e)))?;

        let config_file: ConfigFile = if config_path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::from_str(&config_content)
                .map_err(|e| Error::config(format!("Invalid JSON config: {}", e)))?
        } else {
            // Default to YAML
            serde_yaml::from_str(&config_content)
                .map_err(|e| Error::config(format!("Invalid YAML config: {}", e)))?
        };

        if config_file.accounts.is_empty() {
            return Err(Error::config("No WeChat accounts configured".to_string()));
        }

        // Determine which account to use
        let selected_account_name = account_name
            .or(config_file.default_account.as_deref())
            .or_else(|| config_file.accounts.keys().next().map(|s| s.as_str()))
            .ok_or_else(|| Error::config("No account specified and no default account set".to_string()))?;

        let selected_account = config_file
            .accounts
            .get(selected_account_name)
            .ok_or_else(|| {
                Error::config(format!(
                    "Account '{}' not found in configuration. Available accounts: {}",
                    selected_account_name,
                    config_file.accounts.keys().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                ))
            })?
            .clone();

        // Convert AI provider configuration
        let ai_provider = if let Some(ai_config) = config_file.ai_provider {
            match ai_config.provider.to_lowercase().as_str() {
                "openai" => Some(AiProvider::OpenAI {
                    api_key: ai_config.api_key,
                    base_url: ai_config.base_url,
                }),
                "gemini" => Some(AiProvider::Gemini {
                    api_key: ai_config.api_key,
                    base_url: ai_config.base_url,
                }),
                _ => {
                    return Err(Error::config(format!(
                        "Unsupported AI provider: {}",
                        ai_config.provider
                    )));
                }
            }
        } else {
            // Try environment variables as fallback
            Self::determine_ai_provider_from_env()
        };

        Ok(Self {
            wechat_account: selected_account,
            available_accounts: config_file.accounts,
            ai_provider,
            verbose: config_file.settings.as_ref().and_then(|s| s.verbose).unwrap_or(false),
            config_file_path: Some(config_path.to_string_lossy().to_string()),
        })
    }

    /// Lists all available accounts in the current configuration
    pub fn list_accounts(&self) -> Vec<&WeChatAccount> {
        self.available_accounts.values().collect()
    }

    /// Switches to a different account
    ///
    /// # Arguments
    ///
    /// * `account_name` - Name of the account to switch to
    ///
    /// # Errors
    ///
    /// Returns an error if the account is not found
    pub fn switch_account(&mut self, account_name: &str) -> Result<()> {
        let account = self
            .available_accounts
            .get(account_name)
            .ok_or_else(|| {
                Error::config(format!(
                    "Account '{}' not found. Available accounts: {}",
                    account_name,
                    self.available_accounts.keys().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                ))
            })?
            .clone();

        self.wechat_account = account;
        Ok(())
    }

    /// Determine AI provider from environment variables
    fn determine_ai_provider_from_env() -> Option<AiProvider> {
        let provider_type = env::var("AI_PROVIDER").unwrap_or_else(|_| "openai".to_string()).to_lowercase();
        
        match provider_type.as_str() {
            "gemini" => {
                if let Ok(api_key) = env::var("GEMINI_API_KEY") {
                    Some(AiProvider::gemini(api_key))
                } else {
                    None
                }
            }
            "openai" | _ => {
                // Default to OpenAI, also check legacy OPENAI_API_KEY
                if let Ok(api_key) = env::var("OPENAI_API_KEY") {
                    Some(AiProvider::openai(api_key))
                } else {
                    None
                }
            }
        }
    }

    /// Creates a new configuration with explicit values (single account)
    pub fn new(
        wechat_app_id: String,
        wechat_app_secret: String,
        ai_provider: Option<AiProvider>,
        verbose: bool,
    ) -> Self {
        let account = WeChatAccount {
            name: "main".to_string(),
            app_id: wechat_app_id,
            app_secret: wechat_app_secret,
            description: Some("Main account".to_string()),
        };

        let mut available_accounts = HashMap::new();
        available_accounts.insert("main".to_string(), account.clone());

        Self {
            wechat_account: account,
            available_accounts,
            ai_provider,
            verbose,
            config_file_path: None,
        }
    }

    /// Creates a new configuration with multiple accounts
    pub fn new_with_accounts(
        accounts: HashMap<String, WeChatAccount>,
        default_account_name: &str,
        ai_provider: Option<AiProvider>,
        verbose: bool,
    ) -> Result<Self> {
        let default_account = accounts
            .get(default_account_name)
            .ok_or_else(|| {
                Error::config(format!(
                    "Default account '{}' not found in provided accounts",
                    default_account_name
                ))
            })?
            .clone();

        Ok(Self {
            wechat_account: default_account,
            available_accounts: accounts,
            ai_provider,
            verbose,
            config_file_path: None,
        })
    }

    /// Creates a new configuration with legacy OpenAI key support
    #[deprecated(note = "Use new() with AiProvider instead")]
    pub fn new_with_openai_key(
        wechat_app_id: String,
        wechat_app_secret: String,
        openai_api_key: Option<String>,
        verbose: bool,
    ) -> Self {
        let ai_provider = openai_api_key.map(AiProvider::openai);
        Self::new(wechat_app_id, wechat_app_secret, ai_provider, verbose)
    }

    /// Sets the verbose flag
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Validates the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - WeChat app ID is empty
    /// - WeChat app secret is empty
    /// - No accounts are available
    pub fn validate(&self) -> Result<()> {
        if self.available_accounts.is_empty() {
            return Err(Error::config("No WeChat accounts configured"));
        }

        if self.wechat_account.app_id.trim().is_empty() {
            return Err(Error::config("WeChat app ID cannot be empty"));
        }

        if self.wechat_account.app_secret.trim().is_empty() {
            return Err(Error::config("WeChat app secret cannot be empty"));
        }

        // Validate all accounts
        for (name, account) in &self.available_accounts {
            if account.app_id.trim().is_empty() {
                return Err(Error::config(format!(
                    "Account '{}' has empty app ID",
                    name
                )));
            }
            if account.app_secret.trim().is_empty() {
                return Err(Error::config(format!(
                    "Account '{}' has empty app secret",
                    name
                )));
            }
        }

        Ok(())
    }
}

/// YAML frontmatter structure for markdown files.
///
/// This struct represents the frontmatter that can be present at the beginning
/// of markdown files. It supports common fields like `title` and `published`,
/// and uses `#[serde(flatten)]` to capture any additional fields.
///
/// # Examples
///
/// ```yaml
/// ---
/// title: "My Article"
/// published: "draft"
/// author: "John Doe"
/// tags: ["rust", "wechat"]
/// theme: "lapis"
/// code: "github"
/// cover: "cover.png"
/// ---
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct Frontmatter {
    /// The title of the article. Optional field that will be omitted from
    /// serialization if not present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Publication status of the article.
    ///
    /// Common values:
    /// - `None` or missing: not uploaded
    /// - `"draft"`: uploaded as draft to WeChat
    /// - `"true"`: published (will be skipped in directory mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,

    /// Cover image filename for the article.
    /// If missing, the system will attempt to generate one using AI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,

    /// Theme for the WeChat article styling.
    ///
    /// Available themes: default, lapis, maize, orangeheart, phycat, pie, purple, rainbow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,

    /// Code highlighter for syntax highlighting.
    ///
    /// Available highlighters: github, github-dark, vscode, atom-one-light, atom-one-dark,
    /// solarized-light, solarized-dark, monokai, dracula, xcode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// Description of the article.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// Captures any additional fields in the frontmatter that are not
    /// explicitly defined in this struct.
    #[serde(flatten)]
    pub other: serde_yaml::Value,
}

impl Frontmatter {
    /// Creates a new empty frontmatter
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a frontmatter with a title
    pub fn with_title(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            ..Default::default()
        }
    }

    /// Sets the title
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = Some(title.into());
    }

    /// Sets the published status
    pub fn set_published(&mut self, status: impl Into<String>) {
        self.published = Some(status.into());
    }

    /// Sets the cover image
    pub fn set_cover(&mut self, cover: impl Into<String>) {
        self.cover = Some(cover.into());
    }

    /// Sets the theme
    pub fn set_theme(&mut self, theme: impl Into<String>) {
        self.theme = Some(theme.into());
    }

    /// Sets the code highlighter
    pub fn set_code_highlighter(&mut self, code: impl Into<String>) {
        self.code = Some(code.into());
    }

    /// Checks if the article is published
    pub fn is_published(&self) -> bool {
        // Check the published field first
        if matches!(self.published.as_deref(), Some("true") | Some("\"true\"")) {
            return true;
        }

        // Check if published is a boolean true in the other field
        if let serde_yaml::Value::Mapping(map) = &self.other
            && let Some(serde_yaml::Value::Bool(true)) =
                map.get(serde_yaml::Value::String("published".to_string()))
        {
            return true;
        }

        false
    }

    /// Checks if the article is a draft
    pub fn is_draft(&self) -> bool {
        matches!(self.published.as_deref(), Some("draft"))
    }

    /// Checks if the article is unpublished
    pub fn is_unpublished(&self) -> bool {
        self.published.is_none() || self.published.as_deref() == Some("")
    }

    /// Validates the frontmatter
    pub fn validate(&self) -> Result<()> {
        // Validate theme if present
        if let Some(theme) = &self.theme
            && !is_valid_theme(theme)
        {
            return Err(Error::config(format!(
                "Invalid theme '{}'. Available themes: {}",
                theme,
                VALID_THEMES.join(", ")
            )));
        }

        // Validate code highlighter if present
        if let Some(code) = &self.code
            && !is_valid_code_highlighter(code)
        {
            return Err(Error::config(format!(
                "Invalid code highlighter '{}'. Available highlighters: {}",
                code,
                VALID_CODE_HIGHLIGHTERS.join(", ")
            )));
        }

        Ok(())
    }
}

/// Valid themes for WeChat articles
pub const VALID_THEMES: &[&str] = &[
    "default",
    "lapis",
    "maize",
    "orangeheart",
    "phycat",
    "pie",
    "purple",
    "rainbow",
];

/// Valid code highlighters for syntax highlighting
pub const VALID_CODE_HIGHLIGHTERS: &[&str] = &[
    "github",
    "github-dark",
    "vscode",
    "atom-one-light",
    "atom-one-dark",
    "solarized-light",
    "solarized-dark",
    "monokai",
    "dracula",
    "xcode",
];

/// Checks if a theme is valid
pub fn is_valid_theme(theme: &str) -> bool {
    VALID_THEMES.contains(&theme)
}

/// Checks if a code highlighter is valid
pub fn is_valid_code_highlighter(highlighter: &str) -> bool {
    VALID_CODE_HIGHLIGHTERS.contains(&highlighter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_creation() {
        let ai_provider = Some(AiProvider::openai("test_openai_key".to_string()));
        let config = Config::new(
            "test_app_id".to_string(),
            "test_secret".to_string(),
            ai_provider,
            true,
        );

        assert_eq!(config.wechat_account.app_id, "test_app_id");
        assert_eq!(config.wechat_account.app_secret, "test_secret");
        assert_eq!(config.wechat_account.name, "main");
        assert!(config.ai_provider.is_some());
        assert_eq!(config.ai_provider.as_ref().unwrap().api_key(), "test_openai_key");
        assert!(config.verbose);
        assert_eq!(config.available_accounts.len(), 1);
    }

    #[test]
    fn test_config_with_verbose() {
        let mut config = Config::new(
            "test_app_id".to_string(),
            "test_secret".to_string(),
            None,
            false,
        );
        
        config.verbose = true;
        assert!(config.verbose);
    }

    #[test]
    fn test_config_validation() {
        let valid_config = Config::new("app_id".to_string(), "secret".to_string(), None, false);
        assert!(valid_config.validate().is_ok());

        // Test with empty app_id
        let mut accounts = std::collections::HashMap::new();
        accounts.insert(
            "test".to_string(),
            WeChatAccount {
                name: "test".to_string(),
                app_id: "".to_string(),
                app_secret: "secret".to_string(),
                description: None,
            },
        );
        let empty_app_id = Config::new_with_accounts(accounts, "test", None, false);
        assert!(empty_app_id.is_ok());
        assert!(empty_app_id.unwrap().validate().is_err());

        // Test with empty app_secret
        let mut accounts = std::collections::HashMap::new();
        accounts.insert(
            "test".to_string(),
            WeChatAccount {
                name: "test".to_string(),
                app_id: "app_id".to_string(),
                app_secret: "".to_string(),
                description: None,
            },
        );
        let empty_secret = Config::new_with_accounts(accounts, "test", None, false);
        assert!(empty_secret.is_ok());
        assert!(empty_secret.unwrap().validate().is_err());
    }

    #[test]
    fn test_config_from_env() {
        // Set environment variables
        unsafe {
            env::set_var("WECHAT_APP_ID", "test_id");
            env::set_var("WECHAT_APP_SECRET", "test_secret");
            env::set_var("OPENAI_API_KEY", "test_openai");
        }

        let config = Config::from_env().unwrap();
        assert_eq!(config.wechat_account.app_id, "test_id");
        assert_eq!(config.wechat_account.app_secret, "test_secret");
        assert_eq!(config.wechat_account.name, "default");
        assert!(config.ai_provider.is_some());
        assert_eq!(config.ai_provider.unwrap().api_key(), "test_openai");
        assert_eq!(config.available_accounts.len(), 1);

        // Clean up
        unsafe {
            env::remove_var("WECHAT_APP_ID");
            env::remove_var("WECHAT_APP_SECRET");
            env::remove_var("OPENAI_API_KEY");
        }
    }

    #[test]
    fn test_ai_provider_creation() {
        let openai_provider = AiProvider::openai("test_key".to_string());
        assert_eq!(openai_provider.api_key(), "test_key");
        assert_eq!(openai_provider.name(), "OpenAI");

        let gemini_provider = AiProvider::gemini("test_key".to_string());
        assert_eq!(gemini_provider.api_key(), "test_key");
        assert_eq!(gemini_provider.name(), "Gemini");
    }

    #[test]
    fn test_config_from_env_gemini() {
        // Set environment variables for Gemini
        unsafe {
            env::set_var("WECHAT_APP_ID", "test_id");
            env::set_var("WECHAT_APP_SECRET", "test_secret");
            env::set_var("AI_PROVIDER", "gemini");
            env::set_var("GEMINI_API_KEY", "test_gemini");
        }

        let config = Config::from_env().unwrap();
        assert_eq!(config.wechat_account.app_id, "test_id");
        assert_eq!(config.wechat_account.app_secret, "test_secret");
        assert_eq!(config.wechat_account.name, "default");
        assert!(config.ai_provider.is_some());
        
        let provider = config.ai_provider.unwrap();
        assert_eq!(provider.api_key(), "test_gemini");
        assert_eq!(provider.name(), "Gemini");
        assert_eq!(config.available_accounts.len(), 1);

        // Clean up
        unsafe {
            env::remove_var("WECHAT_APP_ID");
            env::remove_var("WECHAT_APP_SECRET");
            env::remove_var("AI_PROVIDER");
            env::remove_var("GEMINI_API_KEY");
        }
    }

    #[test]
    fn test_frontmatter_creation() {
        let frontmatter = Frontmatter::new();
        assert_eq!(frontmatter.title, None);
        assert_eq!(frontmatter.published, None);
        assert_eq!(frontmatter.cover, None);

        let frontmatter = Frontmatter::with_title("Test Article");
        assert_eq!(frontmatter.title, Some("Test Article".to_string()));
    }

    #[test]
    fn test_frontmatter_methods() {
        let mut frontmatter = Frontmatter::new();

        frontmatter.set_title("My Article");
        frontmatter.set_published("draft");
        frontmatter.set_cover("cover.png");
        frontmatter.set_theme("lapis");
        frontmatter.set_code_highlighter("github");

        assert_eq!(frontmatter.title, Some("My Article".to_string()));
        assert_eq!(frontmatter.published, Some("draft".to_string()));
        assert_eq!(frontmatter.cover, Some("cover.png".to_string()));
        assert_eq!(frontmatter.theme, Some("lapis".to_string()));
        assert_eq!(frontmatter.code, Some("github".to_string()));

        assert!(frontmatter.is_draft());
        assert!(!frontmatter.is_published());
        assert!(!frontmatter.is_unpublished());
    }

    #[test]
    fn test_frontmatter_status_checks() {
        let mut frontmatter = Frontmatter::new();

        // Test unpublished
        assert!(frontmatter.is_unpublished());
        assert!(!frontmatter.is_draft());
        assert!(!frontmatter.is_published());

        // Test draft
        frontmatter.set_published("draft");
        assert!(frontmatter.is_draft());
        assert!(!frontmatter.is_published());
        assert!(!frontmatter.is_unpublished());

        // Test published
        frontmatter.set_published("true");
        assert!(frontmatter.is_published());
        assert!(!frontmatter.is_draft());
        assert!(!frontmatter.is_unpublished());
    }

    #[test]
    fn test_frontmatter_validation() {
        let mut frontmatter = Frontmatter::new();

        // Valid frontmatter
        assert!(frontmatter.validate().is_ok());

        // Valid theme and code
        frontmatter.set_theme("lapis");
        frontmatter.set_code_highlighter("github");
        assert!(frontmatter.validate().is_ok());

        // Invalid theme
        frontmatter.set_theme("invalid_theme");
        assert!(frontmatter.validate().is_err());

        // Fix theme, invalid code
        frontmatter.set_theme("lapis");
        frontmatter.set_code_highlighter("invalid_highlighter");
        assert!(frontmatter.validate().is_err());
    }

    #[test]
    fn test_theme_validation() {
        assert!(is_valid_theme("lapis"));
        assert!(is_valid_theme("default"));
        assert!(!is_valid_theme("invalid"));
        assert!(!is_valid_theme(""));
    }

    #[test]
    fn test_code_highlighter_validation() {
        assert!(is_valid_code_highlighter("github"));
        assert!(is_valid_code_highlighter("monokai"));
        assert!(!is_valid_code_highlighter("invalid"));
        assert!(!is_valid_code_highlighter(""));
    }

    #[test]
    fn test_frontmatter_serialization() {
        let frontmatter = Frontmatter {
            title: Some("Test Article".to_string()),
            published: Some("draft".to_string()),
            description: "Test Article".to_string(),
            cover: Some("cover.png".to_string()),
            theme: Some("lapis".to_string()),
            code: Some("github".to_string()),
            other: serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        };

        let yaml = serde_yaml::to_string(&frontmatter).unwrap();
        assert!(yaml.contains("title: Test Article"));
        assert!(yaml.contains("published: draft"));
        assert!(yaml.contains("cover: cover.png"));
        assert!(yaml.contains("theme: lapis"));
        assert!(yaml.contains("code: github"));

        let deserialized: Frontmatter = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(frontmatter, deserialized);
    }
}
