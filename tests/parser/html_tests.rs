use vite_plugin_norg_parser::{convert_ast_to_html, convert_ast_to_html_with_toc};

#[test]
fn test_convert_ast_to_html_empty() {
    let ast = vec![];
    let result = convert_ast_to_html(&ast);
    assert_eq!(result, "<p>No renderable content found</p>");
}

#[test]
fn test_convert_single_ast_to_html_code_block() {
    use rust_norg::NorgAST::VerbatimRangedTag;

    let node = VerbatimRangedTag {
        name: vec!["code".to_string()],
        content: "console.log(\"Hello, World!\");".to_string(),
        parameters: Default::default(),
    };

    let ast = vec![node];
    let result = convert_ast_to_html(&ast);
    assert_eq!(
        result,
        "<pre><code>console.log(&quot;Hello, World!&quot;);</code></pre>"
    );
}

#[test]
fn test_convert_single_ast_to_html_code_block_with_language() {
    use rust_norg::NorgAST::VerbatimRangedTag;

    let node = VerbatimRangedTag {
        name: vec!["code".to_string()],
        parameters: vec!["javascript".to_string()],
        content: "const message = \"Hello, World!\";".to_string(),
    };

    let ast = vec![node];
    let result = convert_ast_to_html(&ast);
    assert_eq!(result, "<pre class=\"language-javascript\"><code class=\"language-javascript\">const message = &quot;Hello, World!&quot;;</code></pre>");
}

#[test]
fn test_convert_single_ast_to_html_document_meta_ignored() {
    use rust_norg::NorgAST::VerbatimRangedTag;

    let node = VerbatimRangedTag {
        name: vec!["document".to_string(), "meta".to_string()],
        content: "title: Test Document".to_string(),
        parameters: Default::default(),
    };

    let ast = vec![node];
    let result = convert_ast_to_html(&ast);
    assert_eq!(result, "<p>No renderable content found</p>");
}

#[test]
fn test_convert_single_ast_to_html_verbatim() {
    use rust_norg::NorgAST::VerbatimRangedTag;

    let node = VerbatimRangedTag {
        name: vec!["custom".to_string()],
        content: "Some custom content with <html> & symbols".to_string(),
        parameters: Default::default(),
    };

    let ast = vec![node];
    let result = convert_ast_to_html(&ast);
    assert_eq!(
        result,
        "<div class=\"verbatim\">Some custom content with &lt;html&gt; &amp; symbols</div>"
    );
}

#[test]
fn test_convert_single_ast_to_html_paragraph() {
    use rust_norg::NorgAST::Paragraph;

    let segments = vec![];
    let node = Paragraph(segments);

    let ast = vec![node];
    let result = convert_ast_to_html(&ast);
    assert_eq!(result, "<p>No renderable content found</p>");
}

#[test]
fn test_convert_single_ast_to_html_heading() {
    use rust_norg::NorgAST::Heading;

    let node = Heading {
        level: 1,
        title: vec![],
        content: vec![],
        extensions: Default::default(),
    };

    let ast = vec![node];
    let (result, toc) = convert_ast_to_html_with_toc(&ast);
    assert!(result.starts_with("<h1 id=\"\">"));
    assert!(result.ends_with("</h1>"));
    assert_eq!(toc.len(), 1);
}

#[test]
fn test_convert_ast_to_html_multiple_nodes() {
    use rust_norg::NorgAST::VerbatimRangedTag;

    let ast = vec![
        VerbatimRangedTag {
            name: vec!["code".to_string()],
            parameters: vec![],
            content: "code1".to_string(),
        },
        VerbatimRangedTag {
            name: vec!["code".to_string()],
            parameters: vec!["rust".to_string()],
            content: "code2".to_string(),
        },
    ];

    let result = convert_ast_to_html(&ast);
    assert!(result.contains("<pre><code>code1</code></pre>"));
    assert!(result
        .contains("<pre class=\"language-rust\"><code class=\"language-rust\">code2</code></pre>"));
    assert!(result.contains("\n"));
}

#[test]
fn test_convert_ast_to_html_mixed_with_empty_nodes() {
    use rust_norg::NorgAST::VerbatimRangedTag;

    let ast = vec![
        VerbatimRangedTag {
            name: vec!["document".to_string(), "meta".to_string()],
            content: "title: Test".to_string(),
            parameters: Default::default(),
        },
        VerbatimRangedTag {
            name: vec!["code".to_string()],
            content: "actual code".to_string(),
            parameters: Default::default(),
        },
    ];

    let result = convert_ast_to_html(&ast);
    assert!(!result.contains("title: Test"));
    assert!(result.contains("<pre><code>actual code</code></pre>"));
}

#[test]
fn test_code_blocks_with_multiple_languages() {
    use rust_norg::NorgAST::VerbatimRangedTag;

    let ast = vec![
        VerbatimRangedTag {
            name: vec!["code".to_string()],
            parameters: vec!["javascript".to_string()],
            content: "function hello() { return \"world\"; }".to_string(),
        },
        VerbatimRangedTag {
            name: vec!["code".to_string()],
            parameters: vec!["python".to_string()],
            content: "def add(a, b): return a + b".to_string(),
        },
        VerbatimRangedTag {
            name: vec!["code".to_string()],
            parameters: vec![],
            content: "plain text".to_string(),
        },
    ];

    let result = convert_ast_to_html(&ast);

    assert!(result.contains("<pre class=\"language-javascript\"><code class=\"language-javascript\">function hello() { return &quot;world&quot;; }</code></pre>"));
    assert!(result.contains("<pre class=\"language-python\"><code class=\"language-python\">def add(a, b): return a + b</code></pre>"));
    assert!(result.contains("<pre><code>plain text</code></pre>"));
}
