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
#[case::inline_css("tests/fixtures/inline-css.norg")]
fn test_norg_fixture_files(#[case] fixture_path: &str) {
    let content = fs::read_to_string(fixture_path)
        .unwrap_or_else(|_| panic!("Failed to read {fixture_path}"));
    let ast = rust_norg::parse_tree(&content)
        .unwrap_or_else(|_| panic!("Failed to parse {fixture_path}"));

    let (html_parts, _inline_components, inline_css) =
        transform(&ast, None).unwrap_or_else(|_| panic!("Failed to transform {fixture_path}"));
    let html = html_parts.join("");
    let toc = extract_toc(&ast);

    let metadata = extract_metadata(&ast);
    assert_yaml_snapshot!(fixture_path, (html, toc, metadata, inline_css));
}

#[test]
fn test_inline_css_no_components() {
    let content = r#"
@inline css
.test { color: red; }
@end
"#;
    let ast = rust_norg::parse_tree(content).unwrap();
    let (html_parts, inline_components, inline_css) = transform(&ast, None).unwrap();

    assert!(
        inline_components.is_empty(),
        "CSS blocks should not create inline components"
    );
    assert!(
        inline_css.contains(".test { color: red; }"),
        "CSS content should be collected in inline_css"
    );
    // With no inlines, html_parts should have exactly 1 part
    assert_eq!(html_parts.len(), 1);
}
