use insta::assert_yaml_snapshot;
use rstest::rstest;
use std::fs;
use vite_plugin_norg_parser::{extract_metadata, extract_toc, transform};

#[rstest]
#[case::basic("tests/fixtures/basic.norg")]
#[case::code_blocks("tests/fixtures/code-blocks.norg")]
#[case::headings("tests/fixtures/headings.norg")]
#[case::images("tests/fixtures/images.norg")]
#[case::links("tests/fixtures/links.norg")]
fn test_norg_fixture_files(#[case] fixture_path: &str) {
    let content = fs::read_to_string(fixture_path)
        .unwrap_or_else(|_| panic!("Failed to read {fixture_path}"));
    let ast = rust_norg::parse_tree(&content)
        .unwrap_or_else(|_| panic!("Failed to parse {fixture_path}"));

    let html = transform(&ast);
    let toc = extract_toc(&ast);

    let metadata = extract_metadata(&ast);
    assert_yaml_snapshot!(fixture_path, (html, toc, metadata));
}
