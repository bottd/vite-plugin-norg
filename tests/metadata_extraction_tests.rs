use serde_json::Value;
use vite_plugin_norg_parser::extract_metadata;

/// Helper to check if a JSON value contains expected key-value pairs
fn assert_json_contains(json: &Value, key: &str, expected: &str) {
    assert_eq!(json[key].as_str().unwrap(), expected);
}

#[test]
fn test_extract_metadata_empty() {
    let ast = vec![];
    let result = extract_metadata(&ast);
    assert_eq!(result, Value::Null);
}

#[test]
fn test_extract_metadata_basic() {
    use rust_norg::NorgAST::VerbatimRangedTag;

    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "title: Test Document\nauthor: John Doe".to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    assert_json_contains(&metadata, "title", "Test Document");
    assert_json_contains(&metadata, "author", "John Doe");
}

#[test]
fn test_extract_metadata_values() {
    use rust_norg::NorgAST::VerbatimRangedTag;

    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "title: Test Title\ndescription: A description with: colon".to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    assert_json_contains(&metadata, "title", "Test Title");
    assert_json_contains(&metadata, "description", "A description with: colon");
}

#[test]
fn test_extract_metadata_empty_lines() {
    use rust_norg::NorgAST::VerbatimRangedTag;

    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "title: Test\nauthor: Author".to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    assert_json_contains(&metadata, "title", "Test");
    assert_json_contains(&metadata, "author", "Author");
}

#[test]
fn test_extract_metadata_all_valid() {
    use rust_norg::NorgAST::VerbatimRangedTag;

    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "title: Valid\nauthor: Test Author".to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    assert_json_contains(&metadata, "title", "Valid");
    assert_json_contains(&metadata, "author", "Test Author");
}

#[test]
fn test_extract_metadata_non_document_meta() {
    use rust_norg::NorgAST::VerbatimRangedTag;

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
    use rust_norg::NorgAST::VerbatimRangedTag;

    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "title: Test Document\nversion: 42\npublished: true\ntags: [\n  rust\n  norg\n]"
            .to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    assert_json_contains(&metadata, "title", "Test Document");
    assert_eq!(metadata["version"], 42.0);
    assert_eq!(metadata["published"], true);

    let tags = metadata["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0], "rust");
    assert_eq!(tags[1], "norg");
}

#[test]
fn test_extract_metadata_nested_keys() {
    use rust_norg::NorgAST::VerbatimRangedTag;

    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "author: {\n  name: John Doe\n  email: john@example.com\n}".to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    assert_json_contains(&metadata["author"], "name", "John Doe");
    assert_json_contains(&metadata["author"], "email", "john@example.com");
}
