use insta::assert_yaml_snapshot;
use std::fs;
use vite_plugin_norg_parser::{
    NorgParseResult, extract_metadata, extract_toc, parse_norg, transform,
};

fn parse(content: &str) -> NorgParseResult {
    parse_norg(content.to_string(), None).expect("failed to parse norg")
}

#[test]
fn test_norg_fixture_files() {
    for fixture_path in [
        "tests/fixtures/basic.norg",
        "tests/fixtures/code-blocks.norg",
        "tests/fixtures/headings.norg",
        "tests/fixtures/images.norg",
        "tests/fixtures/links.norg",
        "tests/fixtures/embed-css.norg",
        "tests/fixtures/nested-lists.norg",
        "tests/fixtures/blocks.norg",
    ] {
        let content = fs::read_to_string(fixture_path)
            .unwrap_or_else(|_| panic!("Failed to read {fixture_path}"));
        let ast = rust_norg::parse_tree(&content)
            .unwrap_or_else(|_| panic!("Failed to parse {fixture_path}"));

        let (html_parts, _embed_components, embed_css) =
            transform(&ast, None).unwrap_or_else(|_| panic!("Failed to transform {fixture_path}"));
        let html = html_parts.join("");
        let toc = extract_toc(&ast);

        let metadata = extract_metadata(&ast);
        assert_yaml_snapshot!(fixture_path, (html, toc, metadata, embed_css));
    }
}

#[test]
fn test_embed_css_no_components() {
    let content = r#"
@embed css
.test { color: red; }
@end
"#;
    let ast = rust_norg::parse_tree(content).unwrap();
    let (html_parts, embed_components, embed_css) = transform(&ast, None).unwrap();

    assert!(
        embed_components.is_empty(),
        "CSS blocks should not create embed components"
    );
    assert!(
        embed_css.contains(".test { color: red; }"),
        "CSS content should be collected in embed_css"
    );
    // With no embeds, html_parts should have exactly 1 part
    assert_eq!(html_parts.len(), 1);
}

#[test]
fn embed_errors_report_the_ast_declaration() {
    for (content, ordinal) in [
        (
            "@embed css\n.foo {}\n@end\n\n@embed bogus\ncontent\n@end\n",
            "embed #2",
        ),
        (
            "@code norg\n@embed html\n@end\n\n@embed bogus\ncontent\n@end\n",
            "embed #1",
        ),
    ] {
        let error = match parse_norg(content.to_string(), Some("html".to_string())) {
            Ok(_) => panic!("expected embed error"),
            Err(error) => error,
        };
        let message = error.to_string();
        assert!(message.contains(ordinal), "{message}");
        assert!(
            message.contains("Offending line: @embed bogus"),
            "{message}"
        );
    }
}

#[test]
fn deep_nesting_parses_on_the_bounded_stack() {
    let content: String = (1..=200)
        .map(|level| format!("{} item\n", "-".repeat(level)))
        .collect();
    assert!(!parse(&content).html_parts.is_empty());
}

#[test]
fn heading_levels_and_empty_ids_stay_valid() {
    let result = parse("******* Deep heading\n* @@@\n");
    let html = result.html_parts.concat();
    assert!(html.contains("<h6 id=\"deep-heading\">Deep heading</h6>"));
    assert!(!html.contains("<h7"));
    assert!(!html.contains("id=\"\""));
    assert_eq!(result.toc.len(), 1);
    assert_eq!(result.toc[0].level, 6);
}

#[test]
fn heading_ids_reserve_generated_slugs() {
    for (content, expected) in [
        (
            "* Setup\n* Setup\n* Setup 1\n",
            ["setup", "setup-2", "setup-1"],
        ),
        (
            "* Setup\n* Setup 1\n* Setup\n",
            ["setup", "setup-1", "setup-2"],
        ),
    ] {
        let result = parse(content);
        let html = result.html_parts.concat();
        let ids: Vec<_> = result.toc.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(ids, expected);
        for id in expected {
            assert!(html.contains(&format!("id=\"{id}\"")), "{html}");
        }
    }
}

