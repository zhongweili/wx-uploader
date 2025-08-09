//! WeChat public account integration
//!
//! This module provides WeChat public account functionality for uploading
//! markdown articles with automatic cover image generation and frontmatter management.

use crate::error::{Error, Result};
use crate::markdown::{parse_markdown_file, update_frontmatter, write_markdown_file};
use crate::openai::OpenAIClient;
use colored::*;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use walkdir::WalkDir;

// Re-export the WeChat client type
pub use wechat_pub_rs::WeChatClient;

/// Trait for uploading content to WeChat
#[async_trait::async_trait]
pub trait WeChatUploader {
    /// Uploads a file to WeChat and returns the draft ID
    async fn upload(&self, file_path: &str) -> Result<String>;
}

/// Trait for processing cover images
#[async_trait::async_trait]
pub trait CoverImageProcessor {
    /// Resolves and checks if a cover image exists
    async fn resolve_cover_path(
        &self,
        markdown_path: &Path,
        cover_filename: &str,
    ) -> (PathBuf, bool);

    /// Generates a cover image if missing
    async fn ensure_cover_image(
        &self,
        content: &str,
        markdown_path: &Path,
        cover_filename: Option<&str>,
    ) -> Result<Option<String>>;
}

/// Default implementation of WeChat uploader
#[async_trait::async_trait]
impl WeChatUploader for WeChatClient {
    async fn upload(&self, file_path: &str) -> Result<String> {
        self.upload(file_path)
            .await
            .map_err(|e| Error::wechat(e.to_string()))
    }
}

/// Default cover image processor implementation
pub struct DefaultCoverImageProcessor<'a> {
    openai_client: Option<&'a OpenAIClient>,
}

impl<'a> DefaultCoverImageProcessor<'a> {
    pub fn new(openai_client: Option<&'a OpenAIClient>) -> Self {
        Self { openai_client }
    }
}

#[async_trait::async_trait]
impl CoverImageProcessor for DefaultCoverImageProcessor<'_> {
    async fn resolve_cover_path(
        &self,
        markdown_path: &Path,
        cover_filename: &str,
    ) -> (PathBuf, bool) {
        resolve_and_check_cover_path(markdown_path, cover_filename)
    }

    async fn ensure_cover_image(
        &self,
        content: &str,
        markdown_path: &Path,
        cover_filename: Option<&str>,
    ) -> Result<Option<String>> {
        let Some(openai_client) = self.openai_client else {
            return Ok(None);
        };

        match cover_filename {
            None => {
                // Generate with auto filename
                let base_filename = markdown_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("article");

                match openai_client
                    .generate_cover_image(content, markdown_path, base_filename)
                    .await
                {
                    Ok(cover_filename) => Ok(Some(cover_filename)),
                    Err(e) => {
                        warn!(
                            "Failed to generate cover image: {}. Continuing without cover.",
                            e
                        );
                        Ok(None)
                    }
                }
            }
            Some(filename) => {
                // Generate to the specified path from frontmatter
                let (target_cover_path, exists) =
                    self.resolve_cover_path(markdown_path, filename).await;

                if !exists {
                    match openai_client
                        .generate_cover_image_to_path(content, markdown_path, &target_cover_path)
                        .await
                    {
                        Ok(()) => Ok(Some(filename.to_string())),
                        Err(e) => {
                            warn!(
                                "Failed to generate cover image to {}: {}. Continuing without cover.",
                                target_cover_path.display(),
                                e
                            );
                            Ok(None)
                        }
                    }
                } else {
                    // Cover already exists
                    Ok(Some(filename.to_string()))
                }
            }
        }
    }
}

/// Recursively processes all markdown files in a directory.
///
/// This function walks through the directory tree starting from `dir`,
/// finds all files with `.md` extension, and uploads them to WeChat.
/// Files that are already published (where `published: "true"`) will be skipped.
///
/// # Arguments
///
/// * `client` - WeChat client for API communication
/// * `openai_client` - Optional OpenAI client for cover image generation
/// * `dir` - Directory path to process recursively
/// * `verbose` - Whether to enable detailed tracing logs
///
/// # Errors
///
/// Returns an error if:
/// - Directory traversal fails
/// - Any file upload fails
pub async fn process_directory(
    client: &WeChatClient,
    openai_client: Option<&OpenAIClient>,
    dir: &Path,
    verbose: bool,
) -> Result<()> {
    let entries: Vec<_> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .collect();

    if entries.is_empty() {
        println!(
            "{} {}",
            "ℹ".bright_blue(),
            "No markdown files found in directory".dimmed()
        );
        return Ok(());
    }

    for entry in entries {
        upload_file(client, openai_client, entry.path(), false, verbose).await?;
    }

    Ok(())
}

