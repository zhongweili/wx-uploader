use std::fs;
use tempfile::TempDir;

/// Integration tests for file processing functionality.
/// These tests focus on file I/O and directory traversal without
/// making actual WeChat API calls.

#[test]
fn test_markdown_file_discovery() {
    // Create a temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create some test files
    fs::create_dir_all(base_path.join("articles")).unwrap();
    fs::create_dir_all(base_path.join("docs")).unwrap();

    // Create markdown files
    fs::write(
        base_path.join("articles/test1.md"),
        "---\ntitle: Test 1\n---\n# Content 1",
    )
    .unwrap();

    fs::write(
        base_path.join("articles/test2.md"),
        "---\ntitle: Test 2\npublished: true\n---\n# Content 2",
    )
    .unwrap();

    fs::write(
        base_path.join("docs/test3.md"),
        "# Simple markdown without frontmatter",
    )
    .unwrap();

    // Create non-markdown files that should be ignored
    fs::write(base_path.join("articles/readme.txt"), "Not markdown").unwrap();
    fs::write(base_path.join("docs/config.json"), "{}").unwrap();

    // Use walkdir to find markdown files (same logic as the main application)
    let mut md_files = Vec::new();
    for entry in walkdir::WalkDir::new(base_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
    {
        md_files.push(entry.path().to_path_buf());
    }

    // Should find exactly 3 markdown files
    assert_eq!(md_files.len(), 3);

    // Verify the files we expect are found
    let file_names: Vec<String> = md_files
        .iter()
        .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
        .collect();

    assert!(file_names.contains(&"test1.md".to_string()));
    assert!(file_names.contains(&"test2.md".to_string()));
    assert!(file_names.contains(&"test3.md".to_string()));
}

#[test]
fn test_file_content_processing() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_article.md");

    let original_content = r#"---
title: "Integration Test Article"
published: "draft"
author: "Test Suite"
---
# Test Article

This is a test article for integration testing.

## Features

- Markdown parsing
- Frontmatter handling
- File I/O operations
"#;

    // Write the test file
    fs::write(&file_path, original_content).unwrap();

    // Read and verify the file can be processed
    let read_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(read_content, original_content);

    // Verify the file exists and is readable
    assert!(file_path.exists());
    assert!(file_path.is_file());
}

#[test]
fn test_directory_vs_file_detection() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create a file and a directory
    let file_path = base_path.join("test.md");
    let dir_path = base_path.join("test_dir");

    fs::write(&file_path, "# Test file").unwrap();
    fs::create_dir(&dir_path).unwrap();

    // Test file detection
    assert!(file_path.is_file());
    assert!(!file_path.is_dir());

    // Test directory detection
    assert!(dir_path.is_dir());
    assert!(!dir_path.is_file());

    // Test non-existent path
    let non_existent = base_path.join("does_not_exist");
    assert!(!non_existent.is_file());
    assert!(!non_existent.is_dir());
}

#[test]
fn test_nested_directory_structure() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create a nested directory structure
    let nested_path = base_path.join("level1/level2/level3");
    fs::create_dir_all(&nested_path).unwrap();

    // Add markdown files at different levels
    fs::write(
        base_path.join("level1/top.md"),
        "---\ntitle: Top Level\n---\n# Top",
    )
    .unwrap();

    fs::write(
        base_path.join("level1/level2/middle.md"),
        "---\ntitle: Middle Level\n---\n# Middle",
    )
    .unwrap();

    fs::write(
        base_path.join("level1/level2/level3/deep.md"),
        "---\ntitle: Deep Level\n---\n# Deep",
    )
    .unwrap();

    // Count markdown files using walkdir
    let md_count = walkdir::WalkDir::new(base_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .count();

    assert_eq!(md_count, 3);
}

#[test]
fn test_file_with_no_extension() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create files with and without extensions
    fs::write(base_path.join("with_ext.md"), "# Has extension").unwrap();
    fs::write(base_path.join("no_ext"), "# No extension").unwrap();
    fs::write(base_path.join("wrong_ext.txt"), "# Wrong extension").unwrap();

    // Only .md files should be found
    let md_files: Vec<_> = walkdir::WalkDir::new(base_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .collect();

    assert_eq!(md_files.len(), 1);
    assert!(md_files[0].path().file_name().unwrap() == "with_ext.md");
}
