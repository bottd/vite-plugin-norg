#[cfg(target_arch = "wasm32")]
mod wasm_tests {
    use vite_plugin_norg_parser::parse_norg;

    #[test]
    fn test_parse_norg_empty() {
        let result = parse_norg("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_norg_simple_text() {
        let content = "This is a simple paragraph.";
        let result = parse_norg(content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_norg_with_metadata() {
        let content = r#"@document.meta
title: "Test Document"
author: "John Doe"
@end

* Main Title

This is content."#;

        let result = parse_norg(content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_norg_code_block() {
        let content = r#"@code javascript
console.log("Hello, World!");
@end"#;

        let result = parse_norg(content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_norg_heading() {
        let content = "* Main Heading\n\nSome content under the heading.";
        let result = parse_norg(content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_norg_malformed_input() {
        let content = "@document.meta\ntitle: incomplete";
        let result = parse_norg(content);
        // Should either succeed or fail gracefully
        match result {
            Ok(_) => {}
            Err(_) => {}
        }
    }

    #[test]
    fn test_parse_norg_special_characters() {
        let content =
            "* Title with <special> & characters\n\nParagraph with \"quotes\" and 'apostrophes'.";
        let result = parse_norg(content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_markup_formatting() {
        let content = "This is a *bold* and _italic_ test.";
        let result = parse_norg(content);
        assert!(result.is_ok());
    }
}

// For non-WASM targets, create a simple placeholder test
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn wasm_tests_placeholder() {
    // This test exists so that `cargo test` doesn't fail when not targeting WASM
    // The actual WASM tests will only run when targeting the wasm32 architecture
    assert!(true);
}