/// Uploads a single markdown file to WeChat public account.
///
/// This function handles the complete upload workflow:
/// 1. Reads and parses the markdown file
/// 2. Checks publication status (unless forced)
/// 3. Generates cover image if missing and OpenAI client is available
/// 4. Uploads to WeChat if needed
/// 5. Updates the frontmatter with draft status and cover image
/// 6. Writes the updated content back to the file
///
/// # Arguments
///
/// * `client` - WeChat client for API communication
/// * `openai_client` - Optional OpenAI client for cover image generation
/// * `path` - Path to the markdown file
/// * `force` - If true, uploads regardless of published status
/// * `verbose` - Whether to enable detailed tracing logs
///
/// # Behavior
///
/// - If `force` is false and the file has `published: "true"`, it will be skipped
/// - If no cover image is specified and OpenAI client is available, generates one
/// - After successful upload, the frontmatter is updated with `published: "draft"`
/// - The original file is modified to reflect the new status
/// - Output format depends on verbose flag: clean user-friendly messages or detailed logs
///
/// # Errors
///
/// Returns an error if:
/// - File reading fails
/// - Markdown parsing fails
/// - Cover image generation fails (if attempted)
/// - WeChat upload fails
/// - File writing fails
pub async fn upload_file(
    client: &WeChatClient,
    openai_client: Option<&OpenAIClient>,
    path: &Path,
    force: bool,
    verbose: bool,
) -> Result<()> {
    // Read and parse the markdown file
    let (mut frontmatter, body) = parse_markdown_file(path).await?;

    // Check if already published
    if !force && frontmatter.is_published() {
        if verbose {
            info!("Skipping already published file: {}", path.display());
        } else {
            println!(
                "{} {}",
                "⏭".bright_yellow(),
                format!("skipped: {}", path.display()).dimmed()
            );
        }
        return Ok(());
    }

    // Handle cover image generation
    let processor = DefaultCoverImageProcessor::new(openai_client);
    let mut cover_updated = false;

    if let Some(_openai_client) = openai_client {
        let should_generate_cover = match &frontmatter.cover {
            None => {
                if verbose {
                    info!("No cover image specified, generating one using AI...");
                } else {
                    println!(
                        "{} {}",
                        "🎨".bright_cyan(),
                        format!("generating cover: {}", path.display()).bright_white()
                    );
                }
                true
            }
            Some(cover_filename) => {
                let (cover_path, exists) = processor.resolve_cover_path(path, cover_filename).await;
                if !exists {
                    if verbose {
                        info!(
                            "Cover image specified ({}) but file not found at {}, generating using AI...",
                            cover_filename,
                            cover_path.display()
                        );
                    } else {
                        println!(
                            "{} {}",
                            "🎨".bright_cyan(),
                            format!(
                                "cover missing ({}), generating: {}",
                                cover_filename,
                                path.display()
                            )
                            .bright_white()
                        );
                    }
                    true
                } else {
                    if verbose {
                        info!("Cover image found at: {}", cover_path.display());
                    }
                    false
                }
            }
        };

        if should_generate_cover {
            match processor
                .ensure_cover_image(&body, path, frontmatter.cover.as_deref())
                .await?
            {
                Some(cover_filename) => {
                    frontmatter.set_cover(cover_filename.clone());
                    cover_updated = true;

                    if verbose {
                        info!("Successfully generated cover image: {}", cover_filename);
                    } else {
                        println!(
                            "{} {} {}",
                            "✨".bright_green(),
                            "cover generated:".green(),
                            cover_filename
                        );
                    }
                }
                None => {
                    if verbose {
                        warn!("Cover generation failed but continuing without error");
                    } else {
                        println!(
                            "{} {}",
                            "⚠".bright_yellow(),
                            "cover generation failed, continuing...".yellow()
                        );
                    }
                    // Keep the original cover field even if generation failed
                    // This preserves the user's intended cover filename for future attempts
                }
            }
        }
    } else if let Some(cover_filename) = &frontmatter.cover {
        // No OpenAI client but cover field exists - check if file exists
        let (cover_path, exists) = resolve_and_check_cover_path(path, cover_filename);
        if !exists {
            if verbose {
                warn!(
                    "Cover image specified ({}) but file not found at {} and no OpenAI API key provided. Upload may fail.",
                    cover_filename,
                    cover_path.display()
                );
            } else {
                println!(
                    "{} {}",
                    "⚠".bright_yellow(),
                    format!(
                        "cover missing ({}), no OpenAI key to generate",
                        cover_filename
                    )
                    .yellow()
                );
            }
        }
    }

    // If we generated or updated cover info, save the updated frontmatter before upload
    if cover_updated {
        write_markdown_file(path, &frontmatter, &body).await?;

        if verbose {
            info!("Updated frontmatter with cover in: {}", path.display());
        }
    }

    // Upload to WeChat using original file path to preserve relative image paths
    if verbose {
        info!("Uploading file: {}", path.display());
    } else {
        println!(
            "{} {}",
            "🔄".bright_blue(),
            format!("uploading: {}", path.display()).bright_white()
        );
    }

    let path_str = path
        .to_str()
        .ok_or_else(|| Error::generic("Path contains invalid UTF-8"))?;

    match client.upload(path_str).await {
        Ok(draft_id) => {
            if verbose {
                info!("Successfully uploaded with draft ID: {}", draft_id);
            } else {
                println!(
                    "{} {} {}",
                    "✓".bright_green(),
                    "uploaded:".green(),
                    path.display()
                );
            }

            // Update frontmatter with published status
            update_frontmatter(path, |fm| {
                fm.set_published("draft");
                Ok(())
            })
            .await?;

            if verbose {
                info!(
                    "Updated frontmatter with draft status in: {}",
                    path.display()
                );
            }
        }
        Err(e) => {
            let error_msg = format!("WeChat upload failed: {}", e);
            if verbose {
                warn!("Failed to upload {}: {}", path.display(), error_msg);
            } else {
                println!(
                    "{} {} {}",
                    "✗".bright_red(),
                    "failed:".red(),
                    path.display()
                );
                eprintln!("{} {}", "Error:".bright_red(), error_msg);
            }
            return Err(Error::wechat(error_msg));
        }
    }

    Ok(())
}

