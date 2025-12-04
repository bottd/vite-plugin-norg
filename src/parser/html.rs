use crate::ast_handlers::*;
use itertools::Itertools;
use rust_norg::{NestableDetachedModifier, NorgAST};

pub fn transform(ast: &[NorgAST]) -> String {
    ast.iter()
        .chunk_by(|node| match node {
            NorgAST::NestableDetachedModifier { modifier_type, .. } => Some(modifier_type.clone()),
            _ => None,
        })
        .into_iter()
        .map(|(key, group)| match key {
            Some(modifier_type) => {
                let items: String = group
                    .filter_map(|node| match node {
                        NorgAST::NestableDetachedModifier {
                            text, extensions, ..
                        } => nestable_modifier(text, extensions),
                        _ => None,
                    })
                    .collect();

                if items.is_empty() {
                    String::new()
                } else {
                    let tag = match modifier_type {
                        NestableDetachedModifier::UnorderedList => "ul",
                        NestableDetachedModifier::OrderedList => "ol",
                        NestableDetachedModifier::Quote => "blockquote",
                    };
                    format!("<{tag}>{items}</{tag}>")
                }
            }
            None => group
                .filter_map(|node| match node {
                    NorgAST::NestableDetachedModifier { .. } => None,
                    NorgAST::VerbatimRangedTag {
                        name,
                        parameters,
                        content,
                        ..
                    } => verbatim_tag(name, parameters, content),
                    NorgAST::Heading {
                        level,
                        title,
                        content,
                        ..
                    } => Some(heading(*level, title, content)),
                    NorgAST::Paragraph(segments) => paragraph(segments),
                    NorgAST::RangeableDetachedModifier {
                        modifier_type,
                        title,
                        content,
                        ..
                    } => Some(rangeable_modifier(modifier_type, title, content)),
                    NorgAST::DelimitingModifier(delim) => Some(delimiter(delim)),
                    NorgAST::CarryoverTag { .. }
                    | NorgAST::RangedTag { .. }
                    | NorgAST::InfirmTag { .. } => {
                        eprintln!("Warning: unimplemented tag");
                        None
                    }
                })
                .collect(),
        })
        .collect()
}
