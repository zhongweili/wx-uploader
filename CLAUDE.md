# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

wx-uploader is a Rust CLI tool that uploads markdown articles to WeChat public accounts with automatic AI-powered cover image generation. The tool supports multi-account management, processes markdown files with YAML frontmatter, automatically generates cover images using multiple AI providers (OpenAI, Gemini), and integrates with WeChat's publishing platform.

## Development Commands

### Building and Testing
- `cargo build` - Build the project
- `cargo test` - Run all tests with standard test runner
- `cargo nextest run --all-features` - Run tests with nextest (preferred test runner)
- `cargo test --test integration_tests` - Run integration tests specifically
- `cargo test test_frontmatter` - Run specific test module

### Code Quality and Security
- `cargo clippy --all-targets --all-features` - Run linting checks
- `cargo fmt` - Format code according to Rust style guidelines
- `cargo audit` - Check for security vulnerabilities in dependencies
- `cargo doc --open` - Generate and open documentation

### Makefile Commands
- `make build` - Build the project
- `make test` - Run tests using nextest
- `make release` - Create a full release (tag, changelog, publish)

### CLI Commands and Usage

**Basic Upload Commands**:
- `wx-uploader ./posts` - Upload directory using environment variables
- `wx-uploader ./article.md` - Upload single file
- `wx-uploader --verbose ./posts` - Upload with detailed output

**Multi-Account Configuration**:
- `wx-uploader --init-config config.yaml` - Generate example configuration file
- `wx-uploader --config config.yaml --list-accounts` - List configured accounts
- `wx-uploader --config config.yaml --account work ./posts` - Upload using specific account

**AI Provider Override**:
- `wx-uploader --provider gemini ./posts` - Use Gemini instead of default OpenAI
- `wx-uploader --ai-key custom_key ./posts` - Override AI API key for this upload
- `wx-uploader --config config.yaml --provider openai --ai-key custom_key ./posts` - Combined config and override

## Architecture

### Core Components

**Main Application Flow (`main.rs`, `cli.rs`)**:
- Multi-account command-line argument parsing with clap
- Configuration file and environment variable loading
- Account selection and management (`--account`, `--config`, `--list-accounts`)  
- Configuration file generation (`--init-config`)
- Uploader initialization with account-specific credentials
- Colored CLI output and help system with structured formatting

**Library Core (`lib.rs`)**:
- `WxUploader` struct orchestrates WeChat and Universal AI clients
- Public API for single file upload and directory processing
- Multi-account configuration management with account switching capability
- Account listing and current account tracking functionality
- WeChat token refresh capability with account-specific tokens

**Data Models (`models.rs`)**:
- `ConfigFile` struct for YAML/JSON configuration file management
- `WeChatAccount` struct for individual account credentials and metadata
- `AiProviderConfig` struct for AI provider configuration with multiple keys
- `GlobalSettings` struct for project-wide settings
- `Config` struct for runtime configuration with account selection
- `AiProvider` enum for OpenAI and Gemini configurations
- `Frontmatter` struct for YAML frontmatter parsing with serde and validation
- Theme validation (8 themes: default, lapis, maize, orangeheart, phycat, pie, purple, rainbow)
- Code highlighter validation (10 highlighters: github, github-dark, vscode, etc.)
- Publication status tracking with boolean and string support

**WeChat Integration (`wechat.rs`)**:
- Re-exports `WeChatClient` from `wechat-pub-rs` library
- File upload workflow orchestration with multi-step validation
- Directory processing with published file filtering using walkdir
- Cover image path resolution and validation
- Trait-based design (`WeChatUploader`, `LocalCoverImageProcessor`) with testability
- Structured output formatting through output module integration

**AI Provider Integration (`providers.rs`, legacy `openai.rs`)**:
- Universal AI client supporting multiple providers (OpenAI, Gemini)
- GPT-4o-mini and DALL-E 3 for OpenAI; Gemini 2.5 Flash and Imagen for Google
- Scene description generation from markdown content
- Studio Ghibli-style cover image generation with Base64 support
- Trait-based design (`SceneDescriptionGenerator`, `ImageGenerator`, `CoverImageProcessor`)

**Markdown Processing (`markdown.rs`)**:
- YAML frontmatter parsing and updating
- Content extraction and file writing
- Frontmatter preservation during updates

### Key Design Patterns

- **Async/Await**: All I/O operations are async using tokio
- **Error Handling**: Custom `Error` enum with anyhow integration
- **Trait-Based Design**: Core functionality abstracted behind traits for testing
- **Multi-Account Architecture**: Flexible configuration with file-based and environment-based options
- **Configuration Priority System**: CLI flags > config files > environment variables > defaults
- **Frontmatter Management**: Automatic status tracking in YAML headers

### Dependencies and Integration

**Core Dependencies**:
- `wechat-pub-rs`: WeChat public account API integration
- `tokio`: Async runtime for all I/O operations
- `reqwest`: HTTP client for AI provider API calls
- `serde` + `serde_yaml`: YAML frontmatter serialization
- `clap`: Command-line argument parsing with colored output and environment support
- `anyhow` + `thiserror`: Error handling and propagation
- `walkdir`: Recursive directory traversal
- `uuid`: Unique filename generation for cover images
- `base64`: Base64 image data handling
- `regex`: Pattern matching for content processing
- `tracing`: Structured logging throughout the application

**Configuration Methods**:

*Environment Variables (Single Account)*:
- `WECHAT_APP_ID` (required): WeChat application ID
- `WECHAT_APP_SECRET` (required): WeChat application secret  
- `OPENAI_API_KEY` (optional): OpenAI API key for cover image generation
- `GEMINI_API_KEY` (optional): Google Gemini API key for cover image generation
- `AI_PROVIDER` (optional): Which provider to use ("openai" or "gemini", defaults to "openai")

*Configuration Files (Multi-Account)*:
- YAML (`.yaml`, `.yml`) or JSON (`.json`) format support
- Multiple WeChat accounts with individual credentials
- Per-project AI provider settings with multiple API keys
- Default account selection and global settings
- Account descriptions and metadata for better organization

### File Processing Logic

**Single File Upload**: Force uploads regardless of published status
**Directory Processing**: Recursively finds .md files, skips those with `published: true`
**Frontmatter Updates**: Automatically sets `published: draft` and cover filename after successful upload
**Cover Image Generation**: AI-generated images saved alongside markdown files with unique filenames

### Testing Structure

- Unit tests embedded in each module using `#[cfg(test)]`
- Comprehensive integration tests in `tests/integration_tests.rs`
- Mock-friendly trait-based architecture for AI and WeChat clients
- Temporary file handling using `tempfile` crate
- Concurrent testing scenarios and edge case validation
- Unicode filename and path resolution testing

## Configuration Notes

- Uses Rust 2024 edition
- Security scanning configured with `deny.toml` for license and vulnerability checks
- Clippy and formatting enforced as part of quality checks
- MIT licensed open source project