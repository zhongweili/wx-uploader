//! WeChat Public Account Markdown Uploader
//!
//! A command-line tool for uploading markdown files to WeChat public accounts.
//! This tool provides functionality to:
//!
//! - Upload individual markdown files to WeChat public accounts
//! - Recursively scan directories for markdown files and upload them
//! - Parse YAML frontmatter to track publication status
//! - Automatically update frontmatter after successful uploads
//! - Skip already published files when processing directories
//!
//! ## Usage
//!
//! Set the required environment variables:
//! ```bash
//! export WECHAT_APP_ID="your_app_id"
//! export WECHAT_APP_SECRET="your_app_secret"
//! ```
//!
//! Upload a single file (forces upload regardless of published status):
//! ```bash
//! wx-uploader article.md
//! ```
//!
//! Process all markdown files in a directory:
//! ```bash
//! wx-uploader ./articles/
//! ```
//!
//! ## Frontmatter Format
//!
//! The tool recognizes YAML frontmatter in the following format:
//! ```markdown
//! ---
//! title: "Article Title"
//! published: "draft"
//! ---
//! # Article Content
//! ```
//!
//! The `published` field is used to track upload status:
//! - `null` or missing: not uploaded
//! - `"draft"`: uploaded as draft
//! - `"true"`: published (will be skipped in directory mode)

use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use wechat_pub_rs::WeChatClient;

/// Print colored help message
fn print_colored_help() {
    use colored::*;

    // Force colors to be enabled
    colored::control::set_override(true);

    println!(
        "{}",
        "A tool to upload articles to WeChat Official Account"
            .bright_white()
            .bold()
    );
    println!();
    println!(
        "{}: {} [OPTIONS] <PATH>",
        "Usage".bright_green().bold(),
        "wx-uploader".bright_cyan()
    );
    println!();
    println!("{}", "Arguments:".bright_yellow().bold());
    println!(
        "  {}  Path to markdown file or directory to upload. Files uploaded regardless of status.",
        "<PATH>".bright_cyan()
    );
    println!(
        "          Directories skip published files. Set theme and code highlighter in frontmatter - see help"
    );
    println!("          for complete lists.");
    println!();
    println!("{}", "Options:".bright_yellow().bold());
    println!(
        "  {}, {}  Enable verbose logging with detailed tracing information",
        "-v".bright_cyan(),
        "--verbose".bright_cyan()
    );
    println!(
        "  {}, {}     Print help",
        "-h".bright_cyan(),
        "--help".bright_cyan()
    );
    println!(
        "  {}, {}  Print version",
        "-V".bright_cyan(),
        "--version".bright_cyan()
    );
    println!();
    println!(
        "{}: Set {} and {} environment variables",
        "REQUIREMENTS".bright_red().bold(),
        "WECHAT_APP_ID".bright_cyan(),
        "WECHAT_APP_SECRET".bright_cyan()
    );
    println!(
        "{}: Set {} for automatic cover image generation",
        "OPTIONAL".bright_blue().bold(),
        "OPENAI_API_KEY".bright_cyan()
    );
    println!();
    println!(
        "{}: Supports {} themes and {} code highlighters via frontmatter:",
        "THEMING".bright_green().bold(),
        "8".bright_white().bold(),
        "10".bright_white().bold()
    );
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
        "github, github-dark, vscode, atom-one-light, atom-one-dark,".white()
    );
    println!(
        "                        {}",
        "solarized-light, solarized-dark, monokai, dracula, xcode".white()
    );
    println!("  {}: \"draft\"", "published".bright_cyan());
    println!(
        "  {}: \"cover.png\"     {} Auto-generated if missing with OpenAI",
        "cover".bright_cyan(),
        "#".bright_black()
    );
    println!("  {}", "---".bright_black());
    println!();
    println!("{}", "EXAMPLES:".bright_blue().bold());
    println!(
        "  {}      {} Upload single file (force)",
        "wx-uploader article.md".bright_white().bold(),
        "#".bright_black()
    );
    println!(
        "  {}     {} Process directory (skip published)",
        "wx-uploader ./articles/".bright_white().bold(),
        "#".bright_black()
    );
    println!(
        "  {}      {} Verbose logging",
        "wx-uploader -v ./blog/".bright_white().bold(),
        "#".bright_black()
    );
}

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "A tool to upload articles to WeChat Official Account",
    override_help = "Run with --help to see colored help",
    color = clap::ColorChoice::Always,
    styles = clap::builder::Styles::styled()
        .header(clap::builder::styling::AnsiColor::Yellow.on_default())
        .usage(clap::builder::styling::AnsiColor::Green.on_default())
        .literal(clap::builder::styling::AnsiColor::Green.on_default())
        .placeholder(clap::builder::styling::AnsiColor::Green.on_default())
)]
struct Args {
    #[arg(
        help = "Path to markdown file or directory to upload. Files uploaded regardless of status. Directories skip published files. Set theme and code highlighter in frontmatter - see help for complete lists."
    )]
    path: PathBuf,

    #[arg(
        short,
        long,
        help = "Enable verbose logging with detailed tracing information"
    )]
    verbose: bool,
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
/// ---
/// ```
#[derive(Debug, Deserialize, Serialize, Default)]
struct Frontmatter {
    /// The title of the article. Optional field that will be omitted from
    /// serialization if not present.
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,

