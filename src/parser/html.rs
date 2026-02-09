use crate::ast_handlers::*;
use crate::types::InlineComponent;
use itertools::Itertools;
use rust_norg::{NestableDetachedModifier, NorgAST};

#[derive(Default)]
struct TransformState<'a> {
    parts: Vec<String>,
    current_html: String,
    inline_components: Vec<InlineComponent>,
    css_blocks: Vec<String>,
    target_framework: Option<&'a str>,
}

impl TransformState<'_> {
    fn push_inline(&mut self, mut inline: InlineComponent) {
        inline.index = self.inline_components.len() as u32;
        self.parts.push(std::mem::take(&mut self.current_html));
        self.inline_components.push(inline);
    }

    fn push_html(&mut self, html: &str) {
        self.current_html.push_str(html);
    }

    fn apply_verbatim(&mut self, result: VerbatimTagResult) {
        if let Some(inline) = result.inline {
            self.push_inline(inline);
        }
        if let Some(html) = result.html {
            self.push_html(&html);
        }
        if let Some(css) = result.css {
            self.css_blocks.push(css);
        }
    }

    fn finalize(mut self) -> (Vec<String>, Vec<InlineComponent>, String) {
        self.parts.push(self.current_html);
        let inline_css = self.css_blocks.join("\n");
        (self.parts, self.inline_components, inline_css)
    }
}

pub fn transform(
    ast: &[NorgAST],
    target_framework: Option<&str>,
) -> Result<(Vec<String>, Vec<InlineComponent>, String), InlineParseError> {
    let mut state = TransformState {
        target_framework,
        ..Default::default()
    };
    transform_nodes(ast, &mut state)?;
    Ok(state.finalize())
}

fn transform_nodes(nodes: &[NorgAST], state: &mut TransformState) -> Result<(), InlineParseError> {
    for (list_type, group) in nodes
        .iter()
        .chunk_by(|node| match node {
            NorgAST::NestableDetachedModifier { modifier_type, .. } => Some(modifier_type.clone()),
            _ => None,
        })
        .into_iter()
    {
        match list_type {
            Some(modifier_type) => {
                let list_items: String = group
                    .filter_map(|node| match node {
                        NorgAST::NestableDetachedModifier {
                            text, extensions, ..
                        } => nestable_modifier(text.as_ref(), extensions),
                        _ => None,
                    })
                    .collect();

                if !list_items.is_empty() {
                    let tag = match modifier_type {
                        NestableDetachedModifier::UnorderedList => "ul",
                        NestableDetachedModifier::OrderedList => "ol",
                        NestableDetachedModifier::Quote => "blockquote",
                    };
                    state.push_html(&format!("<{tag}>{list_items}</{tag}>"));
                }
            }
            None => {
                for node in group {
                    transform_node(node, state)?;
                }
            }
        }
    }
    Ok(())
}

fn transform_node(node: &NorgAST, state: &mut TransformState) -> Result<(), InlineParseError> {
    match node {
        NorgAST::NestableDetachedModifier { .. } => {}
        NorgAST::VerbatimRangedTag {
            name,
            parameters,
            content,
            ..
        } => {
            if let Some(result) =
                verbatim_tag_with_embeds(name, parameters, content, state.target_framework)
                    .map_err(|kind| InlineParseError {
                        index: state.inline_components.len(),
                        kind,
                    })?
            {
                state.apply_verbatim(result);
            }
        }
        NorgAST::Heading {
            level,
            title,
            content,
            ..
        } => {
            let title_html = crate::segments::convert_segments(title);
            let id = crate::utils::into_slug(&title_html);
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
        } => {
            state.push_html(&rangeable_modifier(modifier_type, title, content));
        }
        NorgAST::DelimitingModifier(delim) => {
            state.push_html(delimiter(delim));
        }
        NorgAST::CarryoverTag { .. } | NorgAST::RangedTag { .. } | NorgAST::InfirmTag { .. } => {
            eprintln!("Warning: unimplemented tag");
        }
    }
    Ok(())
}

fn rangeable_modifier(
    modifier_type: &rust_norg::RangeableDetachedModifier,
    title: &[rust_norg::ParagraphSegment],
    content: &[rust_norg::NorgASTFlat],
) -> String {
    let title_html = crate::segments::convert_segments(title);
    let body: String = content
        .iter()
        .filter_map(|node| match node {
            rust_norg::NorgASTFlat::Paragraph(segments) => {
                let html = crate::segments::convert_segments(segments);
                (!html.trim().is_empty()).then(|| format!("<p>{html}</p>"))
            }
            _ => None,
        })
        .collect();

    match modifier_type {
        rust_norg::RangeableDetachedModifier::Definition => format!(
            "<dl><dt>{}</dt><dd>{}</dd></dl>",
            htmlescape::encode_minimal(&title_html),
            body
        ),
        rust_norg::RangeableDetachedModifier::Footnote => {
            let id = crate::utils::into_slug(&title_html);
            format!(
                "<aside id=\"footnote-{}\" class=\"footnote\"><strong>{}</strong><p>{}</p></aside>",
                htmlescape::encode_minimal(&id),
                htmlescape::encode_minimal(&title_html),
                body
            )
        }
        rust_norg::RangeableDetachedModifier::Table => format!(
            "<table><caption>{}</caption><tbody>{}</tbody></table>",
            htmlescape::encode_minimal(&title_html),
            body
        ),
    }
}
