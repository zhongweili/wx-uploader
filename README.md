# wx-uploader

A simple tool to upload markdown files to WeChat public account.

## Prerequisites

Before using this tool, you need to set up the following environment variables:

```bash
export WECHAT_APP_ID="your_app_id"
export WECHAT_APP_SECRET="your_app_secret"
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
3. When specifying a single file, it will be uploaded regardless of its publish status
4. After successful upload, the frontmatter is updated with `published: draft`

## Frontmatter Example

```yaml
---
title: My Article Title
published: draft  # or 'true' to skip upload
---

Your markdown content here...
```

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