#[test]
fn heading_links_resolve_when_generated_ids_collide() {
    let result = parse("* Setup\n* Setup\n* Setup 1\n{* Setup 1}\n");
    let html = result.html_parts.concat();
    assert!(html.contains(r#"<h1 id="setup-1">Setup 1</h1>"#), "{html}");
    assert!(
        html.contains(r##"<a href="#setup-1">Setup 1</a>"##),
        "{html}"
    );
}

#[test]
fn heading_links_use_the_target_level_for_duplicate_titles() {
    let result = parse("* Same\n** Same\n{** Same}\n");
    let html = result.html_parts.concat();
    assert!(html.contains(r#"<h1 id="same">Same</h1>"#), "{html}");
    assert!(html.contains(r#"<h2 id="same-1">Same</h2>"#), "{html}");
    assert!(html.contains(r##"<a href="#same-1">Same</a>"##), "{html}");
}

#[test]
fn heading_links_distinguish_titles_with_the_same_slug() {
    let result = parse("* A+B\n* A B\n{* A B}\n");
    let html = result.html_parts.concat();
    assert!(html.contains(r#"<h1 id="a-b">A+B</h1>"#), "{html}");
    assert!(html.contains(r#"<h1 id="a-b-1">A B</h1>"#), "{html}");
    assert!(html.contains(r##"<a href="#a-b-1">A B</a>"##), "{html}");
}

#[test]
fn heading_locator_keys_follow_norg_normalization() {
    let whitespace = parse("* AB\n* A B\n{* A B}\n").html_parts.concat();
    assert!(
        whitespace.contains(r##"<a href="#ab">A B</a>"##),
        "{whitespace}"
    );

    let markup = parse("* *Foo*\n* Foo\n{* Foo}\n").html_parts.concat();
    assert!(markup.contains(r#"<h1 id="foo-1">Foo</h1>"#), "{markup}");
    assert!(markup.contains(r##"<a href="#foo-1">Foo</a>"##), "{markup}");
}

#[test]
fn footnotes_share_the_document_id_namespace() {
    let result = parse("* footnote note\n^ note\nbody\n{^ note}\n");
    let html = result.html_parts.concat();
    assert!(
        html.contains(r#"<h1 id="footnote-note">footnote note</h1>"#),
        "{html}"
    );
    assert!(html.contains(r#"<aside id="footnote-note-1""#), "{html}");
    assert!(
        html.contains(r##"<a href="#footnote-note-1">note</a>"##),
        "{html}"
    );
}

#[test]
fn filepath_only_heading_gets_an_anchor() {
    let result = parse("* {:docs/readme.norg:}\n");
    let html = result.html_parts.concat();
    assert!(html.contains("id=\"docs-readme-norg\""), "{html}");
    assert_eq!(result.toc[0].id, "docs-readme-norg");
}

#[test]
fn comments_are_suppressed_without_diagnostics() {
    let result = parse("#comment\n* Hidden\nsecret\n* Visible\nshown\n");
    let html = result.html_parts.concat();
    assert!(!html.contains("Hidden"), "{html}");
    assert!(!html.contains("secret"), "{html}");
    assert!(html.contains("Visible"), "{html}");
    assert_eq!(result.toc.len(), 1);
    assert_eq!(result.toc[0].title, "Visible");
    assert!(
        result.diagnostics.as_deref().unwrap_or_default().is_empty(),
        "{:?}",
        result.diagnostics
    );

    let ranged = parse("|comment\nhidden\n|end\nvisible\n");
    let html = ranged.html_parts.concat();
    assert!(!html.contains("hidden"), "{html}");
    assert!(html.contains("visible"), "{html}");
    assert!(ranged.diagnostics.unwrap_or_default().is_empty());

    let verbatim = parse("@comment\nhidden\n@end\nvisible\n");
    let html = verbatim.html_parts.concat();
    assert!(!html.contains("hidden"), "{html}");
    assert!(html.contains("visible"), "{html}");
    assert!(verbatim.diagnostics.unwrap_or_default().is_empty());
}

#[test]
fn commented_lists_and_embeds_are_not_processed() {
    let lists = parse("- retained\n#comment\n- hidden one\n- hidden two\n---\n- visible\n");
    let html = lists.html_parts.concat();
    assert!(!html.contains("hidden"), "{html}");
    assert!(html.contains("retained"), "{html}");
    assert!(html.contains("visible"), "{html}");

    let embed = parse("#comment\n@embed bogus\nsecret\n@end\n---\nvisible\n");
    assert!(embed.html_parts.concat().contains("visible"));
    assert!(embed.embed_components.is_empty());
}

#[test]
fn strong_list_comments_end_at_a_same_level_marker_change() {
    let result = parse("#comment\n- hidden\n~ visible\n");
    let html = result.html_parts.concat();
    assert!(!html.contains("hidden"), "{html}");
    assert!(html.contains("visible"), "{html}");
}

#[test]
fn strong_list_comments_suppress_nested_same_marker_siblings() {
    let result = parse("- parent\n#comment\n-- hidden\n-- also hidden\n- visible\n");
    let html = result.html_parts.concat();
    assert!(html.contains("parent"), "{html}");
    assert!(!html.contains("hidden"), "{html}");
    assert!(html.contains("visible"), "{html}");
}

#[test]
fn consecutive_strong_list_comments_reset_the_suppression_marker() {
    let result = parse("#comment\n- hidden bullet\n#comment\n~ hidden ordered\n~ also hidden\n");
    let html = result.html_parts.concat();
    assert!(!html.contains("hidden"), "{html}");
}

#[test]
fn weak_heading_comments_keep_nested_headings() {
    let result = parse(
        "+comment\n* Hidden parent\nparent secret\n** Visible child\nchild shown\n* Visible sibling\nsibling shown\n",
    );
    let html = result.html_parts.concat();
    assert!(!html.contains("Hidden parent"), "{html}");
    assert!(!html.contains("parent secret"), "{html}");
    assert!(html.contains("Visible child"), "{html}");
    assert!(html.contains("child shown"), "{html}");
    assert!(html.contains("Visible sibling"), "{html}");
    let titles: Vec<_> = result
        .toc
        .iter()
        .map(|entry| entry.title.as_str())
        .collect();
    assert_eq!(titles, ["Visible child", "Visible sibling"]);
}

#[test]
fn chained_strong_comments_suppress_the_heading_scope() {
    let result = parse(
        "#comment\n#tag\n* Hidden parent\nparent secret\n** Hidden child\nchild secret\n* Visible\nshown\n",
    );
    let html = result.html_parts.concat();
    assert!(!html.contains("Hidden"), "{html}");
    assert!(!html.contains("secret"), "{html}");
    assert!(html.contains("Visible"), "{html}");
    assert_eq!(result.toc.len(), 1);
    assert_eq!(result.toc[0].title, "Visible");
}

#[test]
fn inner_strong_comments_override_outer_weak_comments() {
    let result = parse(
        "+comment\n#comment\n* Hidden parent\nparent secret\n** Hidden child\nchild secret\n* Visible\nshown\n",
    );
    let html = result.html_parts.concat();
    assert!(!html.contains("Hidden"), "{html}");
    assert!(!html.contains("secret"), "{html}");
    assert!(html.contains("Visible"), "{html}");
}

#[test]
fn a_delimiter_ends_strong_heading_comment_scope() {
    let result = parse("#comment\n* Hidden\nsecret\n---\n** Visible\nshown\n");
    let html = result.html_parts.concat();
    assert!(!html.contains("Hidden"), "{html}");
    assert!(!html.contains("secret"), "{html}");
    assert!(html.contains("Visible"), "{html}");
    assert!(html.contains("shown"), "{html}");
    assert_eq!(result.toc.len(), 1);
    assert_eq!(result.toc[0].title, "Visible");
}

#[test]
fn horizontal_rules_do_not_end_strong_heading_comment_scope() {
    let result = parse("#comment\n#tag\n* Hidden\nbefore\n___\nafter\n* Visible\nshown\n");
    let html = result.html_parts.concat();
    assert!(!html.contains("Hidden"), "{html}");
    assert!(!html.contains("before"), "{html}");
    assert!(!html.contains("after"), "{html}");
    assert!(!html.contains("<hr"), "{html}");
    assert!(html.contains("Visible"), "{html}");
}

#[test]
fn comments_inside_rangeable_content_do_not_warn() {
    let result = parse("$$ Term\n|comment\nhidden\n|end\nvisible\n$$\n");
    let html = result.html_parts.concat();
    assert!(!html.contains("hidden"), "{html}");
    assert!(html.contains("visible"), "{html}");
    assert!(result.diagnostics.unwrap_or_default().is_empty());
}

#[test]
fn strong_heading_comments_suppress_rangeable_body_scope() {
    let result = parse("$$ Term\n#comment\n* Hidden\nsecret\n$$\n");
    let html = result.html_parts.concat();
    assert!(!html.contains("Hidden"), "{html}");
    assert!(!html.contains("secret"), "{html}");
}

#[test]
fn weak_delimiters_only_reduce_comment_scope_one_level() {
    let result = parse(
        "$$ Term\n#comment\n* Hidden parent\n** Hidden child\n---\nstill hidden parent\n$$\n",
    );
    let html = result.html_parts.concat();
    assert!(!html.contains("Hidden"), "{html}");
    assert!(!html.contains("still hidden parent"), "{html}");
}

#[test]
fn anchor_definitions_render_their_link_target() {
    let result = parse("[Go]{https://example.com}\n");
    let html = result.html_parts.concat();
    assert!(
        html.contains(
            r#"<a href="https://example.com" target="_blank" rel="noopener noreferrer">Go</a>"#
        ),
        "{html}"
    );
}

#[test]
fn empty_anchor_definitions_keep_their_visible_text() {
    let result = parse("[label]{}\n");
    let html = result.html_parts.concat();
    assert!(html.contains("label"), "{html}");
    assert_eq!(result.diagnostics.unwrap_or_default().len(), 1);
}

#[test]
fn toc_rendering_does_not_duplicate_heading_diagnostics() {
    let result = parse("* {javascript:alert(1)}[Unsafe]\n");
    let diagnostics = result.diagnostics.unwrap_or_default();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert!(diagnostics[0].contains("unsafe URL scheme"));
}

#[test]
fn separate_unsafe_links_each_emit_a_diagnostic() {
    let result = parse("{javascript:alert(1)}[First]\n\n{javascript:alert(1)}[Second]\n");
    assert_eq!(result.diagnostics.unwrap_or_default().len(), 2);
}

#[test]
fn unsupported_carryovers_render_and_warn() {
    let result = parse("#tag\n* Tagged Heading\nBody.\n");
    assert_eq!(result.toc[0].title, "Tagged Heading");
    let diagnostics = result.diagnostics.unwrap_or_default();
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].contains("carryover tag 'tag'"));
}

#[test]
fn mixed_marker_grandchildren_stay_under_their_parent() {
    let ast = rust_norg::parse_tree("- parent\n-- child\n~~~ grandchild\n-- sibling\n").unwrap();
    let (parts, _, _) = transform(&ast, None).unwrap();
    assert_eq!(
        parts.concat(),
        "<ul><li>parent<ul><li>child<ol><li>grandchild</li></ol></li><li>sibling</li></ul></li></ul>\n"
    );
}

#[test]
fn embed_component_indexes_ignore_css_declarations() {
    let content = "@embed css\n.foo {}\n@end\n@embed svelte\n<div>one</div>\n@end\n@embed svelte\n<div>two</div>\n@end\n";
    let result = parse_norg(content.to_string(), Some("svelte".to_string())).unwrap();
    let indexes: Vec<_> = result
        .embed_components
        .iter()
        .map(|embed| embed.index)
        .collect();
    assert_eq!(indexes, [0, 1]);
}
