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

    let ast = vec![node];
    let (html, toc) = convert_nodes(&ast);
    assert!(html.starts_with("<h1 id=\"\">"));
    assert!(html.ends_with("</h1>"));
    assert_eq!(toc.len(), 1);
}
