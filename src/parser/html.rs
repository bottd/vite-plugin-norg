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
            Some(modifier_type) => render_list(modifier_type, group, state),
            None => {
                for node in group {
                    transform_node(node, state)?;
                }
            }
        }
    }
    Ok(())
}

fn render_list<'a>(
    modifier_type: NestableDetachedModifier,
    group: impl Iterator<Item = &'a NorgAST>,
    state: &mut TransformState,
) {
    let items: String = group
        .filter_map(|node| match node {
            NorgAST::NestableDetachedModifier {
                text, extensions, ..
            } => nestable_modifier(text, extensions),
            _ => None,
        })
        .collect();

    if items.is_empty() {
        return;
    }

    let tag = match modifier_type {
        NestableDetachedModifier::UnorderedList => "ul",
        NestableDetachedModifier::OrderedList => "ol",
        NestableDetachedModifier::Quote => "blockquote",
    };
    state.push_html(&format!("<{tag}>{items}</{tag}>"));
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
