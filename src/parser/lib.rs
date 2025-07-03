mod html;
mod metadata;
mod types;
mod utils;

pub use html::convert_nodes;
pub use metadata::extract_metadata;
pub use types::{ParsedNorg, TocEntry};
pub use utils::into_slug;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse_norg(content: &str) -> Result<JsValue, JsValue> {
    let ast = rust_norg::parse_tree(content)
        .map_err(|e| JsValue::from_str(&format!("Parse error: {:?}", e)))?;

    let (html, toc) = convert_nodes(&ast);
    let parsed = ParsedNorg {
        metadata: extract_metadata(&ast),
        html,
        toc,
    };

    serde_wasm_bindgen::to_value(&parsed)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {:?}", e)))
}
