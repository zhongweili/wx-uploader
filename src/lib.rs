//! WeChat Public Account Markdown Uploader Library
//!
//! A library for uploading markdown files to WeChat public accounts with automatic
//! cover image generation and frontmatter management.
//!
//! ## Features
//!
//! - Upload individual markdown files or process directories recursively
//! - Parse and manage YAML frontmatter to track publication status
//! - Automatically generate cover images using AI (OpenAI, Gemini) when missing
//! - Skip already published files in directory processing mode
//! - Support for custom themes and code highlighters via frontmatter
//!
//! ## Usage
//!
//! ```rust,no_run
//! use wx_uploader::{WxUploader, Config, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let config = Config::from_env()?;
//!     let uploader = WxUploader::new(config).await?;
//!
//!     // Upload a single file
//!     uploader.upload_file("article.md", true).await?;
//!
//!     // Process a directory
//!     uploader.process_directory("./articles").await?;
//!
//!     Ok(())
//! }
//! ```

pub mod cli;
pub mod error;
pub mod markdown;
pub mod models;
pub mod openai;
pub mod output;
pub mod providers;
pub mod wechat;

pub use error::{Error, Result};
pub use models::{Config, Frontmatter, AiProvider};
// Core uploader functionality is implemented directly in this module

use std::path::Path;

/// Core uploader functionality combining WeChat and AI provider clients
pub struct WxUploader {
    wechat_client: wechat::WeChatClient,
    ai_client: Option<providers::UniversalAIClient>,
    config: Config,
}

impl WxUploader {
    /// Creates a new uploader instance with the provided configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration containing API keys and settings
    ///
    /// # Errors
    ///
    /// Returns an error if WeChat client initialization fails
    pub async fn new(config: Config) -> Result<Self> {
        let wechat_client = wechat::WeChatClient::new(
            config.wechat_account.app_id.clone(),
            config.wechat_account.app_secret.clone(),
        )
        .await
        .map_err(|e| Error::wechat(e.to_string()))?;

        let ai_client = config.ai_provider.as_ref().map(|provider| {
            let provider_config = match provider {
                models::AiProvider::OpenAI { api_key, base_url } => {
                    providers::ProviderConfig::OpenAI {
                        api_key: api_key.clone(),
                        base_url: base_url.clone(),
                    }
                }
                models::AiProvider::Gemini { api_key, base_url } => {
                    providers::ProviderConfig::Gemini {
                        api_key: api_key.clone(),
                        base_url: base_url.clone(),
                    }
                }
            };
            providers::UniversalAIClient::new(provider_config, None)
        });

        Ok(Self {
            wechat_client,
            ai_client,
            config,
        })
    }

    /// Switches to a different WeChat account and reinitializes the client
    ///
    /// # Arguments
    ///
    /// * `account_name` - Name of the account to switch to
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The account is not found in configuration
    /// - WeChat client reinitialization fails
    pub async fn switch_account(&mut self, account_name: &str) -> Result<()> {
        // Switch the account in config
        self.config.switch_account(account_name)?;
        
        // Reinitialize WeChat client with new credentials
        self.wechat_client = wechat::WeChatClient::new(
            self.config.wechat_account.app_id.clone(),
            self.config.wechat_account.app_secret.clone(),
        )
        .await
        .map_err(|e| Error::wechat(e.to_string()))?;
        
        Ok(())
    }
    
    /// Gets the current WeChat account information
    pub fn current_account(&self) -> &models::WeChatAccount {
        &self.config.wechat_account
    }
    
    /// Lists all available WeChat accounts
    pub fn list_accounts(&self) -> Vec<&models::WeChatAccount> {
        self.config.list_accounts()
    }

