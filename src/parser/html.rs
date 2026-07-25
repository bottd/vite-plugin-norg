use crate::ast_handlers::*;
use crate::segments::{DocumentIds, convert_segments_with_ids, heading_html_and_id};
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
    ids: DocumentIds,
}

impl TransformState {
    fn new(mode: Option<OutputMode>, ids: DocumentIds) -> Self {
        Self {
            parts: Vec::new(),
            current_html: String::new(),
            embed_components: Vec::new(),
            css_blocks: Vec::new(),
            mode,
            highlighter: Highlighter::new(),
            embed_decls: 0,
            ids,
        }
    }

    fn push_html(&mut self, html: &str) {
        self.current_html.push_str(html);
        self.current_html.push('\n');
    }

    /// Renders a flattened list run and appends it, skipping an empty render.
    fn push_list(&mut self, events: &[FlatListEvent]) {
        let html = render_list_items(events, &self.ids);
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
                self.embed_components.push(EmbedComponent {
                    index: self.embed_components.len() as u32,
                    mode,
                    code,
                });
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
    let mut state = TransformState::new(mode, document_ids(ast));
    transform_nodes(ast, &mut state)?;
    // Leftovers mean this walk and the pre-pass disagreed about what's visible.
    debug_assert_eq!(
        state.ids.unconsumed(),
        (0, 0),
        "renderer and document_ids disagreed on visible nodes"
    );
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
        let mut events = Vec::new();
        while i < nodes.len() && collect_list_items(&nodes[i], &mut events) {
            i += 1;
        }
        if i > start {
            state.push_list(&events);
            continue;
        }

        if let Some(scope) = comment_scope(nodes, i) {
            for node in scope.visible {
                transform_nodes(std::slice::from_ref(node), state)?;
            }
            i = scope.end;
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
            // `transform_nodes` consumes list runs before dispatching here.
            debug_assert!(false, "list nodes are consumed by transform_nodes");
            let mut events = Vec::new();
            collect_list_items(node, &mut events);
            state.push_list(&events);
        }
        NorgAST::VerbatimRangedTag { name, .. } if is_comment_tag(name) => {}
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
            let (title_html, id, tag_level) = heading_html_and_id(title, *level, &mut state.ids);
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
            if let Some(html) = paragraph(segments, &state.ids) {
                state.push_html(&html);
            }
        }
        NorgAST::RangeableDetachedModifier {
            modifier_type,
            title,
            content,
            ..
        } => {
            let html = rangeable_modifier(modifier_type, title, content, &mut state.ids);
            state.push_html(&html);
        }
        NorgAST::DelimitingModifier(delim) => state.push_html(delimiter(delim)),
        NorgAST::CarryoverTag {
            name, next_object, ..
        } => {
            if comment_target(node).is_none() {
                warn_carryover_ignored(name);
                transform_nodes(std::slice::from_ref(next_object), state)?;
            }
        }
        NorgAST::RangedTag { name, .. } if is_comment_tag(name) => {}
        NorgAST::RangedTag { name, .. } => warn_unimplemented("ranged", name),
        NorgAST::InfirmTag { name, .. } => warn_unimplemented("infirm", name),
    }
    Ok(())
}

fn rangeable_modifier(
    modifier_type: &RangeableDetachedModifier,
    title: &[ParagraphSegment],
    content: &[NorgASTFlat],
    ids: &mut DocumentIds,
) -> String {
    // convert_segments output is final HTML (text already escaped, markup
    // intentional) — re-encoding it would render `&` as `&amp;` and inline
    // markup as literal tags.
    let title_html = convert_segments_with_ids(title, ids);
    let body = rangeable_body(content, ids);

    match modifier_type {
        RangeableDetachedModifier::Definition => {
            format!("<dl><dt>{title_html}</dt><dd>{body}</dd></dl>")
        }
        RangeableDetachedModifier::Footnote => {
            let id = ids.next_footnote();
            // `body` is already a sequence of <p> blocks.
            format!(
                "<aside id=\"{id}\" class=\"footnote\"><strong>{title_html}</strong>{body}</aside>"
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

fn rangeable_body(content: &[NorgASTFlat], ids: &DocumentIds) -> String {
    let mut body = String::new();
    let mut index = 0;
    while index < content.len() {
        if let Some(next) = flat_comment_end(content, index) {
            index = next;
            continue;
        }

        match &content[index] {
            NorgASTFlat::Paragraph(segments) => {
                if let Some(html) = paragraph(segments, ids) {
                    body.push_str(&html);
                }
            }
            NorgASTFlat::RangedTag { name, .. } | NorgASTFlat::VerbatimRangedTag { name, .. }
                if is_comment_tag(name) => {}
            _ => crate::diagnostics::warn(
                "unsupported block inside a definition/footnote/table body — content skipped",
            ),
        }
        index += 1;
    }
    body
}

fn flat_comment_end(content: &[NorgASTFlat], index: usize) -> Option<usize> {
    let (kind, target) = flat_comment_target(&content[index])?;

    if let Some(level) = flat_heading_level(target) {
        let mut next = index + 1;
        let mut current_level = level as i16;
        while next < content.len() {
            match flat_heading_level(&content[next]) {
                Some(next_level) if next_level <= level || kind == CommentKind::Weak => break,
                Some(next_level) => current_level = next_level as i16,
                None if matches!(
                    &content[next],
                    NorgASTFlat::DelimitingModifier(delim)
                        if delimiter_exits_heading_scope(
                            delim,
                            level,
                            &mut current_level,
                        )
                ) =>
                {
                    break;
                }
                None => {}
            }
            next += 1;
        }
        return Some(next);
    }

    if kind == CommentKind::Strong
        && let NorgASTFlat::NestableDetachedModifier {
            modifier_type,
            level,
            ..
        } = target
    {
        let mut next = index + 1;
        while next < content.len() {
            let NorgASTFlat::NestableDetachedModifier {
                modifier_type: next_type,
                level: next_level,
                ..
            } = flat_carryover_target(&content[next])
            else {
                break;
            };
            if next_level < level || (next_level == level && next_type != modifier_type) {
                break;
            }
            next += 1;
        }
        return Some(next);
    }

    Some(index + 1)
}
