//! AI provider abstraction layer
//!
//! This module provides a unified interface for different AI providers
//! including OpenAI, Google Gemini, and other compatible services.

use crate::error::{Error, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use std::path::Path;

/// Configuration for AI providers
#[derive(Debug, Clone)]
pub enum ProviderConfig {
    /// OpenAI configuration
    OpenAI {
        api_key: String,
        base_url: Option<String>,
    },
    /// Google Gemini configuration
    Gemini {
        api_key: String,
        base_url: Option<String>,
    },
}

impl ProviderConfig {
    /// Get the API key for the provider
    pub fn api_key(&self) -> &str {
        match self {
            ProviderConfig::OpenAI { api_key, .. } => api_key,
            ProviderConfig::Gemini { api_key, .. } => api_key,
        }
    }

    /// Get the base URL for the provider
    pub fn base_url(&self) -> &str {
        match self {
            ProviderConfig::OpenAI { base_url, .. } => {
                base_url.as_deref().unwrap_or("https://api.openai.com/v1")
            }
            ProviderConfig::Gemini { base_url, .. } => {
                base_url.as_deref().unwrap_or("https://generativelanguage.googleapis.com/v1beta/models")
            }
        }
    }

    /// Get the provider name for debugging/logging
    pub fn provider_name(&self) -> &'static str {
        match self {
            ProviderConfig::OpenAI { .. } => "OpenAI",
            ProviderConfig::Gemini { .. } => "Gemini",
        }
    }
}

/// Trait for generating scene descriptions from content
#[async_trait]
pub trait SceneDescriptionGenerator {
    /// Generates a vivid scene description from markdown content
    async fn generate_scene_description(&self, content: &str) -> Result<String>;
}

/// Trait for generating images from text descriptions
#[async_trait]
pub trait ImageGenerator {
    /// Generates an image from a text prompt and returns the URL or base64 data
    async fn generate_image(&self, prompt: &str) -> Result<String>;

    /// Downloads an image from a URL and saves it to the specified path
    async fn download_image(&self, url: &str, file_path: &Path) -> Result<()>;
}

/// Trait for creating image generation prompts
pub trait PromptBuilder {
    /// Creates an image prompt for generating a Studio Ghibli-style cover image
    fn create_dalle_prompt(&self, scene_description: &str) -> String;
}

/// Trait for processing cover images
#[async_trait]
pub trait CoverImageProcessor {
    /// Generates and saves a cover image for the given markdown content
    async fn generate_cover_image(
        &self,
        content: &str,
        file_path: &Path,
        base_filename: &str,
    ) -> Result<String>;

    /// Generates and saves a cover image to a specific path
    async fn generate_cover_image_to_path(
        &self,
        content: &str,
        markdown_file_path: &Path,
        target_cover_path: &Path,
    ) -> Result<()>;
}

/// Model configurations for different providers
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// Model name for text generation (scene descriptions)
    pub text_model: String,
    /// Model name for image generation
    pub image_model: String,
    /// Temperature for text generation
    pub temperature: f32,
    /// Image size specification
    pub image_size: String,
    /// Image quality setting
    pub image_quality: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            text_model: "gpt-4o-mini".to_string(),
            image_model: "dall-e-3".to_string(),
            temperature: 0.7,
            image_size: "1536x1024".to_string(),
            image_quality: "standard".to_string(),
        }
    }
}

/// Provider-specific model configurations
impl ModelConfig {
    /// Get OpenAI model configuration
    pub fn openai() -> Self {
        Self {
            text_model: "gpt-4o-mini".to_string(),
            image_model: "dall-e-3".to_string(),
            temperature: 0.7,
            image_size: "1536x1024".to_string(),
            image_quality: "standard".to_string(),
        }
    }

    /// Get Google Gemini model configuration
    pub fn gemini() -> Self {
        Self {
            text_model: "gemini-2.5-flash".to_string(),
            image_model: "imagen-4.0-generate-001".to_string(),
            temperature: 0.7,
            image_size: "16:9".to_string(),
            image_quality: "high".to_string(),
        }
    }
}

/// Universal AI client that works with multiple providers
#[derive(Clone, Debug)]
pub struct UniversalAIClient {
    config: ProviderConfig,
    model_config: ModelConfig,
    http_client: Client,
}

