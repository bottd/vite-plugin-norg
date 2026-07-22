use crate::ast_handlers::{document_ids, visit_visible_headings};
use crate::segments::heading_html_and_id;
use crate::types::TocEntry;
use rust_norg::NorgAST;

pub fn extract_toc(ast: &[NorgAST]) -> Vec<TocEntry> {
    let mut toc = Vec::new();
    let mut ids = document_ids(ast);
    visit_visible_headings(ast, &mut |level, title| {
        let (title, id, level) = heading_html_and_id(title, level, &mut ids);
        if !id.is_empty() {
            toc.push(TocEntry {
                level: level as u32,
                title,
                id,
            });
        }
    });
    toc
}
