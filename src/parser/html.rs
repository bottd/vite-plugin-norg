use crate::ast_handlers::*;
use crate::segments::convert_segments;
use crate::types::{EmbedComponent, OutputMode};
use crate::utils::into_slug;
use arborium::Highlighter;
use htmlescape::encode_minimal;
use itertools::Itertools;
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
    let groups = nodes.iter().chunk_by(|node| match node {
        NorgAST::NestableDetachedModifier { modifier_type, .. } => Some(modifier_type.clone()),
        _ => None,
    });

    for (list_type, group) in &groups {
        match list_type {
            Some(modifier_type) => render_list(modifier_type, group, state)?,
            None => {
                for node in group {
                    transform_node(node, state)?;
                }
            }
        }
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
    let outcome = transform_nodes(nodes, state);
    let captured = std::mem::take(&mut state.current_html);
    state.current_html = saved;
    // rust-norg's stage_4 only allows NestableDetachedModifier nodes (or
    // CarryoverTag wrapping one) inside a list item's `content`, so verbatim
    // tags cannot appear here. If that ever changes, `apply_verbatim` would
    // push to `state.parts` against the swapped-empty current_html and
    // misalign the parts/embed_components stream — assert it stays inert.
    debug_assert_eq!(state.parts.len(), parts_len);
    debug_assert_eq!(state.embed_components.len(), embeds_len);
    outcome?;
    let trimmed = captured.trim_end_matches('\n');
    Ok(if trimmed.len() == captured.len() {
        captured
    } else {
        trimmed.to_owned()
    })
}

fn render_list<'a>(
    modifier_type: NestableDetachedModifier,
    group: impl Iterator<Item = &'a NorgAST>,
    state: &mut TransformState,
) -> Result<(), EmbedParseError> {
    let mut items = String::new();
    for node in group {
        match node {
            NorgAST::NestableDetachedModifier {
                text,
                extensions,
                content,
                ..
            } => {
                let children_html = render_children(content, state)?;
                if let Some(item) = nestable_modifier(text, extensions, &children_html) {
                    items.push_str(&item);
                }
            }
            _ => unreachable!("chunk_by groups only NestableDetachedModifier under Some(_) key"),
        }
    }

    if items.is_empty() {
        return Ok(());
    }

    let tag = match modifier_type {
        NestableDetachedModifier::UnorderedList => "ul",
        NestableDetachedModifier::OrderedList => "ol",
        NestableDetachedModifier::Quote => "blockquote",
    };
    state.push_html(&format!("<{tag}>{items}</{tag}>"));
    Ok(())
}

fn transform_node(node: &NorgAST, state: &mut TransformState) -> Result<(), EmbedParseError> {
    match node {
        NorgAST::NestableDetachedModifier { .. } => {}
        NorgAST::VerbatimRangedTag {
            name,
            parameters,
            content,
            ..
        } => {
            if let Some(result) = VerbatimTag::from(name.as_slice()).render(
                parameters,
                content,
                state.mode,
                &mut state.highlighter,
                state.embed_components.len(),
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
            NorgASTFlat::Paragraph(segments) => {
                let html = convert_segments(segments);
                (!html.trim().is_empty()).then(|| format!("<p>{html}</p>"))
            }
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
