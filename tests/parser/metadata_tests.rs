use insta::assert_yaml_snapshot;
use rust_norg::NorgAST::VerbatimRangedTag;
use std::fs;
use vite_plugin_norg_parser::extract_metadata;

#[test]
fn test_extract_metadata_empty() {
    let ast = vec![];
    let result = extract_metadata(&ast);
    assert_yaml_snapshot!(result);
}

#[test]
fn test_extract_metadata_basic() {
    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "title: Test Document\nauthor: Drake Bott".to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    assert_yaml_snapshot!(metadata);
}

#[test]
fn test_extract_metadata_from_file() {
    let content =
        fs::read_to_string("tests/fixtures/basic.norg").expect("Failed to read basic.norg fixture");
    let ast = rust_norg::parse_tree(&content).expect("Failed to parse basic.norg content");
    let metadata = extract_metadata(&ast);
    assert_yaml_snapshot!(metadata);
}

#[test]
fn test_parse_norg_internal() {
    let content =
        fs::read_to_string("tests/fixtures/basic.norg").expect("Failed to read basic.norg fixture");
    let ast = rust_norg::parse_tree(&content).expect("Failed to parse basic.norg content");
    let metadata = extract_metadata(&ast);

    assert_yaml_snapshot!(metadata);
}

#[test]
fn test_extract_metadata_values() {
    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "title: Test Title\ndescription: A description with: colon".to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    assert_yaml_snapshot!(metadata);
}

#[test]
fn test_extract_metadata_empty_lines() {
    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "title: Test\nauthor: Drake Bott".to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    assert_yaml_snapshot!(metadata);
}

#[test]
fn test_extract_metadata_all_valid() {
    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "title: Valid\nauthor: Drake Bott".to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    assert_yaml_snapshot!(metadata);
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
    assert_yaml_snapshot!(result);
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
    assert_yaml_snapshot!(metadata);
}

#[test]
fn test_extract_metadata_nested_keys() {
    let ast = vec![VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "author: {\n  name: Drake Bott\n  email: drake@example.com\n}".to_string(),
        parameters: Default::default(),
    }];

    let metadata = extract_metadata(&ast);
    assert_yaml_snapshot!(metadata);
}
