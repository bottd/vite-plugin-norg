mod ast_handlers;
mod diagnostics;
mod html;
mod metadata;
mod segments;
mod toc;
mod types;
mod utils;

pub use html::transform;
pub use metadata::extract_metadata;
pub use toc::extract_toc;
pub use types::{EmbedComponent, OutputMode, TocEntry};
pub use utils::into_slug;

use arborium::theme::builtin;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::{Map, Value};

#[napi(object)]
pub struct NorgParseResult {
    pub metadata: Map<String, Value>,
    pub html_parts: Vec<String>,
    pub toc: Vec<TocEntry>,
    pub embed_components: Vec<EmbedComponent>,
    pub embed_css: String,
    /// Non-fatal warnings from rendering (skipped/altered content), for the
    /// host to surface — stderr is invisible in a Vite worker.
    pub diagnostics: Vec<String>,
}

#[napi]
pub fn parse_norg(content: String, mode: Option<String>) -> Result<NorgParseResult> {
    // `rust_norg::parse_tree` (and the metadata parser it feeds) recurse once
    // per nesting level on the native stack, so a pathologically deep document
    // overflows the default stack and *aborts the whole process* — an
    // uncatchable crash, not a Rust panic. Running the parse on a thread with a
    // large stack raises the ceiling far past anything written by hand and
    // covers every recursion path at once (lists/headings, ranged and carryover
    // tags, nested inline modifiers, deeply nested metadata) without
    // re-implementing the grammar to predict its depth.
    //
    // ponytail: 512 MiB stack (virtual — pages commit lazily), not a hard depth
    // cap. A maliciously machine-generated file with hundreds of thousands of
    // nesting levels could still overflow it. If untrusted `.norg` input ever
    // becomes a use case, add an explicit depth limit here — rust_norg does not
    // export its token stream, so that limit can't simply reuse its lexer.
    let handle = std::thread::Builder::new()
        .name("norg-parse".into())
        .stack_size(512 * 1024 * 1024)
        .spawn(move || parse_norg_inner(&content, mode.as_deref()))
        .map_err(|e| Error::from_reason(format!("Failed to spawn parser thread: {e}")))?;

    match handle.join() {
        Ok(result) => result.map_err(|reason| Error::from_reason(reason)),
        Err(_) => Err(Error::from_reason("Parser thread panicked")),
    }
}

fn parse_norg_inner(
    content: &str,
    mode: Option<&str>,
) -> std::result::Result<NorgParseResult, String> {
    let ast = rust_norg::parse_tree(content).map_err(|e| format!("Parse error: {e:?}"))?;

    let output_mode = mode.and_then(|s| s.parse().ok());
    let (html_parts, embed_components, embed_css) =
        transform(&ast, output_mode).map_err(|err| format_embed_error(&err))?;
    let metadata = extract_metadata(&ast);
    let toc = extract_toc(&ast);

    Ok(NorgParseResult {
        metadata,
        html_parts,
        toc,
        embed_components,
        embed_css,
        // Drained last so it captures every warning emitted above.
        diagnostics: diagnostics::take(),
    })
}