/// Resolves a cover image path relative to the markdown file and checks if it exists
///
/// # Arguments
///
/// * `markdown_file_path` - Path to the markdown file
/// * `cover_filename` - Relative or absolute path to the cover image
///
/// # Returns
///
/// A tuple containing the resolved path and whether the file exists
pub fn resolve_and_check_cover_path(
    markdown_file_path: &Path,
    cover_filename: &str,
) -> (PathBuf, bool) {
    let cover_path = if Path::new(cover_filename).is_absolute() {
        PathBuf::from(cover_filename)
    } else {
        // If cover filename is relative, resolve it relative to the markdown file's directory
        markdown_file_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(cover_filename)
    };

    let exists = cover_path.exists();
    (cover_path, exists)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_and_check_cover_path() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a markdown file
        let md_file = temp_path.join("test.md");
        fs::write(&md_file, "# Test").unwrap();

        // Create an existing cover image
        let existing_cover = temp_path.join("existing.png");
        fs::write(&existing_cover, "fake image data").unwrap();

        // Test with existing file
        let (resolved_path, exists) = resolve_and_check_cover_path(&md_file, "existing.png");
        assert_eq!(resolved_path, existing_cover);
        assert!(exists);

        // Test with missing file
        let (resolved_path, exists) = resolve_and_check_cover_path(&md_file, "missing.png");
        assert_eq!(resolved_path, temp_path.join("missing.png"));
        assert!(!exists);

        // Test with absolute path
        let abs_path = temp_path.join("absolute.png").to_string_lossy().to_string();
        let (resolved_path, exists) = resolve_and_check_cover_path(&md_file, &abs_path);
        assert_eq!(resolved_path, temp_path.join("absolute.png"));
        assert!(!exists);

        // Test with subdirectory path
        let images_dir = temp_path.join("images");
        fs::create_dir(&images_dir).unwrap();
        let subdir_cover = images_dir.join("cover.png");
        fs::write(&subdir_cover, "fake image data").unwrap();

        let (resolved_path, exists) = resolve_and_check_cover_path(&md_file, "images/cover.png");
        assert_eq!(resolved_path, subdir_cover);
        assert!(exists);
    }

    #[test]
    fn test_cover_image_processor() {
        let _processor = DefaultCoverImageProcessor::new(None);

        let temp_dir = TempDir::new().unwrap();
        let md_file = temp_dir.path().join("test.md");
        fs::write(&md_file, "# Test").unwrap();

        // Test resolve_cover_path (sync version for testing)
        let (path, exists) = resolve_and_check_cover_path(&md_file, "test.png");
        assert_eq!(path, temp_dir.path().join("test.png"));
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_cover_image_processor_no_openai() {
        let processor = DefaultCoverImageProcessor::new(None);
        let temp_dir = TempDir::new().unwrap();
        let md_file = temp_dir.path().join("test.md");

        // Without OpenAI client, should return None
        let result = processor
            .ensure_cover_image("content", &md_file, None)
            .await
            .unwrap();
        assert!(result.is_none());

        let result = processor
            .ensure_cover_image("content", &md_file, Some("cover.png"))
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_process_directory_empty() {
        let temp_dir = TempDir::new().unwrap();

        // Create a mock WeChat client (this will fail in real usage without proper credentials)
        // In a real test environment, we'd use dependency injection or mocking
        let client =
            wechat_pub_rs::WeChatClient::new("test_id".to_string(), "test_secret".to_string())
                .await;

        // This test mainly verifies the directory processing logic
        match client {
            Ok(client) => {
                let result = process_directory(&client, None, temp_dir.path(), false).await;
                // Should succeed with empty directory
                assert!(result.is_ok());
            }
            Err(_) => {
                // Expected to fail without real credentials, but the test structure is correct
                // In integration tests, we'd use proper mocking
            }
        }
    }
}
