use rust_norg::NorgAST::VerbatimRangedTag;
use serde_json::{json, Value};
use std::fs;
use vite_plugin_norg_parser::{convert_nodes, extract_metadata};

#[test]
fn test_extract_metadata_empty() {
    let ast = vec![];
    let result = extract_metadata(&ast);
    assert_eq!(result, Value::Null);
}

#[test]
fn test_extract_metadata_basic() {
    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "title: Test Document\nauthor: Drake Bott".to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    println!("Metadata result: {:?}", metadata);

    let expected = json!({
        "title": "Test Document",
        "author": "Drake Bott"
    });
    assert_eq!(metadata, expected);
}

#[test]
fn test_extract_metadata_from_actual_file() {
    // Parse the actual basic.norg file
    let content = fs::read_to_string("tests/fixtures/basic.norg").unwrap();
    println!("File content:\n{}", content);

    let ast = rust_norg::parse_tree(&content).unwrap();
    println!("AST: {:?}", ast);

    let metadata = extract_metadata(&ast);
    println!("Extracted metadata: {:?}", metadata);

    // This should match what we expect from the test
    let expected = json!({
        "title": "Basic Norg",
        "author": "Drake Bott"
    });
    assert_eq!(metadata, expected);
}

#[test]
fn test_parse_norg_internal() {
    // Test the internal parsing logic
    let content = fs::read_to_string("tests/fixtures/basic.norg").unwrap();
    println!("File content:\n{}", content);

    let ast = rust_norg::parse_tree(&content).unwrap();
    let (html, toc) = convert_nodes(&ast);
    let metadata = extract_metadata(&ast);

    println!("Metadata: {:?}", metadata);
    println!("HTML length: {}", html.len());
    println!("TOC entries: {}", toc.len());

    // Check that metadata is properly extracted
    assert!(!metadata.is_null());
    let expected = json!({
        "title": "Basic Norg",
        "author": "Drake Bott"
    });
    assert_eq!(metadata, expected);
}

#[test]
fn test_extract_metadata_values() {
    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "title: Test Title\ndescription: A description with: colon".to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    let expected = json!({
        "title": "Test Title",
        "description": "A description with: colon"
    });
    assert_eq!(metadata, expected);
}

#[test]
fn test_extract_metadata_empty_lines() {
    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "title: Test\nauthor: Author".to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    let expected = json!({
        "title": "Test",
        "author": "Author"
    });
    assert_eq!(metadata, expected);
}

#[test]
fn test_extract_metadata_all_valid() {
    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "title: Valid\nauthor: Test Author".to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    let expected = json!({
        "title": "Valid",
        "author": "Test Author"
    });
    assert_eq!(metadata, expected);
}

#[test]
fn test_extract_metadata_non_document_meta() {
    let ast = vec![
        VerbatimRangedTag {
            name: vec!["code".to_string()],
            content: "title: Should not be extracted".to_string(),
            parameters: Default::default(),
        },
        VerbatimRangedTag {
            name: vec!["document".to_string(), "other".to_string()],
            content: "author: Should not be extracted".to_string(),
            parameters: Default::default(),
        },
    ];

    let result = extract_metadata(&ast);
    assert_eq!(result, Value::Null);
}

#[test]
fn test_extract_metadata_with_types() {
    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "title: Test Document\nversion: 42\npublished: true\ntags: [\n  rust\n  norg\n]"
            .to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    let expected = json!({
        "title": "Test Document",
        "version": 42.0,
        "published": true,
        "tags": ["rust", "norg"]
    });
    assert_eq!(metadata, expected);
}

#[test]
fn test_extract_metadata_nested_keys() {
    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "author: {\n  name: John Doe\n  email: john@example.com\n}".to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    let expected = json!({
        "author": {
            "name": "John Doe",
            "email": "john@example.com"
        }
    });
    assert_eq!(metadata, expected);
}
