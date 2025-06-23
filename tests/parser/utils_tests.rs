use vite_plugin_norg_parser::slug_from_text;

#[test]
fn test_slug_from_text() {
    assert_eq!(slug_from_text("Hello World"), "hello-world");
    assert_eq!(slug_from_text("Multiple   Spaces"), "multiple---spaces");
    assert_eq!(
        slug_from_text("Special!@#$%Characters"),
        "special-----characters"
    );
    assert_eq!(slug_from_text(""), "");
}

#[test]
fn test_slug_from_text_edge_cases() {
    assert_eq!(slug_from_text("---"), "");
    assert_eq!(slug_from_text("123"), "123");
    assert_eq!(slug_from_text("Test-Case"), "test-case");
    assert_eq!(slug_from_text("  Leading Spaces  "), "leading-spaces");
    assert_eq!(slug_from_text("CamelCase"), "camelcase");
}
