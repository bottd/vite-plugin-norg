use crate::ast_handlers::*;
use crate::segments::convert_segments;
use crate::types::{EmbedComponent, OutputMode};
use crate::utils::into_slug;
use arborium::Highlighter;
use htmlescape::encode_minimal;
use rust_norg::{
    NestableDetachedModifier, NorgAST, NorgASTFlat, ParagraphSegment, RangeableDetachedModifier,
};

struct TransformState {
    parts: Vec<String>,
    current_html: String,
    embed_components: Vec<EmbedComponent>,
    css_blocks: Vec<String>,
    mode: Option<OutputMode>,
    highlighter: Highlighter,
    /// Ordinal of every `@embed` declaration (incl. CSS, `None`-mode, error),
    /// used in error messages to match `find_embed_line`. Unlike
    /// `embed_components.len()`, it counts embeds that emit no component.
    embed_decls: usize,
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
        }
    }

    fn push_html(&mut self, html: &str) {
        self.current_html.push_str(html);
        self.current_html.push('\n');
    }

    fn apply_verbatim(&mut self, result: VerbatimTagResult) {
        match result {
            VerbatimTagResult::Html(html) => self.push_html(&html),
            VerbatimTagResult::Css(css) => self.css_blocks.push(css),
            VerbatimTagResult::Embed { mode, code } => {
                let index = self.embed_components.len() as u32;
                self.parts.push(std::mem::take(&mut self.current_html));
                self.embed_components
                    .push(EmbedComponent { index, mode, code });
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
    for node in nodes {
        transform_node(node, state)?;
    }
    Ok(())
}

fn render_children(
    nodes: &[NorgAST],
    state: &mut TransformState,
) -> Result<String, EmbedParseError> {
    let saved = std::mem::take(&mut state.current_html);
    let parts_len = state.parts.len();
    let embeds_len = state.embed_components.len();
    let css_len = state.css_blocks.len();
    let outcome = transform_nodes(nodes, state);
    let mut captured = std::mem::take(&mut state.current_html);
    state.current_html = saved;
    outcome?;
    // rust-norg's stage_4 only allows List nodes (or CarryoverTag wrapping a
    // Nestable) inside a list item's `content`, so verbatim tags cannot
    // appear here. If that ever changes, `apply_verbatim` would push to
    // state.parts/embed_components/css_blocks against the swapped-empty
    // current_html and misalign the embed-component stream — assert it stays
    // inert.
    debug_assert_eq!(state.parts.len(), parts_len);
    debug_assert_eq!(state.embed_components.len(), embeds_len);
    debug_assert_eq!(state.css_blocks.len(), css_len);
    let new_len = captured.trim_end_matches('\n').len();
    captured.truncate(new_len);
    Ok(captured)
}

fn render_list(
    modifier_type: &NestableDetachedModifier,
    items: &[NorgAST],
    state: &mut TransformState,
) -> Result<(), EmbedParseError> {
    let mut rendered = String::new();
    for node in items {
        match node {
            NorgAST::NestableDetachedModifier {
                text,
                extensions,
                content,
                ..
            } => {
                let children_html = render_children(content, state)?;
                if let Some(item) = nestable_modifier(text, extensions, &children_html) {
                    rendered.push_str(&item);
                }
            }
            _ => eprintln!("Warning: non-Nestable item inside List"),
        }
    }

    if rendered.is_empty() {
        return Ok(());
    }

    let tag = match modifier_type {
        NestableDetachedModifier::UnorderedList => "ul",
        NestableDetachedModifier::OrderedList => "ol",
        NestableDetachedModifier::Quote => "blockquote",
    };
    state.push_html(&format!("<{tag}>{rendered}</{tag}>"));
    Ok(())
}

fn transform_node(node: &NorgAST, state: &mut TransformState) -> Result<(), EmbedParseError> {
    match node {
        NorgAST::List {
            modifier_type,
            items,
        } => render_list(modifier_type, items, state)?,
        NorgAST::NestableDetachedModifier { .. } => {
            eprintln!("Warning: bare NestableDetachedModifier outside List");
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
            let title_html = convert_segments(title);
            let id = into_slug(&title_html);
            state.push_html(&format!("<h{level} id=\"{id}\">{title_html}</h{level}>"));
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
        NorgAST::CarryoverTag { .. } | NorgAST::RangedTag { .. } | NorgAST::InfirmTag { .. } => {
            eprintln!("Warning: unimplemented tag");
        }
    }
    Ok(())
}

fn rangeable_modifier(
    modifier_type: &RangeableDetachedModifier,
    title: &[ParagraphSegment],
    content: &[NorgASTFlat],
) -> String {
    let title_html = convert_segments(title);
    let body: String = content
        .iter()
        .filter_map(|node| match node {
            NorgASTFlat::Paragraph(segments) => paragraph(segments),
            _ => None,
        })
        .collect();

    let title_escaped = encode_minimal(&title_html);
    match modifier_type {
        RangeableDetachedModifier::Definition => {
            format!("<dl><dt>{title_escaped}</dt><dd>{body}</dd></dl>")
        }
        RangeableDetachedModifier::Footnote => {
            let id = into_slug(&title_html);
            format!(
                "<aside id=\"footnote-{id}\" class=\"footnote\"><strong>{title_escaped}</strong><p>{body}</p></aside>"
            )
        }
        RangeableDetachedModifier::Table => {
            format!("<table><caption>{title_escaped}</caption><tbody>{body}</tbody></table>")
        }
    }
}
