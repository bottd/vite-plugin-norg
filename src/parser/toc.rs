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
        match node {
            NorgAST::Heading {
                level,
                title,
                content,
                ..
            } => {
                let text = conv_segs(title);
                let id = into_slug(&text);

                toc.push(TocEntry {
                    level: *level as usize,
                    title: text,
                    id,
                });

                extract_toc_recursive(content, toc);
            }
            _ => {}
        }
    }
}

use htmlescape::encode_minimal;
use rust_norg::{ParagraphSegment, ParagraphSegmentToken};

fn conv_segs(segments: &[ParagraphSegment]) -> String {
    segments
        .iter()
        .map(|segment| match segment {
            ParagraphSegment::Token(token) => match token {
                ParagraphSegmentToken::Whitespace => " ".to_string(),
                ParagraphSegmentToken::Text(text) => encode_minimal(text),
                ParagraphSegmentToken::Special(ch) => encode_minimal(&ch.to_string()),
                ParagraphSegmentToken::Escape(ch) => ch.to_string(),
            },
            ParagraphSegment::AttachedModifier { content, .. } => conv_segs(content),
            ParagraphSegment::Link { description, .. } => description
                .as_ref()
                .map(|d| conv_segs(d))
                .unwrap_or_default(),
            ParagraphSegment::Anchor { content, .. } => conv_segs(content),
            ParagraphSegment::InlineVerbatim(tokens) => {
                encode_minimal(&tokens.iter().map(ToString::to_string).collect::<String>())
            }
            _ => String::new(),
        })
        .collect()
}