    /// Forces a refresh of the WeChat access token
    ///
    /// This clears the in-memory token cache and fetches a new token from the WeChat API.
    ///
    /// # Errors
    ///
    /// Returns an error if the token refresh fails
    pub async fn refresh_token(&self) -> Result<String> {
        self.wechat_client
            .refresh_token()
            .await
            .map_err(|e| Error::wechat(e.to_string()))
    }

    /// Uploads a single markdown file to WeChat
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the markdown file
    /// * `force` - If true, uploads regardless of published status
    ///
    /// # Errors
    ///
    /// Returns an error if the upload process fails
    pub async fn upload_file<P: AsRef<Path>>(&self, path: P, force: bool) -> Result<()> {
        wechat::upload_file(
            &self.wechat_client,
            self.ai_client.as_ref(),
            path.as_ref(),
            force,
            self.config.verbose,
        )
        .await
    }

    /// Processes all markdown files in a directory recursively
    ///
    /// Files marked as published will be skipped unless forced.
    ///
    /// # Arguments
    ///
    /// * `dir` - Directory path to process
    ///
    /// # Errors
    ///
    /// Returns an error if directory processing fails
    pub async fn process_directory<P: AsRef<Path>>(&self, dir: P) -> Result<()> {
        wechat::process_directory(
            &self.wechat_client,
            self.ai_client.as_ref(),
            dir.as_ref(),
            self.config.verbose,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_uploader_creation_without_ai() {
        let config = Config::new(
            "test_app_id".to_string(),
            "test_secret".to_string(),
            None,
            false,
        );

        // This would fail in real scenario without valid WeChat credentials,
        // but tests the structure
        let result = WxUploader::new(config).await;
        // We expect this to fail with network/auth error, not a compilation error
        assert!(result.is_err());
    }
    
    #[test]
    fn test_account_switching() {
        use std::collections::HashMap;
        
        let mut accounts = HashMap::new();
        accounts.insert(
            "test1".to_string(),
            models::WeChatAccount {
                name: "test1".to_string(),
                app_id: "app1".to_string(),
                app_secret: "secret1".to_string(),
                description: None,
            },
        );
        accounts.insert(
            "test2".to_string(),
            models::WeChatAccount {
                name: "test2".to_string(),
                app_id: "app2".to_string(),
                app_secret: "secret2".to_string(),
                description: None,
            },
        );
        
        let mut config = Config::new_with_accounts(accounts, "test1", None, false).unwrap();
        
        assert_eq!(config.wechat_account.name, "test1");
        assert_eq!(config.wechat_account.app_id, "app1");
        
        // Test switching accounts
        config.switch_account("test2").unwrap();
        assert_eq!(config.wechat_account.name, "test2");
        assert_eq!(config.wechat_account.app_id, "app2");
        
        // Test switching to non-existent account
        let result = config.switch_account("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_from_env_missing_required() {
        // Clear environment variables
        unsafe {
            std::env::remove_var("WECHAT_APP_ID");
            std::env::remove_var("WECHAT_APP_SECRET");
        }

        let result = Config::from_env();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_multi_account_config() {
        use std::collections::HashMap;
        
        let mut accounts = HashMap::new();
        accounts.insert(
            "personal".to_string(),
            models::WeChatAccount {
                name: "personal".to_string(),
                app_id: "personal_app_id".to_string(),
                app_secret: "personal_secret".to_string(),
                description: Some("Personal account".to_string()),
            },
        );
        accounts.insert(
            "work".to_string(),
            models::WeChatAccount {
                name: "work".to_string(),
                app_id: "work_app_id".to_string(),
                app_secret: "work_secret".to_string(),
                description: Some("Work account".to_string()),
            },
        );
        
        let config = Config::new_with_accounts(accounts, "personal", None, false).unwrap();
        
        assert_eq!(config.wechat_account.name, "personal");
        assert_eq!(config.available_accounts.len(), 2);
        
        let account_list = config.list_accounts();
        assert_eq!(account_list.len(), 2);
        
        // Validate configuration
        assert!(config.validate().is_ok());
    }
}