fn format_embed_error(err: &crate::ast_handlers::EmbedParseError) -> String {
    match err.offending_line() {
        Some(line) => format!("{err}. Offending line: {line}"),
        None => err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_error_after_css_reports_correct_number_and_line() {
        // A CSS embed precedes the failing one. The error ordinal and offending
        // line must point at the failing `@embed bogus` (the 2nd declaration),
        // not get shifted by the CSS embed that emits no component.
        let content = "@embed css\n.foo { color: red; }\n@end\n\n@embed bogus\ncontent\n@end\n";
        let ast = rust_norg::parse_tree(content).unwrap();
        let err = transform(&ast, Some(OutputMode::html)).unwrap_err();
        let msg = format_embed_error(&err);

        assert!(msg.contains("embed #2"), "wrong ordinal in: {msg}");
        assert!(
            msg.contains("Offending line: @embed bogus"),
            "wrong offending line in: {msg}"
        );
    }

    #[test]
    fn embed_error_ignores_embed_lines_inside_other_verbatim_blocks() {
        // The `@embed html` line is raw @code content, not a declaration. The
        // error must point at `@embed bogus` — the only real embed — instead
        // of matching the lookalike line inside the code block.
        let content = "@code norg\n@embed html\n@end\n\n@embed bogus\ncontent\n@end\n";
        let ast = rust_norg::parse_tree(content).unwrap();
        let err = transform(&ast, Some(OutputMode::html)).unwrap_err();
        let msg = format_embed_error(&err);

        assert!(msg.contains("embed #1"), "wrong ordinal in: {msg}");
        assert!(
            msg.contains("Offending line: @embed bogus"),
            "wrong offending line in: {msg}"
        );
    }

    #[test]
    fn deep_nesting_parses_without_aborting() {
        // rust_norg::parse_tree recurses once per nesting level; parse_norg runs
        // it on a large-stack thread so a deeply nested document renders instead
        // of overflowing the native stack and aborting the host process.
        let content: String = (1..=200)
            .map(|level| format!("{} item\n", "-".repeat(level)))
            .collect();
        let result = parse_norg(content, None).expect("deep nesting should parse");
        assert!(!result.html_parts.is_empty());
    }

    #[test]
    fn heading_deeper_than_six_clamps_to_h6() {
        // HTML has no <h7>; rust_norg parses 7+ `*` as level 7+, so the
        // renderer must clamp the tag to <h6> to stay valid markup.
        let result = parse_norg("******* Deep heading\n".to_string(), None).unwrap();
        let html = result.html_parts.join("");
        assert!(
            html.contains("<h6 ") && html.contains("</h6>"),
            "level-7 heading did not clamp to <h6>: {html}"
        );
        assert!(!html.contains("<h7"), "emitted an invalid <h7> tag: {html}");
        // The TOC level must agree with the emitted tag.
        assert_eq!(result.toc[0].level, 6, "TOC level did not clamp to 6");
    }

    #[test]
    fn symbol_only_heading_omits_empty_id() {
        // A title with no alphanumerics slugs to "" — the heading must omit the
        // `id` attribute rather than emit an HTML5-invalid `id=""`.
        let result = parse_norg("* @@@\n".to_string(), None).unwrap();
        let html = result.html_parts.join("");
        assert!(html.contains("<h1"), "no heading emitted: {html}");
        assert!(
            !html.contains("id=\"\""),
            "emitted invalid empty id: {html}"
        );
    }

    #[test]
    fn duplicate_heading_titles_get_distinct_ids() {
        // Two headings with the same title must not share an id (invalid HTML);
        // the second is suffixed, and the TOC must agree with the emitted tags.
        let result = parse_norg("* Setup\nOne.\n* Setup\nTwo.\n".to_string(), None).unwrap();
        let html = result.html_parts.join("");
        assert!(html.contains("id=\"setup\""), "{html}");
        assert!(html.contains("id=\"setup-1\""), "{html}");
        let ids: Vec<&str> = result.toc.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["setup", "setup-1"], "toc ids diverged from tags");
    }

    #[test]
    fn carryover_tagged_heading_appears_in_toc() {
        // The renderer unwraps carryover tags and emits the heading; the TOC
        // must list it too or anchors point at entries the TOC doesn't have.
        let content = "#tag\n* Tagged Heading\nBody.\n";
        let ast = rust_norg::parse_tree(content).unwrap();
        let toc = extract_toc(&ast);
        assert_eq!(toc.len(), 1, "TOC missing the tagged heading: {toc:?}");
        assert_eq!(toc[0].title, "Tagged Heading");
    }
}

#[napi]
pub fn get_theme_css(theme: String) -> String {
    builtin::all()
        .into_iter()
        .find(|t| t.name == theme)
        .map(|t| t.to_css("pre.arborium"))
        .unwrap_or_default()
}