impl UniversalAIClient {
    /// Creates a new universal AI client
    pub fn new(config: ProviderConfig, model_config: Option<ModelConfig>) -> Self {
        let model_config = model_config.unwrap_or_else(|| match &config {
            ProviderConfig::OpenAI { .. } => ModelConfig::openai(),
            ProviderConfig::Gemini { .. } => ModelConfig::gemini(),
        });

        Self {
            config,
            model_config,
            http_client: Client::new(),
        }
    }

    /// Creates a new universal AI client with custom HTTP client
    pub fn with_client(
        config: ProviderConfig,
        model_config: Option<ModelConfig>,
        http_client: Client,
    ) -> Self {
        let model_config = model_config.unwrap_or_else(|| match &config {
            ProviderConfig::OpenAI { .. } => ModelConfig::openai(),
            ProviderConfig::Gemini { .. } => ModelConfig::gemini(),
        });

        Self {
            config,
            model_config,
            http_client,
        }
    }

    /// Makes a POST request to the provider API
    async fn post_request(&self, endpoint: &str, body: Value) -> Result<Value> {
        let url = match &self.config {
            ProviderConfig::Gemini { .. } => {
                // For Gemini, endpoint is the complete model path
                format!("{}{}?key={}", self.config.base_url(), endpoint, self.config.api_key())
            }
            _ => format!("{}/{}", self.config.base_url(), endpoint)
        };
        
        let mut request = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json");

        // Add provider-specific headers
        match &self.config {
            ProviderConfig::Gemini { .. } => {
                // Gemini uses API key in URL, no Authorization header needed
            }
            ProviderConfig::OpenAI { .. } => {
                request = request.header("Authorization", format!("Bearer {}", self.config.api_key()));
            }
        }

        let response = request.json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            return Err(Error::openai(format!(
                "{} API request failed with status {}: {}",
                self.config.provider_name(),
                status,
                error_text
            )));
        }

        let response_json: Value = response.json().await?;
        Ok(response_json)
    }
}

