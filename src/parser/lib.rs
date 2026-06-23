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

    let mut ast = rust_norg::parse_tree(&content)
        .map_err(|e| Error::from_reason(format!("Parse error: {e:?}")))?;
    // Clamp heading levels once, before the TOC and HTML are derived, so the
    // recorded TOC level and the emitted `<hN>` tag never disagree.
    clamp_heading_levels(&mut ast);

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

/// Rejects pathologically deep detached-modifier nesting before parsing.
/// `rust_norg::parse_tree` recurses once per nesting level, so a deeply nested
/// list or heading run would overflow the native stack and abort the host
/// process; nesting past the renderer's depth cap is reported as a parse error
/// instead.
///
/// A pre-scan is unavoidable here: a native stack overflow aborts the process
/// (it is not a Rust panic), so it cannot be caught with `catch_unwind`, nor
/// isolated by parsing on a child thread — the abort takes the whole process
/// down. The depth must be bounded *before* `parse_tree` is called.
///
/// Depth is the actual nesting-*tree* depth, not a single line's marker-run
/// length. A lone `------…` line is one item at a high *level* but only one
/// level *deep*, and `rust_norg` parses it without recursing, so it must not be
/// rejected.
///
/// Every construct that `rust_norg` recurses through must be counted: headings,
/// lists, ranged rangeable modifiers, and stacked carryover tags. Their depths
/// are summed because the parser's recursive passes can compose, and headings
/// and lists use separate stacks so same-numbered levels do not pop each other.
///
/// Carryover tags stack onto the single object that follows them, so the run
/// counter is reset once a non-carryover object is consumed (otherwise a
/// document with many independent `#tag\n* H` pairs would falsely accumulate).
/// Ranged modifiers track open/close like lists; an unclosed run of `$$ x`
/// openers can over-count (the parser would actually reject them and not
/// recurse), but that only errs toward a clean rejection, never a missed
/// overflow.
///
/// Blank lines and other content lines both leave the run intact: a text line
/// directly after an item is a paragraph *continuation* (`- a\nx\n-- b` still
/// nests `b` under `a`), so resetting the count on it would let marker lines
/// interleaved with text defeat the guard. Stale open levels can only
/// over-count (erring toward rejection), never hide real nesting.
///
/// Raw content inside a *closed* verbatim block (`@tag` … `@end`) is skipped,
/// since a marker run there is literal text; `rust_norg` parses such a block as
/// a sibling that breaks the surrounding *list* (but not heading) context, so
/// only the list stack is cleared. An *unclosed* `@tag` is NOT a verbatim block
/// in `rust_norg` — it parses as a paragraph and the lines after it are ordinary
/// content — so it is not skipped. Skipping an unclosed `@tag` would hide the
/// real nesting that follows it and reintroduce the overflow.
fn excessive_nesting(content: &str) -> Option<usize> {
    let lines: Vec<&str> = content.lines().map(str::trim_start).collect();
    // Ascending line indices of verbatim terminators. `rust_norg` only accepts
    // a bare `@end` (optionally indented, never with trailing characters) as a
    // closer, which is exactly `trim_start() == "@end"`.
    let end_lines: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|&(_, line)| *line == "@end")
        .map(|(i, _)| i)
        .collect();

    let mut heading_levels: Vec<usize> = Vec::new();
    let mut list_levels: Vec<usize> = Vec::new();
    // Open ranged rangeable modifiers (`$$`/`^^`/`::`), and the length of the
    // run of carryover tags (`#`/`+`) currently stacked on the next object.
    let mut ranged_open: usize = 0;
    let mut carryover_run: usize = 0;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        // A verbatim opener with a matching `@end` skips its raw body. Without
        // a closer it is just a paragraph and falls through to normal handling.
        // The block breaks the surrounding list (a list after it is a sibling),
        // but not the heading whose content it belongs to.
        if is_verbatim_opener(line)
            && let Some(end) = next_after(&end_lines, i)
        {
            list_levels.clear();
            carryover_run = 0;
            i = end + 1;
            continue;
        }

        // Blank lines do not break list nesting in `rust_norg`.
        if line.is_empty() {
            i += 1;
            continue;
        }

        // Classify the line and update the open-nesting counts. A carryover tag
        // stacks onto the object that follows it (the run grows until a real
        // object consumes it); every other line *is* that object, so it counts
        // the stacked carryovers and then clears the run. A bare continuation
        // line changes no stack but still consumes the run.
        let carryover = is_carryover(line);
        if carryover {
            carryover_run += 1;
        } else if let Some(opens) = ranged_modifier(line) {
            // A ranged rangeable modifier opens/closes a recursing body.
            if opens {
                ranged_open += 1;
            } else {
                ranged_open = ranged_open.saturating_sub(1);
            }
        } else if let Some((marker, level)) = nesting_level(line) {
            if marker == '*' {
                // A heading closes any heading at or above its level, nests
                // under what remains, and ends the previous heading's list.
                while heading_levels.last().is_some_and(|&top| top >= level) {
                    heading_levels.pop();
                }
                heading_levels.push(level);
                list_levels.clear();
            } else {
                // Close any list siblings/ancestors at or above this level,
                // then nest under what remains.
                while list_levels.last().is_some_and(|&top| top >= level) {
                    list_levels.pop();
                }
                list_levels.push(level);
            }
        }

        let depth = heading_levels.len() + list_levels.len() + ranged_open + carryover_run;
        if depth > ast_handlers::MAX_LIST_DEPTH {
            return Some(depth);
        }
        if !carryover {
            carryover_run = 0;
        }
        i += 1;
    }
    None
}

