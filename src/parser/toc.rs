use crate::segments::convert_segments;
use crate::types::TocEntry;
use crate::utils::into_slug;
use rust_norg::NorgAST;

pub fn extract_toc(ast: &[NorgAST]) -> Vec<TocEntry> {
    let mut toc = Vec::new();
    extract_toc_recursive(ast, &mut toc);
    toc
}

fn extract_toc_recursive(ast: &[NorgAST], toc: &mut Vec<TocEntry>) {
    for node in ast {
        if let NorgAST::Heading {
            level,
            title,
            content,
            ..
        } = node
        {
            let text = convert_segments(title);
            let id = into_slug(&text);

            toc.push(TocEntry {
                level: *level as usize,
                title: text,
                id,
            });

            extract_toc_recursive(content, toc);
        }
    }
}