#[async_trait]
impl SceneDescriptionGenerator for UniversalAIClient {
    async fn generate_scene_description(&self, content: &str) -> Result<String> {
        let (request_body, endpoint) = match &self.config {
            ProviderConfig::OpenAI { .. } => {
                let body = json!({
                    "model": self.model_config.text_model,
                    "messages": [
                        {
                            "role": "system",
                            "content": "Generate a 2-sentence visual scene description in English for a cover image based on the article content."
                        },
                        {
                            "role": "user",
                            "content": format!("Article content:\n\n{}\n\nScene description:",
                                if content.len() > 2000 { &content[..2000] } else { content })
                        }
                    ],
                    "temperature": self.model_config.temperature
                });
                (body, "chat/completions".to_string())
            }
            ProviderConfig::Gemini { .. } => {
                let body = json!({
                    "contents": [
                        {
                            "parts": [
                                {
                                    "text": format!("Generate a 2-sentence visual scene description in English for a cover image based on this article content:\n\n{}\n\nScene description:",
                                        if content.len() > 2000 { &content[..2000] } else { content })
                                }
                            ]
                        }
                    ],
                    "generationConfig": {
                        "temperature": self.model_config.temperature
                    }
                });
                (body, format!("/{}:generateContent", self.model_config.text_model))
            }
        };

        let response_json = self.post_request(&endpoint, request_body).await?;

        let mut scene_description = match &self.config {
            ProviderConfig::OpenAI { .. } => {
                response_json["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            }
            ProviderConfig::Gemini { .. } => {
                response_json["candidates"][0]["content"]["parts"][0]["text"]
                    .as_str()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            }
        };

        if scene_description.is_empty() {
            scene_description = "A serene landscape with rolling hills under a soft, dreamy sky filled with gentle clouds. The scene evokes a sense of peaceful contemplation and infinite possibilities.".to_string();
        }

        Ok(scene_description)
    }
}

#[async_trait]
impl ImageGenerator for UniversalAIClient {
    async fn generate_image(&self, prompt: &str) -> Result<String> {
        let (request_body, endpoint) = match &self.config {
            ProviderConfig::OpenAI { .. } => {
                let body = json!({
                    "model": self.model_config.image_model,
                    "prompt": prompt,
                    "size": self.model_config.image_size,
                    "quality": self.model_config.image_quality,
                    "n": 1
                });
                (body, "images/generations".to_string())
            }
            ProviderConfig::Gemini { .. } => {
                let body = json!({
                    "instances": [
                        {
                            "prompt": prompt
                        }
                    ],
                    "parameters": {
                        "numberOfImages": 1,
                        "aspectRatio": "16:9"
                    }
                });
                (body, format!("/{}:predict", self.model_config.image_model))
            }
        };

        let response_json = self.post_request(&endpoint, request_body).await?;

        // Handle different response formats
        match &self.config {
            ProviderConfig::OpenAI { .. } => {
                if let Some(url) = response_json["data"][0]["url"].as_str() {
                    Ok(url.to_string())
                } else if let Some(b64) = response_json["data"][0]["b64_json"].as_str() {
                    Ok(format!("base64:{}", b64))
                } else {
                    Err(Error::openai(format!(
                        "Failed to extract image data from {} response",
                        self.config.provider_name()
                    )))
                }
            }
            ProviderConfig::Gemini { .. } => {
                // Gemini returns image bytes directly in predictions array
                if let Some(prediction) = response_json["predictions"].get(0) {
                    // Handle the response format - might need to decode base64 or process bytes
                    if let Some(bytes_data) = prediction["bytesBase64Encoded"].as_str() {
                        Ok(format!("base64:{}", bytes_data))
                    } else if let Some(image_data) = prediction.as_str() {
                        // If it's a direct base64 string
                        Ok(format!("base64:{}", image_data))
                    } else {
                        Err(Error::openai(format!(
                            "Failed to extract image data from {} response",
                            self.config.provider_name()
                        )))
                    }
                } else {
                    Err(Error::openai(format!(
                        "No predictions found in {} response",
                        self.config.provider_name()
                    )))
                }
            }
        }
    }

    async fn download_image(&self, url: &str, file_path: &Path) -> Result<()> {
        use base64::Engine;
        
        let image_bytes = if let Some(base64_str) = url.strip_prefix("base64:") {
            // Decode base64 data
            base64::engine::general_purpose::STANDARD
                .decode(base64_str)
                .map_err(|e| Error::openai(format!("Failed to decode base64 image: {}", e)))?
        } else {
            // Download from URL
            let response = self.http_client.get(url).send().await?;

            if !response.status().is_success() {
                return Err(Error::openai(format!(
                    "Failed to download image: HTTP {}",
                    response.status()
                )));
            }

            let bytes = response.bytes().await?;
            bytes.to_vec()
        };

        // Ensure the directory exists
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(file_path, image_bytes).await?;
        Ok(())
    }
}

impl PromptBuilder for UniversalAIClient {
    fn create_dalle_prompt(&self, scene_description: &str) -> String {
        format!(
            "Create a wide, Ghibli-style image to represent this scene: {}",
            scene_description
        )
    }
}

#[async_trait]
impl CoverImageProcessor for UniversalAIClient {
    async fn generate_cover_image(
        &self,
        content: &str,
        file_path: &Path,
        base_filename: &str,
    ) -> Result<String> {
        use crate::output::{FORMATTER, OutputFormatter, FilePathFormatter};
        use tracing::info;

        // Generate scene description from content
        let scene_description = match self.generate_scene_description(content).await {
            Ok(desc) => {
                info!("Generated scene description: {}", desc);
                desc
            }
            Err(e) => {
                FORMATTER.print_error(&format!("Failed to generate scene description: {}", e));
                return Err(e);
            }
        };

        // Create DALL-E prompt
        let dalle_prompt = self.create_dalle_prompt(&scene_description);
        info!("DALL-E prompt: {}", dalle_prompt);

        // Show prompt in console for user visibility
        println!("{}", FORMATTER.format_image_prompt(&dalle_prompt));

        // Generate image
        let image_url = match self.generate_image(&dalle_prompt).await {
            Ok(url) => {
                info!("Successfully generated image URL: {}", url);
                url
            }
            Err(e) => {
                FORMATTER.print_error(&format!("Failed to generate image: {}", e));
                return Err(e);
            }
        };

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

    async fn generate_cover_image_to_path(
        &self,
        content: &str,
        _markdown_file_path: &Path,
        target_cover_path: &Path,
    ) -> Result<()> {
        use crate::output::{FORMATTER, OutputFormatter, FilePathFormatter};
        use tracing::info;

        println!("{}", FORMATTER.format_target_path(target_cover_path));

        // Generate scene description from content
        let scene_description = match self.generate_scene_description(content).await {
            Ok(desc) => {
                info!("Generated scene description: {}", desc);
                desc
            }
            Err(e) => {
                FORMATTER.print_error(&format!("Failed to generate scene description: {}", e));
                return Err(e);
            }
        };

        // Create DALL-E prompt
        let dalle_prompt = self.create_dalle_prompt(&scene_description);
        info!("DALL-E prompt: {}", dalle_prompt);

        // Show prompt in console for user visibility
        println!("{}", FORMATTER.format_image_prompt(&dalle_prompt));

        // Generate image
        let image_url = match self.generate_image(&dalle_prompt).await {
            Ok(url) => {
                info!("Successfully generated image URL: {}", url);
                url
            }
            Err(e) => {
                FORMATTER.print_error(&format!("Failed to generate image: {}", e));
                return Err(e);
            }
        };

        // Download and save the image to the specified path
        match self.download_image(&image_url, target_cover_path).await {
            Ok(()) => {
                println!("{}", FORMATTER.format_image_saved(target_cover_path));
                Ok(())
            }
            Err(e) => {
                FORMATTER.print_error(&format!("Failed to download image: {}", e));
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config() {
        let openai_config = ProviderConfig::OpenAI {
            api_key: "test-key".to_string(),
            base_url: None,
        };
        
        assert_eq!(openai_config.api_key(), "test-key");
        assert_eq!(openai_config.base_url(), "https://api.openai.com/v1");
        assert_eq!(openai_config.provider_name(), "OpenAI");

        let gemini_config = ProviderConfig::Gemini {
            api_key: "test-key".to_string(),
            base_url: Some("https://custom.generativelanguage.googleapis.com/v1beta/models".to_string()),
        };
        
        assert_eq!(gemini_config.api_key(), "test-key");
        assert_eq!(gemini_config.base_url(), "https://custom.generativelanguage.googleapis.com/v1beta/models");
        assert_eq!(gemini_config.provider_name(), "Gemini");
    }

    #[test]
    fn test_model_config() {
        let default_config = ModelConfig::default();
        assert_eq!(default_config.text_model, "gpt-4o-mini");
        assert_eq!(default_config.image_model, "dall-e-3");

        let openai_config = ModelConfig::openai();
        assert_eq!(openai_config.text_model, "gpt-4o-mini");
        assert_eq!(openai_config.image_model, "dall-e-3");

        let gemini_config = ModelConfig::gemini();
        assert_eq!(gemini_config.text_model, "gemini-2.5-flash");
        assert_eq!(gemini_config.image_model, "imagen-4.0-generate-001");
    }

    #[test]
    fn test_universal_client_creation() {
        let config = ProviderConfig::OpenAI {
            api_key: "test-key".to_string(),
            base_url: None,
        };
        
        let client = UniversalAIClient::new(config, None);
        assert_eq!(client.model_config.text_model, "gpt-4o-mini");
        
        let config = ProviderConfig::Gemini {
            api_key: "test-key".to_string(),
            base_url: None,
        };
        
        let client = UniversalAIClient::new(config, None);
        assert_eq!(client.model_config.text_model, "gemini-2.5-flash");
    }

    #[test]
    fn test_dalle_prompt_creation() {
        let config = ProviderConfig::OpenAI {
            api_key: "test-key".to_string(),
            base_url: None,
        };
        
        let client = UniversalAIClient::new(config, None);
        let scene_description = "A serene forest with morning mist";
        let prompt = client.create_dalle_prompt(scene_description);

        assert!(prompt.contains("Ghibli-style"));
        assert!(prompt.contains("A serene forest with morning mist"));
    }
}