/// The smallest index in the ascending `ends` strictly greater than `i`.
fn next_after(ends: &[usize], i: usize) -> Option<usize> {
    let pos = ends.partition_point(|&e| e <= i);
    ends.get(pos).copied()
}

/// Whether `line` can start a ranged verbatim tag (`@name ...`). A bare `@`
/// or `@ name` is ordinary content in `rust_norg`, so treating it as a raw block
/// would let real nesting after it bypass the pre-parse guard.
fn is_verbatim_opener(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('@') else {
        return false;
    };
    line != "@end" && rest.chars().next().is_some_and(|c| !c.is_whitespace())
}

/// The marker and nesting level of a detached-modifier line: the marker char
/// (`*` heading, `-`/`~`/`>` list) and the length of its leading run, when
/// followed by whitespace. `None` for any other line, including non-nesting
/// markers and a marker not followed by whitespace (e.g. `**bold**`). `line`
/// must already be `trim_start`ed. The marker char lets the caller separate
/// heading from list nesting, which recurse on independent stacks.
fn nesting_level(line: &str) -> Option<(char, usize)> {
    let first = line.chars().next()?;
    if !matches!(first, '-' | '~' | '>' | '*') {
        return None;
    }
    let run = line.chars().take_while(|&c| c == first).count();
    // The markers are ASCII, so `run` chars == `run` bytes: a char boundary.
    line[run..]
        .starts_with(|c: char| c.is_whitespace())
        .then_some((first, run))
}

/// Whether `line` is a carryover tag (`#name` / `+name`). The lexer requires the
/// name to follow the marker with no intervening space, so `# foo` is not one.
/// Each carryover tag recurses once in `rust_norg`, so a stacked run of them
/// must count toward the depth bound. `line` must already be `trim_start`ed.
fn is_carryover(line: &str) -> bool {
    let mut chars = line.chars();
    matches!(chars.next(), Some('#' | '+')) && chars.next().is_some_and(|c| !c.is_whitespace())
}

/// Classifies a *ranged* rangeable-modifier line by its double marker
/// (`$$`/`^^`/`::`): `Some(true)` opens one (`$$ title`), `Some(false)` closes
/// one (a bare `$$`). A single marker (`$ term`) is non-ranged and does not
/// recurse, so it returns `None`; three or more markers are plain text, not a
/// modifier. The body of a ranged modifier is parsed by recursive `stage_3`, so
/// nested ranged modifiers must be counted like list levels. `line` must
/// already be `trim_start`ed.
fn ranged_modifier(line: &str) -> Option<bool> {
    let first = line.chars().next()?;
    if !matches!(first, '$' | '^' | ':') {
        return None;
    }
    // The lexer accepts at most two marker chars; the opener is two markers, a
    // space, then a title, and the closer is exactly two markers alone.
    if line.chars().take_while(|&c| c == first).count() != 2 {
        return None;
    }
    let rest = &line[2..]; // markers are ASCII, so byte 2 is a char boundary
    if rest.is_empty() {
        return Some(false); // bare `$$` is the closer
    }
    (rest.starts_with(|c: char| c.is_whitespace()) && !rest.trim().is_empty()).then_some(true)
}

