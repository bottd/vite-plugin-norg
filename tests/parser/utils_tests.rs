use vite_plugin_norg_parser::utils::into_slug;

#[test]
fn test_into_slug() {
    assert_eq!(into_slug("Hello World"), "hello-world");
    assert_eq!(into_slug("Multiple   Spaces"), "multiple---spaces");
    assert_eq!(
        into_slug("Special!@#$%Characters"),
        "special-----characters"
    );
    assert_eq!(into_slug(""), "");
    assert_eq!(into_slug("---"), "");
    assert_eq!(into_slug("123"), "123");
    assert_eq!(into_slug("Test-Case"), "test-case");
    assert_eq!(into_slug("  Leading Spaces  "), "leading-spaces");
    assert_eq!(into_slug("CamelCase"), "camelcase");
}
