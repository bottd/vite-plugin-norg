use vite_plugin_norg_parser::convert_nodes;

#[test]
fn test_convert_nodes() {
    use rust_norg::NorgAST::Heading;

    let node = Heading {
        level: 1,
        title: vec![],
        content: vec![],
        extensions: Default::default(),
    };

    let mut toc = Vec::new();
    let ast = vec![node];
    let result = convert_nodes(&ast, &mut toc);
    assert!(result.starts_with("<h1 id=\"\">"));
    assert!(result.ends_with("</h1>"));
    assert_eq!(toc.len(), 1);
}