    /// Publication status of the article.
    ///
    /// Common values:
    /// - `None` or missing: not uploaded
    /// - `"draft"`: uploaded as draft to WeChat
    /// - `"true"`: published (will be skipped in directory mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    published: Option<String>,

    /// Cover image filename for the article.
    /// If missing, the system will attempt to generate one using AI.
    #[serde(skip_serializing_if = "Option::is_none")]
    cover: Option<String>,

    /// Captures any additional fields in the frontmatter that are not
    /// explicitly defined in this struct.
    #[serde(flatten)]
    other: serde_yaml::Value,
}

/// OpenAI API client for generating cover images and scene descriptions
#[derive(Clone)]
struct OpenAIClient {
    api_key: String,
    http_client: reqwest::Client,
}

impl OpenAIClient {
    /// Creates a new OpenAI client with the provided API key
    fn new(api_key: String) -> Self {
        Self {
            api_key,
            http_client: reqwest::Client::new(),
        }
    }

    /// Generates a scene description from markdown content using GPT-4
    async fn generate_scene_description(&self, content: &str) -> Result<String> {
        let request_body = serde_json::json!({
            "model": "gpt-4",
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

        let response = self
            .http_client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to OpenAI API")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("OpenAI API request failed: {}", error_text);
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse OpenAI API response")?;

        let scene_description = response_json["choices"][0]["message"]["content"]
            .as_str()
            .context("Failed to extract scene description from OpenAI response")?
            .trim()
            .to_string();

        Ok(scene_description)
    }

    /// Generates a DALL-E prompt for creating a Studio Ghibli-style cover image
    fn create_dalle_prompt(&self, scene_description: &str) -> String {
        format!(
            "Create a wide, Ghibli-style image to represent this scene: {}",
            scene_description
        )
    }

    /// Generates an image using DALL-E based on the provided prompt
    async fn generate_image(&self, prompt: &str) -> Result<String> {
        let request_body = serde_json::json!({
            "model": "dall-e-3",
            "prompt": prompt,
            "size": "1792x1024",  // 16:9 aspect ratio
            "quality": "standard",
            "response_format": "url",
            "n": 1
        });

        let response = self
            .http_client
            .post("https://api.openai.com/v1/images/generations")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send image generation request to OpenAI API")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("OpenAI DALL-E API request failed: {}", error_text);
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse OpenAI DALL-E API response")?;

        let image_url = response_json["data"][0]["url"]
            .as_str()
            .context("Failed to extract image URL from OpenAI response")?
            .to_string();

        Ok(image_url)
    }

    /// Downloads an image from a URL and saves it to the specified path
    async fn download_image(&self, url: &str, file_path: &Path) -> Result<()> {
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .context("Failed to download generated image")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to download image: HTTP {}", response.status());
        }

        let image_bytes = response
            .bytes()
            .await
            .context("Failed to read image bytes")?;

        // Ensure the directory exists
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        tokio::fs::write(file_path, image_bytes)
            .await
            .with_context(|| format!("Failed to save image to: {}", file_path.display()))?;

        Ok(())
    }

    /// Generates and saves a cover image for the given markdown content
    async fn generate_cover_image(
        &self,
        content: &str,
        file_path: &Path,
        base_filename: &str,
    ) -> Result<String> {
        // Generate scene description from content
        let scene_description = self
            .generate_scene_description(content)
            .await
            .context("Failed to generate scene description")?;

        tracing::info!("Generated scene description: {}", scene_description);

        // Create DALL-E prompt
        let dalle_prompt = self.create_dalle_prompt(&scene_description);
        tracing::info!("DALL-E prompt: {}", dalle_prompt);

        // Show prompt in console for user visibility
        println!(
            "  {} {}",
            "→".bright_blue(),
            format!("Image prompt: {}", dalle_prompt).bright_white()
        );

        // Generate image
        let image_url = self
            .generate_image(&dalle_prompt)
            .await
            .context("Failed to generate image with DALL-E")?;

        // Create filename for the cover image
        let cover_filename = format!(
            "{}_cover_{}.png",
            base_filename,
            uuid::Uuid::new_v4().simple()
        );
        let cover_path = file_path
            .parent()
            .context("Failed to get parent directory")?
            .join(&cover_filename);

        // Download and save the image
        self.download_image(&image_url, &cover_path)
            .await
            .context("Failed to download and save cover image")?;

        Ok(cover_filename)
    }

    /// Generates and saves a cover image to a specific path
    async fn generate_cover_image_to_path(
        &self,
        content: &str,
        _markdown_file_path: &Path,
        target_cover_path: &Path,
    ) -> Result<()> {
        // Generate scene description from content
        let scene_description = self
            .generate_scene_description(content)
            .await
            .context("Failed to generate scene description")?;

        tracing::info!("Generated scene description: {}", scene_description);

        // Create DALL-E prompt
        let dalle_prompt = self.create_dalle_prompt(&scene_description);
        tracing::info!("DALL-E prompt: {}", dalle_prompt);

        // Show prompt in console for user visibility
        println!(
            "  {} {}",
            "→".bright_blue(),
            format!("Image prompt: {}", dalle_prompt).bright_white()
        );

        // Generate image
        let image_url = self
            .generate_image(&dalle_prompt)
            .await
            .context("Failed to generate image with DALL-E")?;

        // Download and save the image to the specified path
        self.download_image(&image_url, target_cover_path)
            .await
            .context("Failed to download and save cover image")?;

        Ok(())
    }
}

/// Resolves a cover image path relative to the markdown file and checks if it exists
fn resolve_and_check_cover_path(
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

/// Main entry point for the WeChat uploader application.
///
/// This function:
/// 1. Parses command line arguments
/// 2. Initializes logging conditionally based on verbose flag
/// 3. Reads WeChat API credentials from environment variables
/// 4. Creates a WeChat client
/// 5. Processes the input path (file or directory)
///
/// # Environment Variables
///
/// - `WECHAT_APP_ID`: WeChat application ID
/// - `WECHAT_APP_SECRET`: WeChat application secret
/// - `OPENAI_API_KEY`: OpenAI API key for generating cover images (optional)
///
/// # Command Line Options
///
/// - `-v, --verbose`: Enable detailed tracing logs
///
/// # Errors
///
/// Returns an error if:
/// - Required environment variables are not set
/// - WeChat client initialization fails
/// - File/directory processing fails
/// - The provided path is neither a file nor a directory

#[tokio::main]
async fn main() -> Result<()> {
    // Check if help is requested before clap processes args
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        print_colored_help();
        std::process::exit(0);
    }

    let args = Args::parse();

    // Initialize logging based on verbose flag
    if args.verbose {
        tracing_subscriber::fmt::init();
    }

    // WeChatClient needs app_id and app_secret from environment variables
    let app_id =
        std::env::var("WECHAT_APP_ID").context("WECHAT_APP_ID environment variable not set")?;
    let app_secret = std::env::var("WECHAT_APP_SECRET")
        .context("WECHAT_APP_SECRET environment variable not set")?;

    let client = WeChatClient::new(app_id, app_secret).await?;

    // OpenAI client is optional - only create if API key is available
    let openai_client = std::env::var("OPENAI_API_KEY").ok().map(OpenAIClient::new);

    if args.path.is_file() {
        // Force upload single file
        upload_file(
            &client,
            openai_client.as_ref(),
            &args.path,
            true,
            args.verbose,
        )
        .await?;
    } else if args.path.is_dir() {
        // Process directory
        process_directory(&client, openai_client.as_ref(), &args.path, args.verbose).await?;
    } else {
        anyhow::bail!("Path must be a file or directory");
    }

    Ok(())
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
///
/// # Examples
///
/// ```no_run
/// # use wechat_pub_rs::WeChatClient;
/// # use std::path::Path;
/// # async {
/// let client = WeChatClient::new("app_id".to_string(), "secret".to_string()).await?;
/// process_directory(&client, None, Path::new("./articles"), false).await?;
/// # Ok::<(), anyhow::Error>(())
/// # };
/// ```
async fn process_directory(
    client: &WeChatClient,
    openai_client: Option<&OpenAIClient>,
    dir: &Path,
    verbose: bool,
) -> Result<()> {
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
    {
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
///
/// # Examples
///
/// ```no_run
/// # use wechat_pub_rs::WeChatClient;
/// # use std::path::Path;
/// # async {
/// let client = WeChatClient::new("app_id".to_string(), "secret".to_string()).await?;
///
/// // Force upload regardless of status with clean output
/// upload_file(&client, None, Path::new("article.md"), true, false).await?;
///
/// // Upload only if not already published with verbose logging
/// upload_file(&client, None, Path::new("article.md"), false, true).await?;
/// # Ok::<(), anyhow::Error>(())
/// # };
/// ```
async fn upload_file(
    client: &WeChatClient,
    openai_client: Option<&OpenAIClient>,
    path: &Path,
    force: bool,
    verbose: bool,
) -> Result<()> {
    // Read and parse the markdown file
    let content = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    let (mut frontmatter, body) = parse_markdown(&content)?;

    // Check if already published
    if !force
        && let Some(published) = &frontmatter.published
        && published == "true"
    {
        if verbose {
            tracing::info!("Skipping already published file: {}", path.display());
        } else {
            println!(
                "{} {}",
                "⏭".bright_yellow(),
                format!("skipped: {}", path.display()).dimmed()
            );
        }
        return Ok(());
    }

    // Generate cover image if missing or if file doesn't exist and OpenAI client is available
    let mut needs_cover_regeneration = false;
    if let Some(openai_client) = openai_client {
        let should_generate_cover = match &frontmatter.cover {
            None => {
                // No cover field specified - generate new cover with auto filename
                if verbose {
                    tracing::info!("No cover image specified, generating one using AI...");
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
                // Cover field exists - check if the file actually exists
                let (cover_path, exists) = resolve_and_check_cover_path(path, cover_filename);
                if !exists {
                    if verbose {
                        tracing::info!(
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
                        tracing::info!("Cover image found at: {}", cover_path.display());
                    }
                    false
                }
            }
        };

        if should_generate_cover {
            match &frontmatter.cover {
                None => {
                    // Generate with auto filename
                    let base_filename = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("article");

                    match openai_client
                        .generate_cover_image(&body, path, base_filename)
                        .await
                    {
                        Ok(cover_filename) => {
                            frontmatter.cover = Some(cover_filename.clone());
                            needs_cover_regeneration = true;
                            if verbose {
                                tracing::info!(
                                    "Successfully generated cover image: {}",
                                    cover_filename
                                );
                            } else {
                                println!(
                                    "{} {} {}",
                                    "✨".bright_green(),
                                    "cover generated:".green(),
                                    cover_filename
                                );
                            }
                        }
                        Err(e) => {
                            if verbose {
                                tracing::warn!(
                                    "Failed to generate cover image: {}. Continuing without cover.",
                                    e
                                );
                            } else {
                                println!(
                                    "{} {}",
                                    "⚠".bright_yellow(),
                                    format!("cover generation failed: {}", e).yellow()
                                );
                            }
                        }
                    }
                }
                Some(cover_filename) => {
                    // Generate to the specified path from frontmatter
                    let (target_cover_path, _) = resolve_and_check_cover_path(path, cover_filename);

                    match openai_client
                        .generate_cover_image_to_path(&body, path, &target_cover_path)
                        .await
                    {
                        Ok(()) => {
                            needs_cover_regeneration = true;
                            if verbose {
                                tracing::info!(
                                    "Successfully generated cover image to: {}",
                                    target_cover_path.display()
                                );
                            } else {
                                println!(
                                    "{} {} {}",
                                    "✨".bright_green(),
                                    "cover generated:".green(),
                                    cover_filename
                                );
                            }
                        }
                        Err(e) => {
                            if verbose {
                                tracing::warn!(
                                    "Failed to generate cover image to {}: {}. Continuing without cover.",
                                    target_cover_path.display(),
                                    e
                                );
                            } else {
                                println!(
                                    "{} {}",
                                    "⚠".bright_yellow(),
                                    format!("cover generation failed: {}", e).yellow()
                                );
                            }
                            // Clear the cover field since we couldn't generate it
                            frontmatter.cover = None;
                            needs_cover_regeneration = true;
                        }
                    }
                }
            }
        }
    } else if let Some(cover_filename) = &frontmatter.cover {
        // No OpenAI client but cover field exists - check if file exists
        let (cover_path, exists) = resolve_and_check_cover_path(path, cover_filename);
        if !exists {
            if verbose {
                tracing::warn!(
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

    // If we generated a cover, save the updated frontmatter before upload
    if needs_cover_regeneration {
        let updated_content = format_markdown(&frontmatter, &body)?;
        tokio::fs::write(path, &updated_content)
            .await
            .with_context(|| format!("Failed to update file with cover: {}", path.display()))?;

        if verbose {
            tracing::info!("Updated frontmatter with cover in: {}", path.display());
        }
    }

    // Upload to WeChat using original file path to preserve relative image paths
    if verbose {
        tracing::info!("Uploading file: {}", path.display());
    } else {
        println!(
            "{} {}",
            "🔄".bright_blue(),
            format!("uploading: {}", path.display()).bright_white()
        );
    }

    match client.upload(path.to_str().unwrap()).await {
        Ok(draft_id) => {
            if verbose {
                tracing::info!("Successfully uploaded with draft ID: {}", draft_id);
            } else {
                println!(
                    "{} {} {}",
                    "✓".bright_green(),
                    "uploaded:".green(),
                    path.display()
                );
            }

            // Update frontmatter with published status
            frontmatter.published = Some("draft".to_string());

            let updated_content = format_markdown(&frontmatter, &body)?;
            tokio::fs::write(path, updated_content)
                .await
                .with_context(|| format!("Failed to update file: {}", path.display()))?;

            if verbose {
                tracing::info!("Updated frontmatter in: {}", path.display());
            }
        }
        Err(e) => {
            if verbose {
                tracing::error!("Failed to upload {}: {}", path.display(), e);
            } else {
                println!(
                    "{} {} {}",
                    "✗".bright_red(),
                    "failed:".red(),
                    path.display()
                );
                eprintln!("{} {}", "Error:".bright_red(), e);
            }
            return Err(e).context("Upload failed");
        }
    }

    Ok(())
}

/// Parses a markdown file with optional YAML frontmatter.
///
/// This function splits a markdown file into its frontmatter and body content.
/// It expects frontmatter to be delimited by `---` at the beginning and end,
/// followed by the markdown body.
///
/// # Arguments
///
/// * `content` - The complete markdown file content as a string
///
/// # Returns
///
/// A tuple containing:
/// - `Frontmatter` - Parsed YAML frontmatter (or default if none exists)
/// - `String` - The markdown body content
///
/// # Errors
///
/// Returns an error if:
/// - The regex compilation fails (should never happen with a valid pattern)
/// - The YAML frontmatter is malformed and cannot be parsed
///
/// # Examples
///
/// ```
/// # use wx_uploader::parse_markdown;
/// let content = r#"---
/// title: "My Article"
/// published: "draft"
/// ---
/// # Hello World
///
/// This is the content.
/// "#;
///
/// let (frontmatter, body) = parse_markdown(content).unwrap();
/// assert_eq!(frontmatter.title, Some("My Article".to_string()));
/// assert!(body.contains("Hello World"));
/// ```
///
/// Files without frontmatter are also supported:
///
/// ```
/// # use wx_uploader::parse_markdown;
/// let content = "# Just a title\n\nSome content.";
/// let (frontmatter, body) = parse_markdown(content).unwrap();
/// assert_eq!(frontmatter.title, None);
/// assert_eq!(body, content);
/// ```
fn parse_markdown(content: &str) -> Result<(Frontmatter, String)> {
    // Use (?s) flag to make . match newlines
    let re = Regex::new(r"(?s)^---\n(.*?)\n---\n(.*)$").context("Failed to create regex")?;

    if let Some(captures) = re.captures(content) {
        let yaml_str = captures.get(1).unwrap().as_str();
        let body = captures.get(2).unwrap().as_str();

        let frontmatter: Frontmatter =
            serde_yaml::from_str(yaml_str).context("Failed to parse frontmatter")?;

        Ok((frontmatter, body.to_string()))
    } else {
        // No frontmatter, create default
        let frontmatter = Frontmatter::default();
        Ok((frontmatter, content.to_string()))
    }
}

/// Formats frontmatter and body content back into a complete markdown file.
///
/// This function takes parsed frontmatter and markdown body content and
/// reconstructs the complete markdown file with proper YAML frontmatter
/// delimiters.
///
/// # Arguments
///
/// * `frontmatter` - The frontmatter structure to serialize
/// * `body` - The markdown body content
///
/// # Returns
///
/// A complete markdown file as a string with frontmatter and body
///
/// # Errors
///
/// Returns an error if the frontmatter cannot be serialized to YAML
///
/// # Examples
///
/// ```
/// # use wx_uploader::{Frontmatter, format_markdown};
/// let mut frontmatter = Frontmatter::default();
/// frontmatter.title = Some("My Article".to_string());
/// frontmatter.published = Some("draft".to_string());
///
/// let body = "# Hello World\n\nThis is content.";
/// let result = format_markdown(&frontmatter, body).unwrap();
///
/// assert!(result.starts_with("---\n"));
/// assert!(result.contains("title: My Article"));
/// assert!(result.contains("Hello World"));
/// ```
fn format_markdown(frontmatter: &Frontmatter, body: &str) -> Result<String> {
    let yaml = serde_yaml::to_string(frontmatter).context("Failed to serialize frontmatter")?;

    Ok(format!("---\n{yaml}---\n{body}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown_with_frontmatter() {
        let content = r#"---
title: "Test Article"
published: "draft"
author: "John Doe"
---
# Hello World

This is the content of the article.
"#;

        let (frontmatter, body) = parse_markdown(content).unwrap();

        assert_eq!(frontmatter.title, Some("Test Article".to_string()));
        assert_eq!(frontmatter.published, Some("draft".to_string()));
        assert_eq!(
            body,
            "# Hello World\n\nThis is the content of the article.\n"
        );

        // Check that additional fields are captured
        if let serde_yaml::Value::Mapping(map) = &frontmatter.other {
            assert!(map.contains_key(serde_yaml::Value::String("author".to_string())));
        } else {
            panic!("Expected mapping for other fields");
        }
    }

    #[test]
    fn test_parse_markdown_without_frontmatter() {
        let content = "# Just a Title\n\nSome content without frontmatter.";

        let (frontmatter, body) = parse_markdown(content).unwrap();

        assert_eq!(frontmatter.title, None);
        assert_eq!(frontmatter.published, None);
        assert_eq!(body, content);
    }

    #[test]
    fn test_parse_markdown_empty_frontmatter() {
        let content = r#"---

---
# Content Only

Just the body.
"#;

        let (frontmatter, body) = parse_markdown(content).unwrap();

        assert_eq!(frontmatter.title, None);
        assert_eq!(frontmatter.published, None);
        assert_eq!(body, "# Content Only\n\nJust the body.\n");
    }

    #[test]
    fn test_parse_markdown_minimal_frontmatter() {
        let content = r#"---
published: "true"
---
Content here.
"#;

        let (frontmatter, body) = parse_markdown(content).unwrap();

        assert_eq!(frontmatter.title, None);
        assert_eq!(frontmatter.published, Some("true".to_string()));
        assert_eq!(body, "Content here.\n");
    }

    #[test]
    fn test_parse_markdown_multiline_content() {
        let content = r#"---
title: "Multi-line Test"
---
# First Line

Second line

Third line with **markdown**.

```rust
fn test() {
    println!("Hello");
}
```
"#;

        let (frontmatter, body) = parse_markdown(content).unwrap();

        assert_eq!(frontmatter.title, Some("Multi-line Test".to_string()));
        assert!(body.contains("First Line"));
        assert!(body.contains("```rust"));
        assert!(body.contains("println!"));
    }

    #[test]
    fn test_format_markdown_basic() {
        let frontmatter = Frontmatter {
            title: Some("Test Title".to_string()),
            published: Some("draft".to_string()),
            ..Default::default()
        };

        let body = "# Content\n\nSome text.";

        let result = format_markdown(&frontmatter, body).unwrap();

        assert!(result.starts_with("---\n"));
        assert!(result.contains("title: Test Title"));
        assert!(result.contains("published: draft"));
        assert!(result.contains("---\n# Content"));
        assert!(result.ends_with("Some text."));
    }

    #[test]
    fn test_format_markdown_empty_frontmatter() {
        let frontmatter = Frontmatter::default();
        let body = "Just content.";

        let result = format_markdown(&frontmatter, body).unwrap();

        assert!(result.starts_with("---\n"));
        assert!(result.ends_with("---\nJust content."));
        // Should only contain the YAML null marker and body
        assert!(result.contains("null\n") || result.contains("{}\n"));
    }

    #[test]
    fn test_format_markdown_preserves_body_formatting() {
        let frontmatter = Frontmatter {
            title: Some("Formatting Test".to_string()),
            ..Default::default()
        };

        let body = r#"# Title

## Subtitle

- List item 1
- List item 2

```rust
fn main() {
    println!("Hello");
}
```

End of content."#;

        let result = format_markdown(&frontmatter, body).unwrap();

        assert!(result.contains("# Title"));
        assert!(result.contains("## Subtitle"));
        assert!(result.contains("- List item 1"));
        assert!(result.contains("```rust"));
        assert!(result.contains("fn main()"));
        assert!(result.ends_with("End of content."));
    }

    #[test]
    fn test_roundtrip_parse_and_format() {
        let original_content = r#"---
title: "Roundtrip Test"
published: "draft"
author: "Test Author"
tags:
  - rust
  - testing
---
# Test Article

This is a test article with various content.

## Section

More content here.
"#;

        // Parse the content
        let (frontmatter, body) = parse_markdown(original_content).unwrap();

        // Format it back
        let formatted = format_markdown(&frontmatter, &body).unwrap();

        // Parse again to verify
        let (parsed_frontmatter, parsed_body) = parse_markdown(&formatted).unwrap();

        // Verify the data is preserved
        assert_eq!(frontmatter.title, parsed_frontmatter.title);
        assert_eq!(frontmatter.published, parsed_frontmatter.published);
        assert_eq!(body, parsed_body);
    }

    #[test]
    fn test_parse_markdown_invalid_yaml() {
        let content = r#"---
title: "Test"
invalid: [unclosed bracket
---
Content
"#;

        let result = parse_markdown(content);
        assert!(result.is_err());

        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Failed to parse frontmatter"));
    }

    #[test]
    fn test_frontmatter_with_cover() {
        let content = r#"---
title: "Article with Cover"
published: "draft"
cover: "my-cover.png"
---
# Article Content
"#;

        let (frontmatter, body) = parse_markdown(content).unwrap();

        assert_eq!(frontmatter.title, Some("Article with Cover".to_string()));
        assert_eq!(frontmatter.published, Some("draft".to_string()));
        assert_eq!(frontmatter.cover, Some("my-cover.png".to_string()));
        assert!(body.contains("Article Content"));
    }

    #[test]
    fn test_frontmatter_without_cover() {
        let content = r#"---
title: "Article without Cover"
published: "draft"
---
# Article Content
"#;

        let (frontmatter, _) = parse_markdown(content).unwrap();

        assert_eq!(frontmatter.title, Some("Article without Cover".to_string()));
        assert_eq!(frontmatter.published, Some("draft".to_string()));
        assert_eq!(frontmatter.cover, None);
    }

    #[test]
    fn test_format_markdown_with_cover() {
        let frontmatter = Frontmatter {
            title: Some("Test Article".to_string()),
            published: Some("draft".to_string()),
            cover: Some("test-cover.png".to_string()),
            ..Default::default()
        };

        let body = "# Test Content";
        let result = format_markdown(&frontmatter, body).unwrap();

        assert!(result.contains("title: Test Article"));
        assert!(result.contains("published: draft"));
        assert!(result.contains("cover: test-cover.png"));
        assert!(result.contains("# Test Content"));
    }

    #[test]
    fn test_openai_client_create_dalle_prompt() {
        let client = OpenAIClient::new("test-key".to_string());
        let scene_description = "A serene forest with morning mist";
        let prompt = client.create_dalle_prompt(scene_description);

        assert!(prompt.contains("Ghibli-style"));
        assert!(prompt.contains("A serene forest with morning mist"));
    }

    #[test]
    fn test_resolve_and_check_cover_path() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory for testing
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
}
