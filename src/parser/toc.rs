use crate::segments::heading_html_and_id;
use crate::types::TocEntry;
use rust_norg::NorgAST;
use std::collections::HashMap;

pub fn extract_toc(ast: &[NorgAST]) -> Vec<TocEntry> {
    let mut toc = Vec::new();
    // Same document-order walk as the renderer, so the id de-duplication in
    // `heading_html_and_id` produces ids that match the emitted heading tags.
    let mut seen = HashMap::new();
    collect_headings(ast, &mut toc, &mut seen);
    toc
}

fn collect_headings(ast: &[NorgAST], toc: &mut Vec<TocEntry>, seen: &mut HashMap<String, u32>) {
    for node in ast {
        match node {
            NorgAST::Heading {
                level,
                title,
                content,
                ..
            } => {
                let (text, id, level) = heading_html_and_id(title, *level, seen);

                toc.push(TocEntry {
                    level: level as u32,
                    title: text,
                    id,
                });

                collect_headings(content, toc, seen);
            }
            // The renderer unwraps carryover tags and renders the annotated
            // object, so a tagged heading must appear in the TOC too.
            NorgAST::CarryoverTag { next_object, .. } => {
                collect_headings(std::slice::from_ref(next_object), toc, seen);
            }
            _ => {}
        }
    }
}
