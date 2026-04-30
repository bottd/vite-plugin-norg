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
use serde_json::{Map, Value};

#[napi(object)]
pub struct NorgParseResult {
    pub metadata: Map<String, Value>,
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

    Ok(NorgParseResult {
        metadata: extract_metadata(&ast),
        html_parts,
        toc: extract_toc(&ast),
        embed_components,
        embed_css,
    })
}

fn format_embed_error(content: &str, err: &crate::ast_handlers::EmbedParseError) -> String {
    match find_embed_line(content, err.index()) {
        Some(line) => format!("{err}. Offending line: {line}"),
        None => err.to_string(),
    }
}

fn find_embed_line(content: &str, index: usize) -> Option<String> {
    content
        .lines()
        .filter(|line| {
            line.trim_start()
                .strip_prefix("@embed")
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
        })
        .nth(index)
        .map(str::to_string)
}

#[napi]
pub fn get_theme_css(theme: String) -> String {
    builtin::all()
        .into_iter()
        .find(|t| t.name == theme)
        .map(|t| t.to_css("pre.arborium"))
        .unwrap_or_default()
}
