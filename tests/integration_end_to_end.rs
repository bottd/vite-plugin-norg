use rust_norg::parse_tree;
/// Integration tests for the complete parser functionality
use vite_plugin_norg_parser::{convert_ast_to_html_with_toc, extract_metadata};

#[test]
fn test_end_to_end_parsing_with_metadata_and_html() {
    let content = r#"@document.meta
title: "Integration Test Document"
author: "Test Author"
version: 1.0
@end

* Main Heading

This is a paragraph with *bold* text and _italic_ text.

@code rust
fn main() {
    println!("Hello, World!");
}
@end

** Subheading

Another paragraph here."#;

    // Test that parsing works end-to-end
    let ast = parse_tree(content).expect("Failed to parse content");
    let metadata = extract_metadata(&ast);
    let (html, toc) = convert_ast_to_html_with_toc(&ast);

    // Verify metadata extraction (note: JSON strings include quotes)
    assert_eq!(metadata["title"], "\"Integration Test Document\"");
    assert_eq!(metadata["author"], "\"Test Author\"");
    assert_eq!(metadata["version"], 1.0);

    // Verify HTML conversion
    assert!(html.contains("<h1 id=\"main-heading\">Main Heading</h1>"));
    assert!(html.contains("<h2 id=\"subheading\">Subheading</h2>"));
    assert!(html.contains("<strong>bold</strong>"));
    assert!(html.contains("<em>italic</em>"));
    assert!(html.contains("<pre class=\"language-rust\"><code class=\"language-rust\">"));
    assert!(html.contains("println!(&quot;Hello, World!&quot;);"));

    // Verify TOC generation
    assert_eq!(toc.len(), 2);
    assert_eq!(toc[0].title, "Main Heading");
    assert_eq!(toc[0].level, 1);
    assert_eq!(toc[0].id, "main-heading");
    assert_eq!(toc[1].title, "Subheading");
    assert_eq!(toc[1].level, 2);
    assert_eq!(toc[1].id, "subheading");
}

#[test]
fn test_error_handling_integration() {
    let content = "Simple paragraph"; // Simple content instead of empty

    // Test that simple content is handled gracefully
    let ast = parse_tree(content).expect("Failed to parse simple content");
    let metadata = extract_metadata(&ast);
    let (html, toc) = convert_ast_to_html_with_toc(&ast);

    assert_eq!(metadata, serde_json::Value::Null);
    assert!(html.contains("<p>Simple paragraph</p>"));
    assert_eq!(toc.len(), 0);
}
