mod ast_handlers;
mod html;
mod metadata;
mod segments;
mod toc;
mod types;
mod utils;

pub use html::transform;
pub use metadata::extract_metadata;
pub use toc::extract_toc;
pub use types::{EmbedComponent, OutputMode, TocEntry};
pub use utils::into_slug;

use arborium::theme::builtin;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Map;

#[napi(object)]
pub struct NorgParseResult {
    pub metadata: Map<String, serde_json::Value>,
    pub html_parts: Vec<String>,
    pub toc: Vec<TocEntry>,
    pub embed_components: Vec<EmbedComponent>,
    pub embed_css: String,
}

#[napi]
pub fn parse_norg(content: String, mode: Option<String>) -> Result<NorgParseResult> {
    let ast = rust_norg::parse_tree(&content)
        .map_err(|e| Error::from_reason(format!("Parse error: {e:?}")))?;

    let output_mode = mode.as_deref().and_then(|s| s.parse().ok());
    let (html_parts, embed_components, embed_css) = transform(&ast, output_mode)
        .map_err(|err| Error::from_reason(format_embed_error(&content, &err)))?;
    let toc = extract_toc(&ast);
    let metadata = extract_metadata(&ast);

    Ok(NorgParseResult {
        metadata,
        html_parts,
        toc,
        embed_components,
        embed_css,
    })
}

fn format_embed_error(content: &str, err: &crate::ast_handlers::EmbedParseError) -> String {
    let base = err.to_string();
    if let Some(line) = find_embed_line(content, err.index()) {
        format!("{base}. Offending line: {line}")
    } else {
        base
    }
}

fn find_embed_line(content: &str, index: usize) -> Option<String> {
    let mut count = 0;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("@embed")
            && (rest.is_empty() || rest.chars().next().is_none_or(|c| c.is_whitespace()))
        {
            if count == index {
                return Some(line.to_string());
            }
            count += 1;
        }
    }
    None
}

#[napi]
pub fn get_theme_css(theme: String) -> String {
    builtin::all()
        .into_iter()
        .find(|t| t.name == theme)
        .map(|t| t.to_css("pre.arborium"))
        .unwrap_or_default()
}
