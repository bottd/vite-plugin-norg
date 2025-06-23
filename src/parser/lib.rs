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
    let ast = rust_norg::parse_tree(content).map_err(|e| format!("Parse error: {:?}", e))?;

    let (html, toc) = convert_nodes(&ast);
    let parsed = ParsedNorg {
        metadata: extract_metadata(&ast),
        html,
        toc,
    };

    js_sys::JSON::parse(&serde_json::to_string(&parsed).unwrap())
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {:?}", e)))
}
