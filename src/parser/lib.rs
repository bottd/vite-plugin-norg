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
pub use types::{ParsedNorg, TocEntry};
pub use utils::into_slug;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Map;

#[napi(object)]
pub struct NorgParseResult {
    pub metadata: Map<String, serde_json::Value>,
    pub html: String,
    pub toc: Vec<TocEntry>,
}

#[napi]
pub fn parse_norg(content: String) -> Result<NorgParseResult> {
    let ast = rust_norg::parse_tree(&content)
        .map_err(|e| Error::from_reason(format!("Parse error: {e:?}")))?;

    let html = transform(&ast);
    let toc = extract_toc(&ast);
    let metadata = extract_metadata(&ast);

    Ok(NorgParseResult {
        metadata,
        html,
        toc,
    })
}
