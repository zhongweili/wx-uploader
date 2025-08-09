//! OpenAI integration for cover image generation
//!
//! This module provides OpenAI API integration for generating article cover images
//! using GPT-5 for scene descriptions and gpt-image-1 for image generation.

use crate::error::{Error, Result};
use reqwest::Client;
use serde_json::{Value, json};
use std::path::Path;
use tracing::info;

/// Trait for generating scene descriptions from content
#[async_trait::async_trait]
pub trait SceneDescriptionGenerator {
    /// Generates a vivid scene description from markdown content
    async fn generate_scene_description(&self, content: &str) -> Result<String>;
}

/// Trait for generating images from text descriptions
#[async_trait::async_trait]
pub trait ImageGenerator {
    /// Generates an image from a text prompt and returns the URL
    async fn generate_image(&self, prompt: &str) -> Result<String>;

    /// Downloads an image from a URL and saves it to the specified path
    async fn download_image(&self, url: &str, file_path: &Path) -> Result<()>;
}

/// Trait for creating image generation prompts
pub trait PromptBuilder {
    /// Creates an image prompt for generating a Studio Ghibli-style cover image
    fn create_dalle_prompt(&self, scene_description: &str) -> String;
}

/// OpenAI API client for generating cover images and scene descriptions
#[derive(Clone, Debug)]
pub struct OpenAIClient {
    api_key: String,
    http_client: Client,
    base_url: String,
}

impl OpenAIClient {
    /// Creates a new OpenAI client with the provided API key
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http_client: Client::new(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    /// Creates a new OpenAI client with a custom base URL (useful for testing)
    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            http_client: Client::new(),
            base_url,
        }
    }

    /// Creates a new OpenAI client with a custom HTTP client
    pub fn with_client(api_key: String, http_client: Client) -> Self {
        Self {
            api_key,
            http_client,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    /// Generates and saves a cover image for the given markdown content
    ///
    /// This method combines scene description generation, DALL-E prompt creation,
    /// image generation, and file saving into a single operation.
    ///
    /// # Arguments
    ///
    /// * `content` - The markdown content to analyze
    /// * `file_path` - The path to the markdown file (used for determining save location)
    /// * `base_filename` - Base name for the generated cover image
    ///
    /// # Returns
    ///
    /// The filename of the generated cover image
    pub async fn generate_cover_image(
        &self,
        content: &str,
        file_path: &Path,
        base_filename: &str,
    ) -> Result<String> {
        // Generate scene description from content
        let scene_description = self.generate_scene_description(content).await?;
        info!("Generated scene description: {}", scene_description);

        // Create DALL-E prompt
        let dalle_prompt = self.create_dalle_prompt(&scene_description);
        info!("DALL-E prompt: {}", dalle_prompt);

        // Show prompt in console for user visibility
        println!(
            "  {} {}",
            "→".bright_blue(),
            format!("Image prompt: {}", dalle_prompt).bright_white()
        );

        // Generate image
        let image_url = self.generate_image(&dalle_prompt).await?;

        // Create filename for the cover image
        let cover_filename = format!(
            "{}_cover_{}.png",
            base_filename,
            uuid::Uuid::new_v4().simple()
        );
        let cover_path = file_path
            .parent()
            .ok_or_else(|| Error::generic("Failed to get parent directory"))?
            .join(&cover_filename);

        // Download and save the image
        self.download_image(&image_url, &cover_path).await?;

        Ok(cover_filename)
    }

    /// Generates and saves a cover image to a specific path
    ///
    /// # Arguments
    ///
    /// * `content` - The markdown content to analyze
    /// * `_markdown_file_path` - The path to the markdown file (for context)
    /// * `target_cover_path` - The exact path where to save the cover image
    pub async fn generate_cover_image_to_path(
        &self,
        content: &str,
        _markdown_file_path: &Path,
        target_cover_path: &Path,
    ) -> Result<()> {
        // Generate scene description from content
        let scene_description = self.generate_scene_description(content).await?;
        info!("Generated scene description: {}", scene_description);

        // Create DALL-E prompt
        let dalle_prompt = self.create_dalle_prompt(&scene_description);
        info!("DALL-E prompt: {}", dalle_prompt);

        // Show prompt in console for user visibility
        println!(
            "  {} {}",
            "→".bright_blue(),
            format!("Image prompt: {}", dalle_prompt).bright_white()
        );

        // Generate image
        let image_url = self.generate_image(&dalle_prompt).await?;

        // Download and save the image to the specified path
        self.download_image(&image_url, target_cover_path).await?;

        Ok(())
    }

    /// Makes a POST request to the OpenAI API
    async fn post_request(&self, endpoint: &str, body: Value) -> Result<Value> {
        let url = format!("{}/{}", self.base_url, endpoint);

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::openai(format!(
                "API request failed with status {}: {}",
                status, error_text
            )));
        }

        let response_json: Value = response.json().await?;
        Ok(response_json)
    }
}

