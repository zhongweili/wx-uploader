# wx-uploader

A command-line tool to upload markdown files to WeChat public account with automatic AI-powered cover image generation. See [README_CN.md](README_CN.md) for Chinese version.

## Installation

Install directly from crates.io:

```bash
cargo install wx-uploader
```

Or build from source:

```bash
git clone https://github.com/tyrchen/wx-uploader.git
cd wx-uploader
cargo install --path .
```

## Prerequisites

You can configure wx-uploader in two ways: environment variables (simple) or configuration files (multi-account).

### Method 1: Environment Variables (Single Account)

```bash
# Required: WeChat public account credentials
export WECHAT_APP_ID="your_app_id"
export WECHAT_APP_SECRET="your_app_secret"

# Optional: AI provider configuration for automatic cover image generation
# Option 1: OpenAI (default)
export OPENAI_API_KEY="your_openai_api_key"

# Option 2: Google Gemini (supports text and image generation)
export AI_PROVIDER="gemini"
export GEMINI_API_KEY="your_gemini_api_key"
```

### Method 2: Configuration Files (Multi-Account Support)

Create a configuration file to manage multiple WeChat accounts:

```bash
# Generate example configuration file
wx-uploader --init-config config.yaml
```

This creates a configuration file like:

```yaml
# Example configuration file
accounts:
  personal:
    name: "personal"
    app_id: "your_personal_app_id_here"
    app_secret: "your_personal_app_secret_here"
    description: "Personal WeChat public account"
  
  work:
    name: "work"
    app_id: "your_work_app_id_here"
    app_secret: "your_work_app_secret_here"
    description: "Work WeChat public account"

default_account: "personal"

ai_provider:
  provider: "openai"  # or "gemini"
  openai:
    api_key: "your_openai_api_key_here"
  gemini:
    api_key: "your_gemini_api_key_here"

settings:
  verbose: false
```

## Usage

### Basic Usage (Single Account)

```bash
# Upload all .md files that don't have `published: true` in their frontmatter
wx-uploader .

# Upload from a specific directory
wx-uploader ./posts

# Enable verbose output
wx-uploader --verbose ./posts

# Use specific AI provider
wx-uploader --provider gemini ./posts

# Use custom AI API key
wx-uploader --ai-key your_custom_key ./posts
```

### Multi-Account Usage

```bash
# List available accounts in config file
wx-uploader --config config.yaml --list-accounts

# Upload using specific account from config
wx-uploader --config config.yaml --account work ./posts

# Upload using default account
wx-uploader --config config.yaml ./posts

# Override AI provider for this upload
wx-uploader --config config.yaml --provider gemini --account personal ./posts
```

### Upload a Specific File

```bash
# Force upload a specific file (ignores publish status)
wx-uploader ./2025/08/01-chat-with-ai.md

# Upload specific file using multi-account config
wx-uploader --config config.yaml --account work ./article.md
```

### Command-Line Options

```bash
wx-uploader [OPTIONS] [PATH]

Options:
    -c, --config <FILE>        Configuration file path (YAML or JSON)
    -a, --account <NAME>       Account name to use from config file
    -p, --provider <PROVIDER>  AI provider: openai, gemini [default: openai]
        --ai-key <KEY>         AI API key (overrides config/env)
    -v, --verbose              Enable verbose output
        --list-accounts        List available accounts from config
        --init-config <FILE>   Generate example configuration file
    -h, --help                 Print help information
    -V, --version              Print version information
```

## How it works

1. The tool scans for markdown files with YAML frontmatter
2. If a file doesn't have `published: true` in its frontmatter, it will be uploaded
3. If no cover image is specified and AI provider is configured, generates a Studio Ghibli-style cover image using AI
4. When specifying a single file, it will be uploaded regardless of its publish status
5. After successful upload, the frontmatter is updated with `published: draft` and the cover filename (if generated)

## Frontmatter Example

```yaml
---
title: My Article Title
published: draft  # or 'true' to skip upload
cover: cover.png  # optional, auto-generated if missing and AI provider is set
description: Article description
author: Author Name
theme: lapis  # optional theme
---

Your markdown content here...
```

## AI Cover Generation

When an AI provider is configured (OpenAI or Gemini), the tool will automatically generate beautiful cover images for articles that don't have one specified.

### How it works

1. **Content Analysis**: AI model analyzes your markdown content to create a vivid scene description
2. **Prompt Generation**: Creates an optimized prompt for image generation focusing on Studio Ghibli-style artwork
3. **Image Generation**: AI image model generates a high-quality 16:9 aspect ratio cover image
4. **Auto-Save**: Downloads and saves the image in the same directory as your markdown file
5. **Metadata Update**: Updates your frontmatter with the generated cover filename

