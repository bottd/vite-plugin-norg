use crate::ast_handlers::*;
use crate::segments::{convert_segments, heading_html_and_id, title_slug};
use crate::types::{EmbedComponent, OutputMode};
use arborium::Highlighter;
use rust_norg::{NorgAST, NorgASTFlat, ParagraphSegment, RangeableDetachedModifier};

struct TransformState {
    parts: Vec<String>,
    current_html: String,
    embed_components: Vec<EmbedComponent>,
    css_blocks: Vec<String>,
    mode: Option<OutputMode>,
    highlighter: Highlighter,
    /// Ordinal of every `@embed` declaration the renderer visits (incl. CSS,
    /// `None`-mode, and failing ones), giving errors their "embed #N" number.
    /// Unlike `embed_components.len()`, it counts embeds that emit no
    /// component.
    embed_decls: usize,
    /// Per-document slug counter so duplicate heading titles get distinct ids
    /// (`setup`, `setup-1`, …); walked in the same order as the TOC pass.
    heading_ids: std::collections::HashMap<String, u32>,
}

impl TransformState {
    fn new(mode: Option<OutputMode>) -> Self {
        Self {
            parts: Vec::new(),
            current_html: String::new(),
            embed_components: Vec::new(),
            css_blocks: Vec::new(),
            mode,
            highlighter: Highlighter::new(),
            embed_decls: 0,
            heading_ids: std::collections::HashMap::new(),
        }
    }

    fn push_html(&mut self, html: &str) {
        self.current_html.push_str(html);
        self.current_html.push('\n');
    }

    /// Renders a flattened list run and appends it, skipping an empty render.
    fn push_list(&mut self, items: &[FlatListItem]) {
        let html = render_list_items(items);
        if !html.is_empty() {
            self.push_html(&html);
        }
    }

    fn apply_verbatim(&mut self, result: VerbatimTagResult) {
        match result {
            VerbatimTagResult::Html(html) => self.push_html(&html),
            VerbatimTagResult::Css(css) => self.css_blocks.push(css),
            VerbatimTagResult::Embed { mode, code } => {
                self.parts.push(std::mem::take(&mut self.current_html));
                self.embed_components.push(EmbedComponent { mode, code });
            }
        }
    }

    fn finalize(mut self) -> (Vec<String>, Vec<EmbedComponent>, String) {
        self.parts.push(self.current_html);
        (
            self.parts,
            self.embed_components,
            self.css_blocks.join("\n"),
        )
    }
}

pub fn transform(
    ast: &[NorgAST],
    mode: Option<OutputMode>,
) -> Result<(Vec<String>, Vec<EmbedComponent>, String), EmbedParseError> {
    let mut state = TransformState::new(mode);
    transform_nodes(ast, &mut state)?;
    Ok(state.finalize())
}

fn transform_nodes(nodes: &[NorgAST], state: &mut TransformState) -> Result<(), EmbedParseError> {
    let mut i = 0;
    while i < nodes.len() {
        // Adjacent list-like nodes form one run so the renderer can re-nest
        // mixed-marker siblings by level (rust-norg emits a deeper list of a
        // different marker type as a sibling `List`, not as item content).
        // List rendering is a pure function over the flattened run — it has
        // no access to the embed/css stream, so it cannot misalign it.
        let start = i;
        let mut items = Vec::new();
        while i < nodes.len() && collect_list_items(&nodes[i], &mut items) {
            i += 1;
        }
        if i > start {
            state.push_list(&items);
            continue;
        }
        transform_node(&nodes[i], state)?;
        i += 1;
    }
    Ok(())
}

fn transform_node(node: &NorgAST, state: &mut TransformState) -> Result<(), EmbedParseError> {
    match node {
        NorgAST::List { .. } | NorgAST::NestableDetachedModifier { .. } => {
            // Defensive: transform_nodes' run loop consumes list-like nodes
            // (including carryover-wrapped ones) before dispatching here, so a
            // list should never reach this arm. Render it the same way rather
            // than silently dropping it if the AST shape ever changes.
            let mut items = Vec::new();
            collect_list_items(node, &mut items);
            state.push_list(&items);
        }
        NorgAST::VerbatimRangedTag {
            name,
            parameters,
            content,
            ..
        } => {
            let tag = VerbatimTag::from(name.as_slice());
            // Capture the ordinal before incrementing; see `embed_decls` doc.
            let embed_index = state.embed_decls;
            if matches!(tag, VerbatimTag::Embed) {
                state.embed_decls += 1;
            }
            if let Some(result) = tag.render(
                parameters,
                content,
                state.mode,
                &mut state.highlighter,
                embed_index,
            )? {
                state.apply_verbatim(result);
            }
        }
        NorgAST::Heading {
            level,
            title,
            content,
            ..
        } => {
            let (title_html, id, tag_level) =
                heading_html_and_id(title, *level, &mut state.heading_ids);
            // A symbol-only title (e.g. `* @@@`) slugs to "" — omit the
            // attribute rather than emit an HTML5-invalid `id=""`.
            let id_attr = if id.is_empty() {
                String::new()
            } else {
                format!(" id=\"{id}\"")
            };
            state.push_html(&format!(
                "<h{tag_level}{id_attr}>{title_html}</h{tag_level}>"
            ));
            transform_nodes(content, state)?;
        }
        NorgAST::Paragraph(segments) => {
            if let Some(html) = paragraph(segments) {
                state.push_html(&html);
            }
        }
        NorgAST::RangeableDetachedModifier {
            modifier_type,
            title,
            content,
            ..
        } => state.push_html(&rangeable_modifier(modifier_type, title, content)),
        NorgAST::DelimitingModifier(delim) => state.push_html(delimiter(delim)),
        NorgAST::CarryoverTag {
            name, next_object, ..
        } => {
            // The annotation is unimplemented, but the object it annotates is
            // real content — warn and render it anyway.
            warn_carryover_ignored(name);
            transform_node(next_object, state)?;
        }
        NorgAST::RangedTag { name, .. } => warn_unimplemented("ranged", name),
        NorgAST::InfirmTag { name, .. } => warn_unimplemented("infirm", name),
    }
    Ok(())
}

fn rangeable_modifier(
    modifier_type: &RangeableDetachedModifier,
    title: &[ParagraphSegment],
    content: &[NorgASTFlat],
) -> String {
    // convert_segments output is final HTML (text already escaped, markup
    // intentional) — re-encoding it would render `&` as `&amp;` and inline
    // markup as literal tags.
    let title_html = convert_segments(title);
    let body: String = content
        .iter()
        .filter_map(|node| match node {
            NorgASTFlat::Paragraph(segments) => paragraph(segments),
            _ => {
                crate::diagnostics::warn(
                    "unsupported block inside a definition/footnote/table body — content skipped",
                );
                None
            }
        })
        .collect();

    match modifier_type {
        RangeableDetachedModifier::Definition => {
            format!("<dl><dt>{title_html}</dt><dd>{body}</dd></dl>")
        }
        RangeableDetachedModifier::Footnote => {
            let id = title_slug(title);
            // `body` is already a sequence of <p> blocks.
            format!(
                "<aside id=\"footnote-{id}\" class=\"footnote\"><strong>{title_html}</strong>{body}</aside>"
            )
        }
        RangeableDetachedModifier::Table => {
            // The Norg table modifier carries free-form body content, not
            // rows; a single cell is the minimal valid placement for it.
            format!(
                "<table><caption>{title_html}</caption><tbody><tr><td>{body}</td></tr></tbody></table>"
            )
        }
    }
}