/// Clamps every heading's level to the `<h1>`–`<h6>` range. HTML has no `<h7>`,
/// and `rust_norg` parses 7+ `*` as level 7+. Doing this once on the AST (rather
/// than only at the render site) keeps `extract_toc`'s recorded level in sync
/// with the `<hN>` tag the renderer emits. Headings occur at the top level, in
/// another heading's content, and wrapped in a carryover tag — all three are
/// walked here.
fn clamp_heading_levels(nodes: &mut [rust_norg::NorgAST]) {
    use rust_norg::NorgAST;
    for node in nodes {
        match node {
            NorgAST::Heading { level, content, .. } => {
                *level = (*level).min(6);
                clamp_heading_levels(content);
            }
            NorgAST::CarryoverTag { next_object, .. } => {
                clamp_heading_levels(std::slice::from_mut(next_object.as_mut()));
            }
            _ => {}
        }
    }
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
    fn unclosed_verbatim_tag_does_not_hide_deep_nesting() {
        // A stray `@foo` is NOT a verbatim block in rust_norg (it parses as a
        // paragraph), so the deep list after it is real nesting that would
        // overflow parse_tree. The guard must look past the `@foo` and catch
        // it, not skip the rest of the document waiting for an `@end`.
        let deep: String = (1..=200)
            .map(|level| format!("{} item\n", "-".repeat(level)))
            .collect();
        assert!(
            excessive_nesting(&format!("@foo\n{deep}")).is_some(),
            "deep nesting after an unclosed @tag slipped past the guard"
        );
        // Same for an unclosed `@code` (no terminating @end).
        assert!(
            excessive_nesting(&format!("@code\n{deep}")).is_some(),
            "deep nesting after an unclosed @code slipped past the guard"
        );
    }

    #[test]
    fn malformed_verbatim_opener_does_not_hide_deep_nesting() {
        // `@` and `@ code` are paragraphs, not verbatim openers, even if a
        // later `@end` appears. The scanner must not skip over real nesting.
        let deep: String = (1..=200)
            .map(|level| format!("{} item\n", "-".repeat(level)))
            .collect();

        for opener in ["@", "@ code"] {
            assert!(
                excessive_nesting(&format!("{opener}\n{deep}@end\n")).is_some(),
                "deep nesting after malformed opener {opener:?} slipped past the guard"
            );
        }
    }

    #[test]
    fn continuation_lines_between_items_do_not_reset_the_depth_count() {
        // A text line directly after an item is a paragraph continuation, not
        // a break: `- a\nx\n-- b` still nests `b` under `a`. A deepening chain
        // interleaved with text overflows parse_tree exactly like a contiguous
        // one, so it must not slip past the guard.
        let content: String = (1..=200)
            .map(|level| format!("{} item\nx\n", "-".repeat(level)))
            .collect();
        assert!(
            excessive_nesting(&content).is_some(),
            "interleaved continuation lines defeated the nesting guard"
        );

        // Same for carryover-tag lines between items.
        let tagged: String = (1..=200)
            .map(|level| format!("#tag\n{} item\n", "-".repeat(level)))
            .collect();
        assert!(
            excessive_nesting(&tagged).is_some(),
            "interleaved carryover tags defeated the nesting guard"
        );
    }

    #[test]
    fn blank_separated_deep_nesting_is_rejected() {
        // Blank lines between items do not break nesting in rust_norg (the run
        // stays a single nested list), so they must not reset the depth count.
        let content: String = (1..=200)
            .map(|level| format!("{} item\n\n", "-".repeat(level)))
            .collect();
        assert!(
            excessive_nesting(&content).is_some(),
            "blank-separated deep nesting slipped past the guard"
        );
    }

    #[test]
    fn single_long_marker_run_is_not_rejected() {
        // One line of 200 dashes is a single item at level 200, not 200 levels
        // deep — rust_norg parses it without recursing, so it must not trip the
        // guard. (The old run-length check rejected it.)
        let content = format!("{} item\n", "-".repeat(200));
        assert!(
            excessive_nesting(&content).is_none(),
            "a lone deep-level item was wrongly rejected as deep nesting"
        );
        // And it really does parse — proving the rejection would have been bogus.
        assert!(parse_norg(content, None).is_ok());
    }

    #[test]
    fn non_nesting_marker_runs_are_not_rejected() {
        // `$`, `^`, `:` are rangeable/other modifiers that do not recurse per
        // repeated char; long runs of them must not be mistaken for nesting.
        for marker in ['$', '^', ':'] {
            let content = format!("{} text\n", marker.to_string().repeat(200));
            assert!(
                excessive_nesting(&content).is_none(),
                "a run of '{marker}' was wrongly rejected as deep nesting"
            );
        }
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
    }

    #[test]
    fn stacked_heading_and_list_nesting_is_summed() {
        // Headings and lists recurse independently and stack: a list nested
        // under a deep heading adds its depth on top of the heading's. Neither
        // chain alone exceeds the cap, but together they do. A single shared
        // level stack would let the list's level-1 item pop every open heading
        // level and under-count, hiding the overflow this guard prevents.
        let headings: String = (1..=60).map(|l| format!("{} h\n", "*".repeat(l))).collect();
        let lists: String = (1..=60)
            .map(|l| format!("{} item\n", "-".repeat(l)))
            .collect();
        assert!(
            excessive_nesting(&format!("{headings}{lists}")).is_some(),
            "stacked heading+list nesting (120 deep) was not summed past the cap"
        );
        // Each chain on its own stays under the cap and is accepted.
        assert!(
            excessive_nesting(&headings).is_none(),
            "60 nested headings alone were wrongly rejected"
        );
        assert!(
            excessive_nesting(&lists).is_none(),
            "60 nested list levels alone were wrongly rejected"
        );
    }

    #[test]
    fn carryover_tag_chain_is_rejected_before_parsing() {
        // A long run of stacked carryover tags recurses once per tag in
        // rust_norg's stage_3/convert. The guard counts `#`/`+` runs so the
        // chain can't slip past and overflow the native stack at parse_tree.
        let content = "#a\n".repeat(200);
        assert!(
            excessive_nesting(&content).is_some(),
            "a deep carryover-tag chain slipped past the nesting guard"
        );
        // `+` (attribute) tags recurse the same way.
        assert!(excessive_nesting(&"+a\n".repeat(200)).is_some());
    }

    #[test]
    fn nested_ranged_modifiers_are_rejected() {
        // A ranged rangeable modifier (`$$ …`) parses its body via recursive
        // stage_3, so deeply nested ones overflow parse_tree. The matching
        // closers nest them rather than making them siblings.
        let opens = "$$ x\n".repeat(150);
        let closes = "$$\n".repeat(150);
        assert!(
            excessive_nesting(&format!("{opens}{closes}")).is_some(),
            "deeply nested ranged modifiers slipped past the nesting guard"
        );
    }

    #[test]
    fn single_rangeable_modifiers_are_not_rejected() {
        // `$ term` is a *single*-marker, non-ranged definition that does not
        // recurse; a long list of them must not be mistaken for nesting.
        assert!(
            excessive_nesting(&"$ term\n".repeat(200)).is_none(),
            "non-ranged single-marker definitions were wrongly rejected"
        );
    }

    #[test]
    fn scattered_carryover_tags_are_not_rejected() {
        // Many independent `#tag\n* H` pairs are shallow (each tag wraps one
        // heading), not a deep chain — the run counter must reset per object.
        assert!(
            excessive_nesting(&"#tag\n* H\n".repeat(60)).is_none(),
            "scattered (non-stacked) carryover tags were wrongly rejected"
        );
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
