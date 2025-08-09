# wx-uploader

A simple tool to upload markdown files to WeChat public account.

## Prerequisites

Before using this tool, you need to set up the following environment variables:

```bash
export WECHAT_APP_ID="your_app_id"
export WECHAT_APP_SECRET="your_app_secret"
```

For automatic cover image generation using AI:

```bash
export OPENAI_API_KEY="your_openai_api_key"
```

## Installation

```bash
cd wx-uploader
cargo build --release
```

## Usage

### Upload all markdown files in a directory

```bash
# Upload all .md files that don't have `published: true` in their frontmatter
wx-uploader .

# Upload from a specific directory
wx-uploader ./posts
```

### Upload a specific file

```bash
# Force upload a specific file (ignores publish status)
wx-uploader ./2025/08/01-chat-with-ai.md
```

## How it works

1. The tool scans for markdown files with YAML frontmatter
2. If a file doesn't have `published: true` in its frontmatter, it will be uploaded
3. If no cover image is specified and OpenAI API key is available, generates a Studio Ghibli-style cover image using GPT-4 and DALL-E
4. When specifying a single file, it will be uploaded regardless of its publish status
5. After successful upload, the frontmatter is updated with `published: draft` and the cover filename (if generated)

## Frontmatter Example

```yaml
---
title: My Article Title
published: draft  # or 'true' to skip upload
cover: cover.png  # optional, auto-generated if missing and OpenAI key is set
---

Your markdown content here...
```

## AI Cover Generation

When the `OPENAI_API_KEY` environment variable is set, the tool will automatically generate beautiful cover images for articles that don't have one specified.

### How it works:

1. **Content Analysis**: GPT-4 analyzes your markdown content to create a vivid scene description
2. **Prompt Generation**: Creates an optimized prompt for DALL-E focusing on Studio Ghibli-style artwork
3. **Image Generation**: DALL-E generates a 16:9 aspect ratio cover image with dreamy, artistic styling
4. **Auto-Save**: Downloads and saves the image in the same directory as your markdown file
5. **Metadata Update**: Updates your frontmatter with the generated cover filename

### Features:

- **Studio Ghibli Style**: Beautiful, hand-drawn animation aesthetic with soft colors and natural elements
- **Content-Aware**: Scene descriptions are based on your actual article content
- **Automatic Naming**: Generated files use unique names to prevent conflicts
- **Graceful Fallback**: Continues normal upload process if image generation fails
- **Optional**: Works without OpenAI integration - just won't generate covers

### Example Output:

For an article about "Building Rust Applications", the AI might generate a scene like:
> "A cozy workshop filled with gears and mechanical tools, bathed in warm golden light streaming through tall windows, with a craftsman's hands carefully assembling intricate clockwork mechanisms."

This becomes a beautiful Studio Ghibli-style cover image that visually represents your content.

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
```

### Generating Documentation

The codebase includes comprehensive Rust documentation for all public items:

```bash
# Generate and open documentation
cargo doc --open

# Generate documentation without dependencies
cargo doc --no-deps --open
```

### Code Quality

This project maintains high code quality standards:

- **Comprehensive Documentation**: All public functions, structs, and modules include rustdoc documentation with examples
- **Unit Tests**: Core parsing and formatting functions have dedicated unit tests
- **Integration Tests**: File I/O and directory traversal functionality is thoroughly tested
- **Error Handling**: Proper error handling with context using the `anyhow` crate
- **Type Safety**: Leverages Rust's type system for reliable frontmatter parsing and manipulation

## Notes

- Files with `published: true` will be skipped during directory scans
- Single file uploads always force upload regardless of publish status
- The tool preserves all other frontmatter fields when updating the `published` status