### Features

- **Studio Ghibli Style**: Beautiful, artistic aesthetic with soft colors and natural elements
- **Content-Aware**: Scene descriptions are based on your actual article content
- **High Quality**: 1536x1024 resolution images optimized for web display
- **Automatic Naming**: Generated files use unique names to prevent conflicts
- **Graceful Fallback**: Continues normal upload process if image generation fails
- **Base64 Support**: Handles both URL and base64-encoded image responses

### Example Output

For an article about "Building Rust Applications", the AI might generate a scene like:
> "A cozy workshop filled with intricate gears and glowing mechanical tools, where a craftsman carefully assembles clockwork mechanisms. Warm golden light streams through tall windows, illuminating floating rust particles that sparkle like fireflies in the dusty air."

This becomes a beautiful Studio Ghibli-style cover image that visually represents your content.

### Supported AI Providers

**OpenAI (Default)**:
- Models: GPT-4o-mini for text, DALL-E 3 for images
- Setup: `export OPENAI_API_KEY="your_key"`
- Direct access to OpenAI's latest models

**Google Gemini**:
- Models: Gemini text models and Imagen for image generation
- Setup: `export AI_PROVIDER="gemini"` and `export GEMINI_API_KEY="your_key"`  
- Access to multiple AI providers through a single API
- Alternative models: `anthropic/claude-3.5-sonnet` for text generation

**CLI Override**:
```bash
# Use Gemini with CLI
wx-uploader --provider gemini --ai-key your_key ./posts

# Use OpenAI with custom key
wx-uploader --provider openai --ai-key your_key ./posts
```

## Features

- 📝 **Batch Upload**: Process entire directories of markdown files
- 🎨 **AI Cover Generation**: Multiple AI providers (OpenAI, Gemini) for cover images
- 🔄 **Smart Processing**: Skip already published articles
- 📊 **Progress Tracking**: Clear console output with colored status indicators
- 🛡️ **Error Recovery**: Graceful handling of API failures
- 🔐 **Secure**: API keys stored in environment variables only
- ⚡ **Flexible**: Multiple AI providers with CLI override support

## Development

### Running Tests

The project includes comprehensive unit and integration tests:

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test test_frontmatter

# Run integration tests only
cargo test --test integration_tests
```

### Code Quality

```bash
# Run clippy for linting
cargo clippy --all-targets --all-features

# Check for security vulnerabilities
cargo audit

# Format code
cargo fmt

# Generate documentation
cargo doc --open
```

### Project Structure

```
wx-uploader/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Public API
│   ├── cli.rs           # Command-line interface and multi-account management
│   ├── error.rs         # Error handling
│   ├── models.rs        # Data structures and configuration
│   ├── markdown.rs      # Markdown parsing
│   ├── providers.rs     # Universal AI provider integration
│   ├── openai.rs        # Legacy OpenAI integration (deprecated)
│   ├── output.rs        # Console output formatting
│   └── wechat.rs        # WeChat API integration
├── examples/
│   ├── config.yaml      # Example YAML configuration
│   └── config.json      # Example JSON configuration
└── tests/
    └── integration_tests.rs  # Integration tests
```

## Configuration Priority

The tool follows this priority order for configuration:

1. **Command-line flags** (highest priority): `--provider`, `--ai-key`, `--verbose`
2. **Configuration file**: Settings from `--config` file
3. **Environment variables**: `WECHAT_APP_ID`, `OPENAI_API_KEY`, etc.
4. **Default values** (lowest priority)

## Multi-Account Workflows

### Setup Workflow
```bash
# 1. Generate configuration template
wx-uploader --init-config my-accounts.yaml

# 2. Edit the file with your actual credentials
# Replace placeholder values with real app_id, app_secret, and API keys

# 3. List configured accounts
wx-uploader --config my-accounts.yaml --list-accounts

# 4. Test upload with specific account
wx-uploader --config my-accounts.yaml --account personal ./test-article.md
```

### Daily Usage Workflow
```bash
# Upload personal blog posts
wx-uploader --config my-accounts.yaml --account personal ./blog/

# Upload work articles with different AI provider
wx-uploader --config my-accounts.yaml --account work --provider gemini ./work-posts/

# Quick upload with default account
wx-uploader --config my-accounts.yaml ./quick-post.md
```

## Notes

- Files with `published: true` will be skipped during directory scans
- Single file uploads always force upload regardless of publish status
- The tool preserves all other frontmatter fields when updating
- Cover images are saved in the same directory as the markdown file
- Supports both string (`"true"`) and boolean (`true`) values for the published field
- Configuration files support both YAML (`.yaml`, `.yml`) and JSON (`.json`) formats
- Account switching is seamless and doesn't require restarting the tool

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