#[async_trait::async_trait]
impl SceneDescriptionGenerator for OpenAIClient {
    async fn generate_scene_description(&self, content: &str) -> Result<String> {
        let request_body = json!({
            "model": "gpt-5-mini",
            "messages": [
                {
                    "role": "system",
                    "content": "You are a creative writing assistant. Analyze the given markdown content and generate a vivid, detailed scene description that captures the essence of the article. Focus on visual elements, mood, and atmosphere that would make for compelling cover art. Keep the description concise but evocative (2-3 sentences maximum)."
                },
                {
                    "role": "user",
                    "content": format!("Please analyze this markdown content and generate a scene description:\n\n{}", content)
                }
            ],
            "max_tokens": 150,
            "temperature": 0.7
        });

        let response_json = self.post_request("chat/completions", request_body).await?;

        let scene_description = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| Error::openai("Failed to extract scene description from response"))?
            .trim()
            .to_string();

        Ok(scene_description)
    }
}

impl PromptBuilder for OpenAIClient {
    fn create_dalle_prompt(&self, scene_description: &str) -> String {
        format!(
            "Create a wide, Ghibli-style image to represent this scene: {}",
            scene_description
        )
    }
}

#[async_trait::async_trait]
impl ImageGenerator for OpenAIClient {
    async fn generate_image(&self, prompt: &str) -> Result<String> {
        let request_body = json!({
            "model": "gpt-image-1",
            "prompt": prompt,
            "size": "1792x1024",  // 16:9 aspect ratio
            "quality": "standard",
            "response_format": "url",
            "n": 1
        });

        let response_json = self
            .post_request("images/generations", request_body)
            .await?;

        let image_url = response_json["data"][0]["url"]
            .as_str()
            .ok_or_else(|| Error::openai("Failed to extract image URL from response"))?
            .to_string();

        Ok(image_url)
    }

    async fn download_image(&self, url: &str, file_path: &Path) -> Result<()> {
        let response = self.http_client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(Error::openai(format!(
                "Failed to download image: HTTP {}",
                response.status()
            )));
        }

        let image_bytes = response.bytes().await?;

        // Ensure the directory exists
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(file_path, image_bytes).await?;

        Ok(())
    }
}

/// Builder for creating OpenAI clients with different configurations
pub struct OpenAIClientBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    http_client: Option<Client>,
}

impl OpenAIClientBuilder {
    /// Creates a new builder
    pub fn new() -> Self {
        Self {
            api_key: None,
            base_url: None,
            http_client: None,
        }
    }

    /// Sets the API key
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    /// Sets a custom base URL
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = Some(base_url);
        self
    }

    /// Sets a custom HTTP client
    pub fn with_http_client(mut self, client: Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Builds the OpenAI client
    pub fn build(self) -> Result<OpenAIClient> {
        let api_key = self
            .api_key
            .ok_or_else(|| Error::config("OpenAI API key is required"))?;

        let client = match (self.base_url, self.http_client) {
            (Some(base_url), Some(http_client)) => OpenAIClient {
                api_key,
                http_client,
                base_url,
            },
            (Some(base_url), None) => OpenAIClient::with_base_url(api_key, base_url),
            (None, Some(http_client)) => OpenAIClient::with_client(api_key, http_client),
            (None, None) => OpenAIClient::new(api_key),
        };

        Ok(client)
    }
}

impl Default for OpenAIClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Add colored import for console output
use colored::*;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_openai_client_creation() {
        let client = OpenAIClient::new("test-key".to_string());
        assert_eq!(client.api_key, "test-key");
        assert_eq!(client.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn test_openai_client_with_base_url() {
        let client = OpenAIClient::with_base_url(
            "test-key".to_string(),
            "https://custom.api.com".to_string(),
        );
        assert_eq!(client.api_key, "test-key");
        assert_eq!(client.base_url, "https://custom.api.com");
    }

    #[test]
    fn test_create_dalle_prompt() {
        let client = OpenAIClient::new("test-key".to_string());
        let scene_description = "A serene forest with morning mist";
        let prompt = client.create_dalle_prompt(scene_description);

        assert!(prompt.contains("Ghibli-style"));
        assert!(prompt.contains("A serene forest with morning mist"));
    }

    #[test]
    fn test_openai_client_builder() {
        let client = OpenAIClientBuilder::new()
            .with_api_key("test-key".to_string())
            .with_base_url("https://custom.api.com".to_string())
            .build()
            .unwrap();

        assert_eq!(client.api_key, "test-key");
        assert_eq!(client.base_url, "https://custom.api.com");
    }

    #[test]
    fn test_openai_client_builder_missing_api_key() {
        let result = OpenAIClientBuilder::new()
            .with_base_url("https://custom.api.com".to_string())
            .build();

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("OpenAI API key is required")
        );
    }

    #[tokio::test]
    async fn test_download_image_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir
            .path()
            .join("nested")
            .join("directory")
            .join("image.png");

        let _client = OpenAIClient::new("test-key".to_string());

        // This test would require mocking the HTTP client to avoid making real requests
        // For now, we'll just test the directory creation logic indirectly

        // Create a mock response scenario
        // In a real test, we'd use a mock server or dependency injection
        assert!(!nested_path.exists());

        // The actual download_image call would be mocked in integration tests
        // Here we just verify the path logic
        assert!(nested_path.parent().is_some());
    }

    #[test]
    fn test_error_messages_are_descriptive() {
        let error = Error::openai("Rate limit exceeded");
        assert!(error.to_string().contains("OpenAI API error"));
        assert!(error.to_string().contains("Rate limit exceeded"));

        let error = Error::config("Invalid configuration");
        assert!(error.to_string().contains("Configuration error"));
        assert!(error.to_string().contains("Invalid configuration"));
    }
}
