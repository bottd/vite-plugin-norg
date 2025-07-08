use crate::ast_handlers::*;
use rust_norg::NorgAST;

pub fn transform(ast: &[NorgAST]) -> String {
    ast.iter()
        .filter_map(|node| match node {
            NorgAST::NestableDetachedModifier {
                modifier_type,
                text,
                extensions,
                ..
            } => handle_nestable_modifier(modifier_type, text, extensions),

            NorgAST::VerbatimRangedTag {
                name,
                parameters,
                content,
                ..
            } => handle_verbatim_tag(name, parameters, content),

            NorgAST::Heading {
                level,
                title,
                content,
                ..
            } => Some(handle_heading(*level, title, content)),

            NorgAST::Paragraph(segments) => handle_paragraph(segments),

            NorgAST::RangeableDetachedModifier {
                modifier_type,
                title,
                content,
                ..
            } => Some(handle_rangeable_modifier(modifier_type, title, content)),

            NorgAST::DelimitingModifier(delimiter) => Some(handle_delimiter(delimiter)),

            NorgAST::CarryoverTag { .. } => {
                eprintln!("Warning: CarryoverTag not implemented");
                None
            }
            NorgAST::RangedTag { .. } => {
                eprintln!("Warning: RangedTag not implemented");
                None
            }
            NorgAST::InfirmTag { .. } => {
                eprintln!("Warning: InfirmTag not implemented");
                None
            }
        })
        .collect::<Vec<String>>()
        .join("")
}
