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
pub use types::{InlineComponent, ParsedNorg, TocEntry};
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
    pub inlines: Vec<InlineComponent>,
}

#[napi]
pub fn parse_norg(content: String) -> Result<NorgParseResult> {
    parse_norg_with_framework(content, Option::<String>::None)
}

#[napi]
pub fn parse_norg_with_framework(
    content: String,
    framework: Option<String>,
) -> Result<NorgParseResult> {
    let ast = rust_norg::parse_tree(&content)
        .map_err(|e| Error::from_reason(format!("Parse error: {e:?}")))?;

    let target_framework = framework.as_deref();
    let (html_parts, inlines) = transform(&ast, target_framework)
        .map_err(|err| Error::from_reason(format_inline_error(&content, &err)))?;
    let toc = extract_toc(&ast);
    let metadata = extract_metadata(&ast);

    Ok(NorgParseResult {
        metadata,
        html_parts,
        toc,
        inlines,
    })
}

fn format_inline_error(content: &str, err: &crate::ast_handlers::InlineParseError) -> String {
    let base = err.to_string();
    if let Some(line) = find_inline_line(content, err.index) {
        format!("{base}. Offending line: {line}")
    } else {
        base
    }
}

fn find_inline_line(content: &str, index: usize) -> Option<String> {
    let mut count = 0;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("@inline") {
            if rest.is_empty() || rest.chars().next().map_or(true, |c| c.is_whitespace()) {
                if count == index {
                    return Some(line.to_string());
                }
                count += 1;
            }
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
