mod ast_handlers;
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
}

#[napi]
pub fn parse_norg(content: String, mode: Option<String>) -> Result<NorgParseResult> {
    if let Some(depth) = excessive_nesting(&content) {
        return Err(Error::from_reason(format!(
            "Parse error: nesting depth {depth} exceeds the supported maximum of {}",
            ast_handlers::MAX_LIST_DEPTH
        )));
    }

    let ast = rust_norg::parse_tree(&content)
        .map_err(|e| Error::from_reason(format!("Parse error: {e:?}")))?;

    let output_mode = mode.as_deref().and_then(|s| s.parse().ok());
    let (html_parts, embed_components, embed_css) =
        transform(&ast, output_mode).map_err(|err| Error::from_reason(format_embed_error(&err)))?;

    Ok(NorgParseResult {
        metadata: extract_metadata(&ast),
        html_parts,
        toc: extract_toc(&ast),
        embed_components,
        embed_css,
    })
}

/// Rejects pathologically deep nesting before parsing: `rust_norg::parse_tree`
/// recurses once per nesting level, so a file like 1000×`-` would overflow the
/// native stack and abort the host process. A detached-modifier marker run
/// past the renderer's depth cap is reported as a parse error instead.
///
/// Lines inside verbatim blocks (`@code` … `@end`) are raw content, not
/// nesting, and are skipped. A lookalike `@end` inside verbatim content ends
/// the skip early; that errs toward scanning too much, never too little.
fn excessive_nesting(content: &str) -> Option<usize> {
    let mut in_verbatim = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if in_verbatim {
            in_verbatim = trimmed != "@end";
            continue;
        }
        if trimmed.starts_with('@') {
            in_verbatim = trimmed != "@end";
            continue;
        }
        let Some(first) = trimmed.chars().next() else {
            continue;
        };
        if !matches!(first, '-' | '~' | '>' | '*' | '$' | '^' | ':') {
            continue;
        }
        let run = trimmed.chars().take_while(|&c| c == first).count();
        if run > ast_handlers::MAX_LIST_DEPTH
            && trimmed[run..].starts_with(|c: char| c.is_whitespace())
        {
            return Some(run);
        }
    }
    None
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
    fn deeply_nested_list_is_rejected_before_parsing() {
        // rust_norg::parse_tree recurses per nesting level; without the
        // pre-parse guard this input overflows the native stack and aborts
        // the host process instead of returning an error.
        let content: String = (1..=200)
            .map(|level| format!("{} item\n", "-".repeat(level)))
            .collect();
        let err = match parse_norg(content, None) {
            Ok(_) => panic!("deep nesting was not rejected"),
            Err(err) => err,
        };
        assert!(
            err.reason.contains("nesting depth"),
            "wrong error: {}",
            err.reason
        );
    }

    #[test]
    fn marker_runs_inside_verbatim_blocks_do_not_trip_the_nesting_guard() {
        // A long dash run is raw @code content here, not list nesting; the
        // pre-parse guard must not reject the document over it.
        let content = format!(
            "@code text\n{} not a list\n@end\n\n- real item\n",
            "-".repeat(150)
        );
        let result = parse_norg(content, None);
        assert!(result.is_ok(), "verbatim content tripped the nesting guard");
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
}

#[napi]
pub fn get_theme_css(theme: String) -> String {
    builtin::all()
        .into_iter()
        .find(|t| t.name == theme)
        .map(|t| t.to_css("pre.arborium"))
        .unwrap_or_default()
}
