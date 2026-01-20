use crate::ast_handlers::*;
use crate::types::InlineComponent;
use itertools::Itertools;
use rust_norg::{NestableDetachedModifier, NorgAST};

/// Transform AST to HTML parts and inline components
///
/// # Arguments
/// * `ast` - The Norg AST to transform
/// * `target_framework` - The target framework from config (e.g., "svelte", "react", "vue")
///
/// # Returns
/// A tuple of (HTML parts, vector of inline components)
/// For N inline components, there will be N+1 HTML parts
/// When target_framework is None, inlines will be empty and parts has single element
pub fn transform(
    ast: &[NorgAST],
    target_framework: Option<&str>,
) -> (Vec<String>, Vec<InlineComponent>) {
    let mut parts: Vec<String> = Vec::new();
    let mut current_part = String::new();
    let mut inlines = Vec::new();

    for (key, group) in ast
        .iter()
        .chunk_by(|node| match node {
            NorgAST::NestableDetachedModifier { modifier_type, .. } => Some(modifier_type.clone()),
            _ => None,
        })
        .into_iter()
    {
        match key {
            Some(modifier_type) => {
                let items: String = group
                    .filter_map(|node| match node {
                        NorgAST::NestableDetachedModifier {
                            text, extensions, ..
                        } => nestable_modifier(text.as_ref(), extensions),
                        _ => None,
                    })
                    .collect();

                if !items.is_empty() {
                    let tag = match modifier_type {
                        NestableDetachedModifier::UnorderedList => "ul",
                        NestableDetachedModifier::OrderedList => "ol",
                        NestableDetachedModifier::Quote => "blockquote",
                    };
                    current_part.push_str(&format!("<{tag}>{items}</{tag}>"));
                }
            }
            None => {
                for node in group {
                    match node {
                        NorgAST::NestableDetachedModifier { .. } => {}
                        NorgAST::VerbatimRangedTag {
                            name,
                            parameters,
                            content,
                            ..
                        } => {
                            if let Some(result) = verbatim_tag_with_embeds(
                                name.as_slice(),
                                parameters.as_slice(),
                                content.as_str(),
                                target_framework,
                            ) {
                                if let Some(mut inline) = result.inline {
                                    // Set the index and end current part
                                    inline.index = inlines.len() as u32;
                                    parts.push(std::mem::take(&mut current_part));
                                    inlines.push(inline);
                                }
                                if let Some(html) = result.html {
                                    current_part.push_str(&html);
                                }
                            }
                        }
                        NorgAST::Heading {
                            level,
                            title,
                            content,
                            ..
                        } => {
                            transform_heading(
                                *level,
                                title.as_slice(),
                                content.as_slice(),
                                target_framework,
                                &mut parts,
                                &mut current_part,
                                &mut inlines,
                            );
                        }
                        NorgAST::Paragraph(segments) => {
                            if let Some(p) = paragraph(segments.as_slice()) {
                                current_part.push_str(&p);
                            }
                        }
                        NorgAST::RangeableDetachedModifier {
                            modifier_type,
                            title,
                            content,
                            ..
                        } => {
                            current_part.push_str(&transform_rangeable_modifier(
                                modifier_type,
                                title.as_slice(),
                                content.as_slice(),
                            ));
                        }
                        NorgAST::DelimitingModifier(delim) => {
                            current_part.push_str(&delimiter(delim));
                        }
                        NorgAST::CarryoverTag { .. }
                        | NorgAST::RangedTag { .. }
                        | NorgAST::InfirmTag { .. } => {
                            eprintln!("Warning: unimplemented tag");
                        }
                    }
                }
            }
        }
    }

    // Push final part
    parts.push(current_part);

    (parts, inlines)
}

/// Transform a heading, appending to parts and collecting inlines
fn transform_heading(
    level: u16,
    title: &[rust_norg::ParagraphSegment],
    content: &[NorgAST],
    target_framework: Option<&str>,
    parts: &mut Vec<String>,
    current_part: &mut String,
    inlines: &mut Vec<InlineComponent>,
) {
    let text = crate::segments::convert_segments(title);
    let id = crate::utils::into_slug(&text);
    current_part.push_str(&format!("<h{level} id=\"{id}\">{text}</h{level}>"));

    // Process heading content
    transform_inner(content, target_framework, parts, current_part, inlines);
}

/// Transform a rangeable modifier
fn transform_rangeable_modifier(
    modifier_type: &rust_norg::RangeableDetachedModifier,
    title: &[rust_norg::ParagraphSegment],
    content: &[rust_norg::NorgASTFlat],
) -> String {
    let title = crate::segments::convert_segments(title);
    let body: String = content
        .iter()
        .filter_map(|node| {
            if let rust_norg::NorgASTFlat::Paragraph(segments) = node {
                let html = crate::segments::convert_segments(segments);
                if html.trim().is_empty() {
                    None
                } else {
                    Some(format!("<p>{html}</p>"))
                }
            } else {
                None
            }
        })
        .collect();

    match modifier_type {
        rust_norg::RangeableDetachedModifier::Definition => format!(
            "<dl><dt>{}</dt><dd>{}</dd></dl>",
            htmlescape::encode_minimal(&title),
            body
        ),
        rust_norg::RangeableDetachedModifier::Footnote => {
            let id = crate::utils::into_slug(&title);
            format!(
                "<aside id=\"footnote-{}\" class=\"footnote\"><strong>{}</strong><p>{}</p></aside>",
                htmlescape::encode_minimal(&id),
                htmlescape::encode_minimal(&title),
                body
            )
        }
        rust_norg::RangeableDetachedModifier::Table => format!(
            "<table><caption>{}</caption><tbody>{}</tbody></table>",
            htmlescape::encode_minimal(&title),
            body
        ),
    }
}

/// Transform inner AST nodes (for nested content like heading bodies)
fn transform_inner(
    nodes: &[NorgAST],
    target_framework: Option<&str>,
    parts: &mut Vec<String>,
    current_part: &mut String,
    inlines: &mut Vec<InlineComponent>,
) {
    for node in nodes {
        match node {
            NorgAST::VerbatimRangedTag {
                name,
                parameters,
                content,
                ..
            } => {
                if let Some(result) = verbatim_tag_with_embeds(
                    name.as_slice(),
                    parameters.as_slice(),
                    content.as_str(),
                    target_framework,
                ) {
                    if let Some(mut inline) = result.inline {
                        inline.index = inlines.len() as u32;
                        parts.push(std::mem::take(current_part));
                        inlines.push(inline);
                    }
                    if let Some(html) = result.html {
                        current_part.push_str(&html);
                    }
                }
            }
            NorgAST::Heading {
                level,
                title,
                content,
                ..
            } => {
                transform_heading(
                    *level,
                    title.as_slice(),
                    content.as_slice(),
                    target_framework,
                    parts,
                    current_part,
                    inlines,
                );
            }
            NorgAST::Paragraph(segments) => {
                if let Some(p) = paragraph(segments.as_slice()) {
                    current_part.push_str(&p);
                }
            }
            NorgAST::RangeableDetachedModifier {
                modifier_type,
                title,
                content,
                ..
            } => {
                current_part.push_str(&transform_rangeable_modifier(
                    modifier_type,
                    title.as_slice(),
                    content.as_slice(),
                ));
            }
            NorgAST::DelimitingModifier(delim) => {
                current_part.push_str(&delimiter(delim));
            }
            _ => {}
        }
    }
}
